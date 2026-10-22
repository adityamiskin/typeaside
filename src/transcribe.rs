use crate::{config::AppConfig, credentials::endpoint_key};
use anyhow::{Context, Result};
use reqwest::blocking::multipart;
use serde::Deserialize;
use std::{io::Read, path::Path, time::Duration};

/// Presses under 250 ms are accidental taps (the hotkey doubles as a modifier).
pub(crate) fn is_tap(audio: &[f32]) -> bool {
    audio.len() < 4000
}

pub(crate) fn has_speech(audio: &[f32]) -> bool {
    (audio.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / audio.len().max(1) as f64).sqrt()
        >= 0.003
}

pub(crate) fn transcribe(config: &AppConfig, audio: &[f32], names: &[String]) -> Result<String> {
    let file = multipart::Part::bytes(encode_wav(audio)?)
        .file_name("dictation.wav")
        .mime_str("audio/wav")?;
    request(
        config,
        file,
        &endpoint_key(&config.stt.base_url)?,
        names,
        Duration::from_secs(30),
    )
}

pub fn transcribe_file(config: &AppConfig, path: &Path) -> Result<String> {
    anyhow::ensure!(
        config.transcription != crate::config::Source::Local,
        "File transcription needs a TypeAside account or your own key"
    );
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "flac" => "audio/flac",
        "mp3" | "mpga" => "audio/mpeg",
        "mp4" => "video/mp4",
        "mpeg" => "video/mpeg",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "webm" => "video/webm",
        _ => anyhow::bail!("Unsupported file type"),
    };
    // Convex HTTP actions, which back the hosted route, reject bodies over 20 MB.
    anyhow::ensure!(
        std::fs::metadata(path)?.len() <= 20_000_000,
        "Files up to 20 MB"
    );
    let bytes = std::fs::read(path)?;
    let file = multipart::Part::bytes(bytes)
        .file_name(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        )
        .mime_str(mime)?;
    if config.transcription == crate::config::Source::Hosted {
        return Ok(crate::pipeline::hosted(
            config,
            Some(file),
            None,
            crate::engine::Mode::File,
            "",
            &[],
            None,
        )?
        .text);
    }
    let raw = request(
        config,
        file,
        &endpoint_key(&config.stt.base_url)?,
        &[],
        Duration::from_secs(120),
    )?;
    Ok(crate::replacements::apply(&raw, &config.replacements))
}

fn request(
    config: &AppConfig,
    file: multipart::Part,
    key: &str,
    names: &[String],
    timeout: Duration,
) -> Result<String> {
    let mut form = multipart::Form::new()
        .part("file", file)
        .text("model", config.stt.model.clone());
    if let Some(language) = &config.language {
        form = form.text("language", language.clone());
    }
    let prompt = crate::context::prompt(&config.dictionary, names);
    if !prompt.is_empty() {
        form = form.text("prompt", prompt);
    }
    // Streaming blocking bodies can hide connection errors behind a closed upload channel.
    let content_type = format!("multipart/form-data; boundary={}", form.boundary());
    let mut body = Vec::new();
    form.into_reader().read_to_end(&mut body)?;
    #[derive(Deserialize)]
    struct Response {
        text: String,
    }
    let response = crate::http::checked(
        crate::http::client()
            .post(format!(
                "{}/audio/transcriptions",
                config.stt.base_url.trim_end_matches('/')
            ))
            .bearer_auth(key)
            .timeout(timeout)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body)
            .send()?,
    )?
    .json::<Response>()?;
    Ok(response.text.trim().to_owned())
}

pub(crate) fn encode_wav(audio: &[f32]) -> Result<Vec<u8>> {
    let bytes = u32::try_from(audio.len().checked_mul(2).context("audio too large")?)?;
    let riff_size = bytes.checked_add(36).context("audio too large")?;
    let mut wav = Vec::with_capacity(audio.len() * 2 + 44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&16_000u32.to_le_bytes());
    wav.extend_from_slice(&32_000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&bytes.to_le_bytes());
    for sample in audio {
        wav.extend_from_slice(&((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    Ok(wav)
}

/// Half the size of WAV, which matters on high-latency uploads to the hosted route.
pub(crate) fn encode_flac(audio: &[f32]) -> Result<Vec<u8>> {
    use flacenc::{component::BitRepr, error::Verify};
    let samples: Vec<i32> = audio
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i32)
        .collect();
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, error)| anyhow::anyhow!("FLAC config: {error}"))?;
    let source = flacenc::source::MemSource::from_samples(&samples, 1, 16, 16_000);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|error| anyhow::anyhow!("FLAC encode: {error}"))?;
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|error| anyhow::anyhow!("FLAC write: {error}"))?;
    Ok(sink.into_inner())
}

pub(crate) fn decode_wav(wav: &[u8]) -> Result<Vec<f32>> {
    anyhow::ensure!(wav.len() >= 44, "incomplete recording WAV");
    anyhow::ensure!(
        &wav[..4] == b"RIFF"
            && &wav[8..20] == b"WAVEfmt \x10\x00\x00\x00"
            && wav[20..36] == [1, 0, 1, 0, 128, 62, 0, 0, 0, 125, 0, 0, 2, 0, 16, 0]
            && &wav[36..40] == b"data",
        "unsupported recording WAV"
    );
    let size = u32::from_le_bytes(wav[40..44].try_into()?) as usize;
    anyhow::ensure!(
        size == wav.len() - 44 && size.is_multiple_of(2),
        "incomplete recording WAV"
    );
    Ok(wav[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|sample| f32::from(i16::from_le_bytes([sample[0], sample[1]])) / f32::from(i16::MAX))
        .collect())
}
