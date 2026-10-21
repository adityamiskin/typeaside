use anyhow::{Context, Result, ensure};
use std::path::Path;

pub fn read_wav(path: &Path) -> Result<Vec<f32>> {
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
