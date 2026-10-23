use crate::{
    cleanup::{self, Answer},
    config::{AppConfig, Source},
    engine::Mode,
    snippets,
    text_injector::FocusedApp,
    transcribe,
};
use anyhow::Result;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const LOCAL_COMMAND: &str = "Commands need your own API key";
pub const LOCAL_TRANSLATE: &str = "Translation needs your own API key";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    Insert,
    Submit,
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

/// A lone "send" submits without pasting text.
fn trailing_send(raw: &str) -> Option<&str> {
    static SEND: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let command = SEND
        .get_or_init(|| regex::Regex::new(r"(?i)(?:^|[^\w])(?P<send>send)\p{P}*\s*$").unwrap())
        .captures(raw)?
        .name("send")?;
    Some(raw[..command.start()].trim_end())
}

fn dictation_text<'a>(config: &AppConfig, mode: Mode, raw: &'a str) -> (&'a str, bool) {
    if config.send_command
        && mode == Mode::Dictate
        && let Some(text) = trailing_send(raw)
    {
        (text, true)
    } else {
        (raw, false)
    }
}

fn without_send(text: String, submit: bool) -> String {
    if submit {
        trailing_send(&text).map(str::to_owned).unwrap_or(text)
    } else {
        text
    }
}

/// Unreachable providers can still dictate on-device.
fn local_fallback(error: &anyhow::Error) -> bool {
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
    stream: Option<crate::local::Stream>,
    cleaning: impl FnOnce(),
) -> Result<Output> {
    run_cancellable(
        config,
        audio,
        app,
        mode,
        selection,
        names,
        stream,
        cleaning,
        Arc::new(AtomicBool::new(false)),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_cancellable(
    config: &AppConfig,
    audio: &[f32],
    app: Option<&FocusedApp>,
    mode: Mode,
    selection: &str,
    names: &[String],
    stream: Option<crate::local::Stream>,
    cleaning: impl FnOnce(),
    cancel: Arc<AtomicBool>,
) -> Result<Output> {
    let started = Instant::now();
    if config.transcription == Source::Local {
        match mode {
            Mode::Command => anyhow::bail!(LOCAL_COMMAND),
            Mode::Translate => anyhow::bail!(LOCAL_TRANSLATE),
            _ => {}
        }
    }
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
        Source::Local => (crate::local::finish(config, audio, stream)?.0, true),
        Source::OwnKey => match transcribe::transcribe(config, audio, names) {
            Ok(raw) => (raw, false),
            Err(error)
                if mode == Mode::Dictate && local_fallback(&error) && crate::local::installed() =>
            {
                (crate::local::transcribe(config, audio)?, true)
            }
            Err(error) => return Err(error),
        },
    };
    let stt = started.elapsed();
    let started = Instant::now();
    let corrected = crate::replacements::apply(&raw, &config.replacements);
    let (text, submit) = dictation_text(config, mode, &corrected);
    let answer = if text.is_empty() {
        Answer {
            delivery: Delivery::Insert,
            text: String::new(),
        }
    } else if local {
        anyhow::ensure!(!cancel.load(Ordering::Relaxed), "Dictation cancelled");
        let text = if config.transcription == Source::Local {
            if crate::local::CleanupSettings::for_app(config, app.and_then(|a| a.id.as_deref()))
                .is_some()
            {
                cleaning();
            }
            crate::local::cleanup_text(
                config,
                text,
                app.and_then(|a| a.id.as_deref()),
                cancel.clone(),
            )
        } else {
            cleanup::remove_fillers(text, config.language.as_deref())
        };
        anyhow::ensure!(!cancel.load(Ordering::Relaxed), "Dictation cancelled");
        Answer {
            delivery: Delivery::Insert,
            text: snippets::expand(&text, &config.snippets),
        }
    } else if mode == Mode::Command {
        cleaning();
        cleanup::command(config, &corrected, selection)?
    } else {
        if config.editing() || mode == Mode::Translate {
            cleaning();
        }
        Answer {
            delivery: Delivery::Insert,
            text: snippets::expand(
                &cleanup::cleanup(config, text, app, names, mode == Mode::Translate),
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
        text: without_send(answer.text, submit),
        delivery: if submit {
            Delivery::Submit
        } else {
            answer.delivery
        },
        stt,
        llm,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn send_setting_and_mode_control_detection() {
        let mut config = crate::config::AppConfig::default();
        assert_eq!(
            super::dictation_text(&config, super::Mode::Dictate, "hello Send."),
            ("hello", true)
        );
        for mode in [
            super::Mode::Translate,
            super::Mode::Command,
            super::Mode::File,
        ] {
            assert_eq!(
                super::dictation_text(&config, mode, "hello Send."),
                ("hello Send.", false)
            );
        }
        config.send_command = false;
        assert_eq!(
            super::dictation_text(&config, super::Mode::Dictate, "hello Send."),
            ("hello Send.", false)
        );
        assert_eq!(super::without_send("hello Send.".into(), true), "hello");
        assert_eq!(
            super::without_send("hello Send.".into(), false),
            "hello Send."
        );
    }

    #[test]
    fn trailing_send_is_a_final_whole_word() {
        for (raw, expected) in [
            ("See you soon. Send.", Some("See you soon.")),
            ("hello SEND!?", Some("hello")),
            ("hello send…  ", Some("hello")),
            ("hello.send.", Some("hello.")),
            ("send.", Some("")),
            ("send", Some("")),
            (" Send! ", Some("")),
            ("resend", None),
            ("please resend.", None),
            ("send it", None),
            ("send this tomorrow", None),
            ("hello send friend", None),
            ("", None),
            ("发送send", None),
        ] {
            assert_eq!(super::trailing_send(raw), expected, "{raw:?}");
        }
    }
}
