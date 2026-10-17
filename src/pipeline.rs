use crate::{
    cleanup,
    config::{AppConfig, Source},
    engine::Mode,
    snippets,
    text_injector::FocusedApp,
    transcribe,
};
use anyhow::Result;
use std::time::{Duration, Instant};

pub struct Output {
    pub raw: String,
    pub text: String,
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
    cleaning: impl FnOnce(),
) -> Result<Output> {
    let effective = config.effective();
    let config = &effective;
    let started = Instant::now();
    let (raw, local) = match config.transcription {
        Source::Local => (crate::local::transcribe(audio)?, true),
        Source::Hosted | Source::OwnKey => match transcribe::transcribe(config, audio) {
            Ok(raw) => (raw, false),
            Err(error) if mode != Mode::Command && offline(&error) && crate::local::installed() => {
                (crate::local::transcribe(audio)?, true)
            }
            Err(error) => return Err(error),
        },
    };
    let stt = started.elapsed();
    let started = Instant::now();
    let text = if raw.is_empty() {
        String::new()
    } else if local {
        snippets::expand(&cleanup::remove_fillers(&raw), &config.snippets)
    } else if mode == Mode::Command {
        cleaning();
        cleanup::command(config, &raw, selection)?
    } else {
        if config.editing() || config.translate_to.is_some() {
            cleaning();
        }
        snippets::expand(&cleanup::cleanup(config, &raw, app), &config.snippets)
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
        text,
        stt,
        llm,
    })
}
