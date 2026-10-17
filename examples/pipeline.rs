use anyhow::{Context, Result, bail, ensure};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use typeaside::{
    config::{AppConfig, Snippet},
    credentials,
    engine::Mode,
    paths::AppPaths,
    pipeline,
    text_injector::FocusedApp,
};

fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let bytes = std::fs::read(path)?;
    ensure!(
        bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WAVE"),
        "expected RIFF WAV"
    );
    let mut offset = 12usize;
    let mut valid = false;
    let mut audio = None;
    while offset + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
        let start = offset + 8;
        let end = start.checked_add(size).context("invalid WAV length")?;
        let chunk = bytes.get(start..end).context("truncated WAV")?;
        match &bytes[offset..offset + 4] {
            b"fmt " if size >= 16 => {
                valid = u16::from_le_bytes(chunk[..2].try_into()?) == 1
                    && u16::from_le_bytes(chunk[2..4].try_into()?) == 1
                    && u32::from_le_bytes(chunk[4..8].try_into()?) == 16_000
                    && u16::from_le_bytes(chunk[14..16].try_into()?) == 16;
            }
            b"data" => audio = Some(chunk),
            _ => {}
        }
        offset = end.checked_add(size % 2).context("invalid WAV padding")?;
    }
    ensure!(valid, "expected 16 kHz mono s16 PCM WAV");
    let audio = audio.context("missing WAV data")?;
    ensure!(audio.len() % 2 == 0, "incomplete PCM sample");
    Ok(audio
        .as_chunks::<2>()
        .0
        .iter()
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32768.)
        .collect())
}

fn fixture_config() -> AppConfig {
    AppConfig {
        language: Some("en".into()),
        snippets: vec![Snippet {
            trigger: "my calendar link".into(),
            text: "https://example.com/calendar".into(),
        }],
        ..AppConfig::default()
    }
}

fn fixtures(generate: bool) -> Result<()> {
    let cases = [
        (
            "cleanup",
            "um so let's meet at 2, actually 3 tomorrow at the uh cafe",
            None,
            None,
        ),
        ("snippet", "my calendar link", None, None),
        (
            "translation",
            "The meeting will start tomorrow at three in the afternoon.",
            Some("Spanish"),
            None,
        ),
        (
            "command",
            "make this more formal",
            None,
            Some("hey can u send the deck"),
        ),
        ("answer", "What is the capital of France?", None, Some("")),
    ];
    std::fs::create_dir_all("tests/fixtures")?;
    std::fs::create_dir_all("/tmp/typeaside-perf")?;
    if generate {
        let key = credentials::get_credential("api.openai.com")?
            .context("no OpenAI key in system keyring")?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        for (name, text, _, _) in &cases {
            let audio = client.post("https://api.openai.com/v1/audio/speech").bearer_auth(&key)
                .json(&serde_json::json!({"model":"gpt-4o-mini-tts", "voice":"alloy", "input":text, "response_format":"wav"}))
                .send()?.error_for_status()?.bytes()?;
            let mut ffmpeg = Command::new("ffmpeg")
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-y",
                    "-i",
                    "pipe:0",
                    "-ar",
                    "16000",
                    "-ac",
                    "1",
                    "-c:a",
                    "pcm_s16le",
                ])
                .arg(format!("tests/fixtures/{name}.wav"))
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()?;
            ffmpeg
                .stdin
                .take()
                .context("ffmpeg stdin")?
                .write_all(&audio)?;
            ensure!(ffmpeg.wait()?.success(), "ffmpeg conversion failed");
        }
    }
    let mut report = String::from(
        "# TypeAside real pipeline\n\nRepeat: `cargo run --example pipeline -- --fixtures`\nRegenerate speech: add `--generate-fixtures`. TTS: gpt-4o-mini-tts/alloy; answer.wav: espeak-ng/en-us. STT: whisper-large-v3-turbo (Groq). LLM: qwen/qwen3.8-27b (Groq). No desktop input or audio device access.\nTranslation case explicitly disables cleanup. Timings include keyring access; no prewarming in this offline-file runner.\n\n| Case | Raw | Output | STT ms | LLM ms | Total ms |\n| --- | --- | --- | ---: | ---: | ---: |\n",
    );
    for (index, (name, _, translation, selection)) in cases.into_iter().enumerate() {
        // Each cleanup reserves its max_tokens against Groq's 8k tokens/minute on free keys.
        if index > 0 {
            std::thread::sleep(Duration::from_secs(20));
        }
        let mut config = fixture_config();
        if let Some(language) = translation {
            config.translate_to = language.into();
        }
        if translation.is_some() {
            config.cleanup.enabled = false;
        }
        let audio = read_wav(Path::new(&format!("tests/fixtures/{name}.wav")))?;
        let started = Instant::now();
        let output = pipeline::run(
            &config,
            &audio,
            Some(&FocusedApp {
                id: Some("org.mozilla.Thunderbird".into()),
                title: None,
            }),
            if selection.is_some() {
                Mode::Command
            } else if translation.is_some() {
                Mode::Translate
            } else {
                Mode::Dictate
            },
            selection.unwrap_or_default(),
            &[],
            || {},
        )?;
        if name == "translation" {
            let translated = output.text.to_lowercase();
            ensure!(
                translated.contains("reunión") && translated.contains("mañana"),
                "expected Spanish translation: {}",
                output.text
            );
        }
        if name == "answer" {
            let answer = output.text.to_lowercase();
            ensure!(
                answer.contains("paris"),
                "answer must contain Paris: {}",
                output.text
            );
            ensure!(
                !["sure", "here", "certainly", "of course", "as an ai"]
                    .iter()
                    .any(|prefix| answer.starts_with(prefix)),
                "answer must have no preamble: {}",
                output.text
            );
            ensure!(
                output.text.lines().count() == 1 && !output.text.contains(['*', '#', '`']),
                "answer must be plain text: {}",
                output.text
            );
        }
        let escape = |value: &str| value.replace('|', "\\|").replace('\n', "<br>");
        report.push_str(&format!(
            "| {name} | {} | {} | {} | {} | {} |\n",
            escape(&output.raw),
            escape(&output.text),
            output.stt.as_millis(),
            output.llm.as_millis(),
            started.elapsed().as_millis()
        ));
        std::fs::write("/tmp/typeaside-perf/pipeline.md", &report)?;
        println!(
            "{name}: raw={:?} output={:?} stt={}ms llm={}ms",
            output.raw,
            output.text,
            output.stt.as_millis(),
            output.llm.as_millis()
        );
    }
    println!("Artifact: /tmp/typeaside-perf/pipeline.md");
    Ok(())
}

fn local_benchmark(measure_idle: bool) -> Result<()> {
    use typeaside::config::Source;
    let root = Path::new("/tmp/typeaside-local-stt");
    std::fs::create_dir_all(root)?;
    let mut clips = Vec::new();
    for name in ["cleanup", "command", "snippet", "translation"] {
        clips.push((
            name.to_owned(),
            read_wav(Path::new(&format!("tests/fixtures/{name}.wav")))?,
        ));
    }
    let mut combined = Vec::new();
    while combined.len() < 30 * 16_000 {
        for (_, samples) in &clips {
            combined.extend_from_slice(samples);
        }
    }
    combined.truncate(30 * 16_000);
    clips.push(("concatenated-30s".into(), combined));
    let mut results = Vec::new();
    for (name, audio) in clips {
        for source in [Source::OwnKey, Source::Local] {
            let mut config = fixture_config();
            config.transcription = source;
            config.cleanup.enabled = false;
            let start = Instant::now();
            let result = pipeline::run(&config, &audio, None, Mode::Dictate, "", &[], || {});
            let mut item = serde_json::json!({ "clip": name, "source": source, "audio_ms": audio.len() / 16, "elapsed_ms": start.elapsed().as_millis() });
            match result {
                Ok(output) => {
                    item["raw"] = output.raw.into();
                    item["text"] = output.text.into();
                    item["stt_ms"] = (output.stt.as_millis() as u64).into();
                }
                Err(error) => item["error"] = format!("{error:#}").into(),
            }
            #[cfg(target_os = "linux")]
            {
                item["process_status"] = std::fs::read_to_string("/proc/self/status")?.into();
            }
            println!("{item}");
            results.push(item);
            std::fs::write(
                root.join("results.json"),
                serde_json::to_vec_pretty(&results)?,
            )?;
        }
    }
    if measure_idle {
        println!("Waiting 305 seconds for automatic model unload");
        std::thread::sleep(Duration::from_secs(305));
    }
    typeaside::local::unload();
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let input = args.next().unwrap_or_else(|| "--fixtures".into());
    if input == "--local-benchmark" {
        return local_benchmark(args.any(|arg| arg == "--measure-idle"));
    }
    if input == "--local-worker" {
        return typeaside::local::worker_main();
    }
    if input == "--download-model" {
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        if let Some(milliseconds) = args.next() {
            let delay = Duration::from_millis(milliseconds.parse()?);
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                std::thread::sleep(delay);
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        }
        return typeaside::local::download(&cancel, |percent| eprintln!("Downloading {percent}%"));
    }
    let paths = AppPaths::discover()?;
    if input == "--fixtures" {
        return fixtures(args.any(|arg| arg == "--generate-fixtures"));
    }
    let mut config = match std::fs::read(&paths.config_file) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => AppConfig::default(),
        Err(error) => return Err(error.into()),
    };
    let mut app = None;
    let mut title = None;
    let mut selection = None;
    let mut mode = Mode::Dictate;
    while let Some(flag) = args.next() {
        if flag == "--local" {
            config.transcription = typeaside::config::Source::Local;
            continue;
        }
        let value = args.next().context("missing option value")?;
        match flag.as_str() {
            "--app" => app = Some(value),
            "--title" => title = Some(value),
            "--translate" => {
                config.translate_to = value;
                mode = Mode::Translate;
            }
            "--command" => {
                selection = Some(value);
                mode = Mode::Command;
            }
            _ => bail!("unknown option {flag}"),
        }
    }
    let audio = read_wav(Path::new(&input))?;
    let output = pipeline::run(
        &config,
        &audio,
        Some(&FocusedApp { id: app, title }),
        mode,
        selection.as_deref().unwrap_or_default(),
        &[],
        || {},
    )?;
    println!(
        "raw: {}\noutput: {}\nstt: {} ms\nllm: {} ms",
        output.raw,
        output.text,
        output.stt.as_millis(),
        output.llm.as_millis()
    );
    Ok(())
}
