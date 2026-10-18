use crate::{
    cleanup::{self, Answer},
    config::{AppConfig, Source},
    engine::Mode,
    snippets,
    text_injector::FocusedApp,
    transcribe,
};
use anyhow::Result;
use std::time::{Duration, Instant};

pub const LOCAL_COMMAND: &str = "Commands need a TypeAside account or your own key";
pub const LOCAL_TRANSLATE: &str = "Translation needs a TypeAside account or your own key";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
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
}

fn offline(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|e| e.status().is_none() && (e.is_connect() || e.is_timeout()))
    })
}

pub fn run(
    config: &AppConfig,
    audio: &[f32],
    app: Option<&FocusedApp>,
    mode: Mode,
    selection: &str,
    names: &[String],
    cleaning: impl FnOnce(),
) -> Result<Output> {
    let effective = config.effective();
    let config = &effective;
    let started = Instant::now();
    if mode == Mode::Translate && audio.is_empty() {
        cleaning();
        return Ok(Output {
            raw: selection.into(),
            text: cleanup::translate(config, selection)?,
            delivery: Delivery::Insert,
            stt: Duration::ZERO,
            llm: started.elapsed(),
        });
    }
    let (raw, local) = match config.transcription {
        Source::Local if mode == Mode::Command => anyhow::bail!(LOCAL_COMMAND),
        Source::Local if mode == Mode::Translate => anyhow::bail!(LOCAL_TRANSLATE),
        Source::Local => (crate::local::transcribe(audio)?, true),
        Source::Hosted | Source::OwnKey => match transcribe::transcribe(config, audio, names) {
            Ok(raw) => (raw, false),
            Err(error) if mode == Mode::Dictate && offline(&error) && crate::local::installed() => {
                (crate::local::transcribe(audio)?, true)
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
            text: snippets::expand(&cleanup::remove_fillers(&raw), &config.snippets),
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
    })
}
