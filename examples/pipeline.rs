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
        "# TypeAside real pipeline\n\nRepeat: `cargo run --example pipeline -- --fixtures`\nRegenerate speech: add `--generate-fixtures`. TTS: gpt-4o-mini-tts/alloy. STT: whisper-large-v3-turbo (Groq). LLM: gpt-6-luna (OpenAI). No desktop input or audio device access.\nTranslation case explicitly disables cleanup. Timings include keyring access; no prewarming in this offline-file runner.\n\n| Case | Raw | Output | STT ms | LLM ms | Total ms |\n| --- | --- | --- | ---: | ---: | ---: |\n",
    );
    for (name, _, translation, selection) in cases {
        let mut config = fixture_config();
        config.translate_to = translation.map(str::to_owned);
        if translation.is_some() {
            config.cleanup.enabled = false;
        }
        let audio = read_wav(Path::new(&format!("tests/fixtures/{name}.wav")))?;
        let started = Instant::now();
        let output = pipeline::run(
            &config,
            &audio,
            Some("org.mozilla.Thunderbird"),
            if selection.is_some() {
                Mode::Command
            } else {
                Mode::Dictate
            },
            selection.unwrap_or_default(),
            || {},
        )?;
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

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let input = args.next().context("usage: pipeline WAV [--app ID] [--translate LANGUAGE] [--command SELECTION], or --fixtures [--generate-fixtures]")?;
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
    let mut selection = None;
    while let Some(flag) = args.next() {
        let value = args.next().context("missing option value")?;
        match flag.as_str() {
            "--app" => app = Some(value),
            "--translate" => config.translate_to = Some(value),
            "--command" => selection = Some(value),
            _ => bail!("unknown option {flag}"),
        }
    }
    let audio = read_wav(Path::new(&input))?;
    let output = pipeline::run(
        &config,
        &audio,
        app.as_deref(),
        if selection.is_some() {
            Mode::Command
        } else {
            Mode::Dictate
        },
        selection.as_deref().unwrap_or_default(),
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
