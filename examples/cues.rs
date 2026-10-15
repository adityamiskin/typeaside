use anyhow::Result;
use hermes::cues::{self, Cue};
use std::path::PathBuf;

const RATE: u32 = 48_000;

fn main() -> Result<()> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or("/tmp/hermes-sounds".into()),
    );
    std::fs::create_dir_all(&dir)?;
    for cue in Cue::ALL {
        let samples = cues::render(cue, RATE);
        let mut wav = Vec::with_capacity(44 + samples.len() * 2);
        let data = (samples.len() * 2) as u32;
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        for field in [16u32, 1 | (1 << 16), RATE, RATE * 2, 2 | (16 << 16)] {
            wav.extend_from_slice(&field.to_le_bytes());
        }
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&((sample.clamp(-1., 1.) * 32767.) as i16).to_le_bytes());
        }
        let path = dir.join(format!("{cue:?}.wav").to_lowercase());
        std::fs::write(&path, wav)?;
        println!("{}", path.display());
    }
    Ok(())
}
