use crate::{config::AppConfig, credentials::endpoint_key, paths::AppPaths};
use anyhow::{Context, Result};
use reqwest::blocking::{Client, multipart};
use serde::Deserialize;
use std::time::Duration;

/// Presses under 250 ms are accidental taps (the hotkey doubles as a modifier).
pub(crate) fn is_tap(audio: &[f32]) -> bool {
    audio.len() < 4000
}

pub(crate) fn has_speech(audio: &[f32]) -> bool {
    (audio.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / audio.len().max(1) as f64).sqrt()
        >= 0.003
}

pub(crate) fn transcribe(config: &AppConfig, paths: &AppPaths, audio: &[f32]) -> Result<String> {
    request(config, audio, &endpoint_key(paths, &config.stt.base_url)?)
}

fn request(config: &AppConfig, audio: &[f32], key: &str) -> Result<String> {
    let file = multipart::Part::bytes(encode_wav(audio)?)
        .file_name("dictation.wav")
        .mime_str("audio/wav")?;
    let mut form = multipart::Form::new()
        .part("file", file)
        .text("model", config.stt.model.clone());
    if let Some(language) = &config.language {
        form = form.text("language", language.clone());
    }
    if !config.dictionary.is_empty() {
        form = form.text("prompt", config.dictionary.join(", "));
    }
    #[derive(Deserialize)]
    struct Response {
        text: String,
    }
    let response = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()?
        .post(format!(
            "{}/audio/transcriptions",
            config.stt.base_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .multipart(form)
        .send()?
        .error_for_status()?
        .json::<Response>()?;
    Ok(response.text.trim().to_owned())
}

fn encode_wav(audio: &[f32]) -> Result<Vec<u8>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    #[test]
    fn wav_and_multipart_request() -> Result<()> {
        let wav = encode_wav(&[0.0, 1.0, -1.0])?;
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into()?), 42);
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes(wav[20..22].try_into()?), 1);
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into()?), 1);
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into()?), 16_000);
        assert_eq!(u32::from_le_bytes(wav[28..32].try_into()?), 32_000);
        assert_eq!(u16::from_le_bytes(wav[32..34].try_into()?), 2);
        assert_eq!(u16::from_le_bytes(wav[34..36].try_into()?), 16);
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into()?), 6);
        assert_eq!(&wav[44..], &[0, 0, 255, 127, 1, 128]);
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}", listener.local_addr()?);
        let mock = thread::spawn(move || -> Result<()> {
            let (mut stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            let mut bytes = Vec::new();
            let mut chunk = [0; 8192];
            loop {
                let count = stream.read(&mut chunk)?;
                anyhow::ensure!(count > 0, "request ended early");
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(str::to_owned)
                        })
                        .context("no content length")?
                        .parse()?;
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let request = String::from_utf8_lossy(&bytes);
            assert!(request.starts_with("POST /audio/transcriptions HTTP/1.1"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer test-key")
            );
            for field in [
                "name=\"file\"; filename=\"dictation.wav\"",
                "audio/wav",
                "name=\"model\"\r\n\r\ntest-model",
                "name=\"language\"\r\n\r\nen",
                "name=\"prompt\"\r\n\r\nHermes, Aditya",
            ] {
                assert!(request.contains(field), "missing {field}");
            }
            let body = r#"{"text":"Hello."}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )?;
            Ok(())
        });
        let mut config = AppConfig::default();
        config.stt = crate::config::Endpoint {
            base_url: url,
            model: "test-model".into(),
        };
        config.language = Some("en".into());
        config.dictionary = vec!["Hermes".into(), "Aditya".into()];
        assert_eq!(request(&config, &[0.1; 4800], "test-key")?, "Hello.");
        mock.join()
            .map_err(|_| anyhow::anyhow!("mock panicked"))??;
        assert!(is_tap(&[0.2; 3999]));
        assert!(!has_speech(&[0.0; 8000]));
        Ok(())
    }
}
