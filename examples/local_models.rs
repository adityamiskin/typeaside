//! TYPEASIDE_HOME=/absolute/scratch cargo run --example local_models -- english.wav french.wav
//! Use --download MODEL to install into that isolated home, or --list to inspect it.
#[path = "support/wav.rs"]
mod wav;
use anyhow::{Context, Result, ensure};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use typeaside::local::{self, ModelId};

fn main() -> Result<()> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--local-worker") {
        return local::worker_main();
    }
    ensure!(
        std::env::var_os("TYPEASIDE_HOME").is_some(),
        "Set TYPEASIDE_HOME to an isolated absolute directory"
    );
    let legacy: typeaside::config::AppConfig =
        serde_json::from_str(r#"{"transcription":"local"}"#)?;
    ensure!(
        legacy.local_model == ModelId::ParakeetV3,
        "Legacy configuration must retain v3"
    );
    match args.first().map(String::as_str) {
        Some("--download") => {
            let cancel = Arc::new(AtomicBool::new(false));
            if let Some(delay) = args.get(2) {
                let delay = Duration::from_millis(delay.parse()?);
                let cancel = cancel.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(delay);
                    cancel.store(true, Ordering::Relaxed);
                });
            }
            return local::download(
                args.get(1).context("Expected model key")?.parse()?,
                &cancel,
                |p| eprintln!("{p}%"),
            );
        }
        Some("--list") => {
            for model in ModelId::ALL {
                println!(
                    "{} installed={} size={}MB",
                    model.key(),
                    model.installed(),
                    model.size_mb()
                );
            }
            return Ok(());
        }
        _ => {}
    }
    let selected = if let Some(index) = args.iter().position(|arg| arg == "--model") {
        args.remove(index);
        ensure!(index < args.len(), "Expected model key");
        Some(args.remove(index).parse::<ModelId>()?)
    } else {
        None
    };
    let auto = if let Some(index) = args.iter().position(|arg| arg == "--auto") {
        args.remove(index);
        true
    } else {
        false
    };
    ensure!(
        args.len() == 2,
        "Expected English and French 16 kHz mono PCM WAV paths"
    );
    let mut count = 0;
    for model in ModelId::ALL
        .into_iter()
        .filter(|m| m.installed() && selected.is_none_or(|selected| *m == selected))
    {
        count += 1;
        for (language, path) in [("en", &args[0]), ("fr", &args[1])] {
            if language != "en" && matches!(model, ModelId::MoonshineBase) {
                continue;
            }
            let samples = wav::read_wav(Path::new(path))?;
            let start = Instant::now();
            let text = local::transcribe_model(
                model,
                if auto {
                    None
                } else {
                    Some(language.to_owned())
                },
                &samples,
            )?;
            ensure!(!text.is_empty(), "{} produced no text", model.key());
            println!(
                "{} {language}: {text:?} | audio={}ms elapsed={}ms (cold load included)",
                model.key(),
                samples.len() / 16,
                start.elapsed().as_millis()
            );
        }
    }
    local::unload();
    ensure!(count > 0, "No installed models");
    Ok(())
}
