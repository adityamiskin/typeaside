use crate::{cleanup, config::AppConfig, engine::Mode, snippets, transcribe};
use anyhow::Result;
use std::time::{Duration, Instant};

pub struct Output {
    pub raw: String,
    pub text: String,
    pub stt: Duration,
    pub llm: Duration,
}

pub fn run(
    config: &AppConfig,
    audio: &[f32],
    app_id: Option<&str>,
    mode: Mode,
    selection: &str,
    cleaning: impl FnOnce(),
) -> Result<Output> {
    let effective = config.effective();
    let config = &effective;
    let started = Instant::now();
    let raw = transcribe::transcribe(config, audio)?;
    let stt = started.elapsed();
    let started = Instant::now();
    let text = if raw.is_empty() {
        String::new()
    } else if mode == Mode::Command {
        cleaning();
        cleanup::command(config, &raw, selection)?
    } else {
        if config.cleanup.enabled || config.translate_to.is_some() {
            cleaning();
        }
        snippets::expand(&cleanup::cleanup(config, &raw, app_id), &config.snippets)
    };
    anyhow::ensure!(!text.is_empty(), "No transcription returned");
    let llm = started.elapsed();
    if std::env::var_os("HERMES_TRACE").is_some_and(|value| value == "1") {
        eprintln!(
            "hermes: stt={}ms llm={}ms",
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
