use crate::{
    cleanup::{self, Answer},
    config::{AppCategory, AppConfig, Source, Tone},
    engine::Mode,
    snippets,
    text_injector::FocusedApp,
    transcribe,
};
use anyhow::Result;
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::{Duration, Instant};

pub const LOCAL_COMMAND: &str = "Commands need a TypeAside account or your own key";
pub const LOCAL_TRANSLATE: &str = "Translation needs a TypeAside account or your own key";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    Insert,
    Reply,
    Open,
}

pub struct Output {
    pub raw: String,
    pub text: String,
    pub delivery: Delivery,
    pub stt: Duration,
    pub llm: Duration,
    pub usage: Option<crate::account::Usage>,
}

fn offline(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|e| e.status().is_none() && (e.is_connect() || e.is_timeout()))
    })
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    config: &AppConfig,
    audio: &[f32],
    app: Option<&FocusedApp>,
    mode: Mode,
    selection: &str,
    names: &[String],
    caret: Option<&(String, String)>,
    stream: Option<crate::stream::Session>,
    cleaning: impl FnOnce(),
) -> Result<Output> {
    let started = Instant::now();
    if config.transcription == Source::Hosted {
        cleaning();
        let streaming = stream.is_some();
        let streamed = if !audio.is_empty() {
            stream.and_then(|stream| {
                let context =
                    match hosted_context(config, app, mode, selection, names, caret, false) {
                        Ok(context) => context,
                        Err(error) => return Some(Err(error)),
                    };
                stream
                    .finish(context)
                    .map(|result| result.map(|response| response.output(started.elapsed())))
            })
        } else {
            None
        };
        if streaming && std::env::var_os("TYPEASIDE_TRACE").is_some_and(|value| value == "1") {
            eprintln!(
                "typeaside: stream={}",
                if streamed.is_some() {
                    "response"
                } else {
                    "POST fallback"
                }
            );
        }
        let result = match streamed {
            Some(result) => result,
            None => {
                let file = if mode == Mode::Translate && audio.is_empty() {
                    None
                } else {
                    Some(
                        multipart::Part::bytes(transcribe::encode_flac(audio)?)
                            .file_name("dictation.flac")
                            .mime_str("audio/flac")?,
                    )
                };
                hosted(config, file, app, mode, selection, names, caret)
            }
        };
        return match result {
            Ok(mut output) => {
                if output.delivery == Delivery::Insert {
                    output.text = snippets::expand(&output.text, &config.snippets);
                }
                Ok(output)
            }
            Err(error) if mode == Mode::Dictate && offline(&error) && crate::local::installed() => {
                let raw = crate::local::transcribe(config, audio)?;
                Ok(Output {
                    text: snippets::expand(
                        &cleanup::remove_fillers(&raw, config.language.as_deref()),
                        &config.snippets,
                    ),
                    raw,
                    delivery: Delivery::Insert,
                    stt: started.elapsed(),
                    llm: Duration::ZERO,
                    usage: None,
                })
            }
            result => result,
        };
    }
    if mode == Mode::Translate && audio.is_empty() {
        cleaning();
        return Ok(Output {
            raw: selection.into(),
            text: cleanup::translate(config, selection)?,
            delivery: Delivery::Insert,
            stt: Duration::ZERO,
            llm: started.elapsed(),
            usage: None,
        });
    }
    let (raw, local) = match config.transcription {
        Source::Local if mode == Mode::Command => anyhow::bail!(LOCAL_COMMAND),
        Source::Local if mode == Mode::Translate => anyhow::bail!(LOCAL_TRANSLATE),
        Source::Local => (crate::local::transcribe(config, audio)?, true),
        Source::Hosted => unreachable!("hosted requests return above"),
        Source::OwnKey => match transcribe::transcribe(config, audio, names) {
            Ok(raw) => (raw, false),
            Err(error) if mode == Mode::Dictate && offline(&error) && crate::local::installed() => {
                (crate::local::transcribe(config, audio)?, true)
            }
            Err(error) => return Err(error),
        },
    };
    let stt = started.elapsed();
    let started = Instant::now();
    let answer = if raw.is_empty() {
        Answer {
            delivery: Delivery::Insert,
            text: String::new(),
        }
    } else if local {
        Answer {
            delivery: Delivery::Insert,
            text: snippets::expand(
                &cleanup::remove_fillers(&raw, config.language.as_deref()),
                &config.snippets,
            ),
        }
    } else if mode == Mode::Command {
        cleaning();
        cleanup::command(config, &raw, selection)?
    } else {
        if config.editing() || mode == Mode::Translate {
            cleaning();
        }
        Answer {
            delivery: Delivery::Insert,
            text: snippets::expand(
                &cleanup::cleanup(config, &raw, app, names, mode == Mode::Translate),
                &config.snippets,
            ),
        }
    };
    let llm = started.elapsed();
    if std::env::var_os("TYPEASIDE_TRACE").is_some_and(|value| value == "1") {
        eprintln!(
            "typeaside: stt={}ms llm={}ms",
            stt.as_millis(),
            llm.as_millis()
        );
    }
    Ok(Output {
        raw,
        text: answer.text,
        delivery: answer.delivery,
        stt,
        llm,
        usage: None,
    })
}

pub(crate) fn hosted(
    config: &AppConfig,
    audio: Option<multipart::Part>,
    app: Option<&FocusedApp>,
    mode: Mode,
    selection: &str,
    names: &[String],
    caret: Option<&(String, String)>,
) -> Result<Output> {
    let started = Instant::now();
    let context = hosted_context(
        config,
        app,
        mode,
        selection,
        names,
        caret,
        mode == Mode::Translate && audio.is_none(),
    )?;
    let mut form = multipart::Form::new().text("context", context);
    if let Some(audio) = audio {
        form = form.part("audio", audio);
    }
    // Buffer uploads so connection failures remain detectable for offline fallback.
    let content_type = format!("multipart/form-data; boundary={}", form.boundary());
    let mut body = Vec::new();
    form.into_reader().read_to_end(&mut body)?;
    let url = crate::account::hosted_url();
    let response: HostedResponse = crate::http::checked_hosted(
        crate::http::client()
            .post(format!("{url}/v1/dictate"))
            .bearer_auth(crate::credentials::endpoint_key(&url)?)
            .timeout(Duration::from_secs(if mode == Mode::File {
                120
            } else {
                30
            }))
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body)
            .send()?,
    )?
    .json()?;
    Ok(response.output(started.elapsed()))
}

#[derive(Deserialize)]
pub(crate) struct HostedResponse {
    usage: crate::account::Usage,
    raw: String,
    text: String,
    delivery: Delivery,
}

impl HostedResponse {
    fn output(self, elapsed: Duration) -> Output {
        Output {
            raw: self.raw,
            text: self.text,
            delivery: self.delivery,
            stt: Duration::ZERO,
            llm: elapsed,
            usage: Some(self.usage),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn hosted_context(
    config: &AppConfig,
    app: Option<&FocusedApp>,
    mode: Mode,
    selection: &str,
    names: &[String],
    caret: Option<&(String, String)>,
    text_only: bool,
) -> Result<String> {
    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum HostedMode {
        Dictate,
        Translate,
        Command,
        Transcribe,
    }
    #[derive(Serialize)]
    struct Context<'a> {
        mode: HostedMode,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        app_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        selection: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        before_cursor: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        after_cursor: Option<&'a str>,
        category: AppCategory,
        dictionary: &'a [String],
        names: &'a [String],
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<&'a str>,
        tone: Tone,
        #[serde(skip_serializing_if = "Option::is_none")]
        instructions: Option<&'a str>,
        translate_to: &'a str,
    }
    let app_id = app.and_then(|app| app.id.as_deref());
    let category = AppCategory::of(app_id);
    let context = Context {
        mode: match mode {
            Mode::Dictate => HostedMode::Dictate,
            Mode::Translate => HostedMode::Translate,
            Mode::Command => HostedMode::Command,
            Mode::File => HostedMode::Transcribe,
        },
        text: text_only.then_some(selection),
        app_id,
        title: app.and_then(|app| app.title.as_deref()),
        selection: (mode == Mode::Command).then_some(selection),
        before_cursor: caret.map(|(before, _)| {
            before
                .char_indices()
                .rev()
                .nth(999)
                .map_or(before.as_str(), |(at, _)| &before[at..])
        }),
        after_cursor: caret.map(|(_, after)| {
            after
                .char_indices()
                .nth(1000)
                .map_or(after.as_str(), |(at, _)| &after[..at])
        }),
        category,
        dictionary: &config.dictionary,
        names,
        language: config.language.as_deref(),
        tone: config.styles.get(category),
        instructions: matches!(mode, Mode::Dictate | Mode::Translate)
            .then(|| config.instructions_for(app_id))
            .flatten(),
        translate_to: &config.translate_to,
    };
    Ok(serde_json::to_string(&context)?)
}
