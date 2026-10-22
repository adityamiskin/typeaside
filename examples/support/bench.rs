use super::wav;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use typeaside::local::{self, ModelId};

const SAMPLES_PER_MS: usize = 16;
const RELEASE_CUTS_MS: [usize; 3] = [0, 100, 200];
const SPEECH_THRESHOLD: f32 = 0.01;

#[derive(Deserialize)]
struct Manifest {
    models: Vec<ModelId>,
    iterations: usize,
    clips: Vec<ClipSpec>,
}

#[derive(Deserialize)]
struct ClipSpec {
    name: String,
    language: Option<String>,
    #[serde(default = "once")]
    repeat: usize,
    #[serde(default)]
    gap_ms: usize,
    parts: Vec<Part>,
}

fn once() -> usize {
    1
}

#[derive(Deserialize)]
struct Part {
    wav: PathBuf,
    text: String,
    end_ms: Option<usize>,
}

struct Clip {
    name: String,
    language: Option<String>,
    samples: Vec<f32>,
    reference: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Record<'a> {
    Run {
        model: &'a str,
        clip: &'a str,
        audio_ms: usize,
        phase: &'static str,
        elapsed_ms: f64,
        rtf: f64,
        wer: f64,
        transcript: &'a str,
    },
    Release {
        model: &'a str,
        clip: &'a str,
        cut_ms: usize,
        final_word: &'a str,
        survived: bool,
        transcript: &'a str,
    },
    Summary {
        model: &'a str,
        warm_runs: usize,
        median_cold_ms: f64,
        median_rtf: f64,
        p95_ms: f64,
        mean_wer: f64,
        release_survived: [usize; RELEASE_CUTS_MS.len()],
        clips: usize,
    },
}

impl ClipSpec {
    fn build(self, dir: &Path) -> Result<Clip> {
        let mut samples = Vec::new();
        let mut reference = Vec::new();
        for _ in 0..self.repeat {
            for part in &self.parts {
                if !samples.is_empty() {
                    samples.resize(samples.len() + self.gap_ms * SAMPLES_PER_MS, 0.0);
                }
                let mut audio = wav::read_wav(&dir.join(&part.wav))?;
                if let Some(end) = part.end_ms {
                    audio.truncate(end * SAMPLES_PER_MS);
                }
                samples.extend(audio);
                reference.push(part.text.as_str());
            }
        }
        ensure!(!samples.is_empty(), "clip {} is empty", self.name);
        Ok(Clip {
            reference: reference.join(" "),
            name: self.name,
            language: self.language,
            samples,
        })
    }
}

pub fn run(manifest: &Path, output: &Path) -> Result<()> {
    let Manifest {
        models,
        iterations,
        clips,
    } = serde_json::from_reader(File::open(manifest)?)?;
    let dir = manifest.parent().context("manifest has no directory")?;
    let clips = clips
        .into_iter()
        .map(|clip| clip.build(dir))
        .collect::<Result<Vec<_>>>()?;
    let mut out = BufWriter::new(File::create(output)?);
    let mut emit = |record: &Record| -> Result<()> {
        serde_json::to_writer(&mut out, record)?;
        writeln!(out)?;
        Ok(())
    };
    println!("model         warm  median_rtf  p95_ms  mean_wer  cold_ms  final_word@0/100/200ms");
    for model in models {
        ensure!(model.installed(), "{} is not installed", model.key());
        let key = model.key();
        let (mut cold, mut warm_ms, mut rtfs, mut wers) = (vec![], vec![], vec![], vec![]);
        let mut survived = [0; RELEASE_CUTS_MS.len()];
        for clip in &clips {
            local::unload();
            let audio_ms = clip.samples.len() / SAMPLES_PER_MS;
            for iteration in 0..=iterations {
                let start = Instant::now();
                let transcript =
                    local::transcribe_model(model, clip.language.clone(), &clip.samples)?;
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.;
                let rtf = elapsed_ms / audio_ms as f64;
                let wer = wer(&clip.reference, &transcript);
                if iteration == 0 {
                    cold.push(elapsed_ms);
                } else {
                    warm_ms.push(elapsed_ms);
                    rtfs.push(rtf);
                    wers.push(wer);
                }
                emit(&Record::Run {
                    model: key,
                    clip: &clip.name,
                    audio_ms,
                    phase: if iteration == 0 { "cold" } else { "warm" },
                    elapsed_ms,
                    rtf,
                    wer,
                    transcript: &transcript,
                })?;
            }
            let reference = words(&clip.reference);
            let final_word = reference.last().context("empty reference")?;
            let speech_end = clip
                .samples
                .iter()
                .rposition(|sample| sample.abs() > SPEECH_THRESHOLD)
                .map_or(0, |end| end + 1);
            for (cut_ms, survived) in RELEASE_CUTS_MS.into_iter().zip(&mut survived) {
                let end = speech_end.saturating_sub(cut_ms * SAMPLES_PER_MS);
                let transcript =
                    local::transcribe_model(model, clip.language.clone(), &clip.samples[..end])?;
                let kept = words(&transcript).last() == Some(final_word);
                *survived += usize::from(kept);
                emit(&Record::Release {
                    model: key,
                    clip: &clip.name,
                    cut_ms,
                    final_word,
                    survived: kept,
                    transcript: &transcript,
                })?;
            }
        }
        local::unload();
        let median_cold_ms = percentile(&mut cold, 0.5);
        let median_rtf = percentile(&mut rtfs, 0.5);
        let p95_ms = percentile(&mut warm_ms, 0.95);
        let mean_wer = wers.iter().sum::<f64>() / wers.len().max(1) as f64;
        println!(
            "{key:<13} {:>4}  {median_rtf:>10.3}  {p95_ms:>6.0}  {mean_wer:>8.3}  {median_cold_ms:>7.0}  {} of {}",
            warm_ms.len(),
            survived.map(|n| n.to_string()).join("/"),
            clips.len()
        );
        emit(&Record::Summary {
            model: key,
            warm_runs: warm_ms.len(),
            median_cold_ms,
            median_rtf,
            p95_ms,
            mean_wer,
            release_survived: survived,
            clips: clips.len(),
        })?;
    }
    out.flush()?;
    Ok(())
}

fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    values[((values.len() as f64 * p).ceil() as usize).clamp(1, values.len()) - 1]
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

fn wer(reference: &str, hypothesis: &str) -> f64 {
    let (reference, hypothesis) = (words(reference), words(hypothesis));
    let mut row: Vec<usize> = (0..=hypothesis.len()).collect();
    for (i, expected) in reference.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, actual) in hypothesis.iter().enumerate() {
            let substitution = diagonal + usize::from(expected != actual);
            diagonal = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(diagonal + 1);
        }
    }
    row[hypothesis.len()] as f64 / reference.len().max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::{wer, words};

    #[test]
    fn normalizes_case_and_punctuation() {
        assert_eq!(
            words("Bonjour, la RÉUNION est demain à 3h. «Merci»  !"),
            [
                "bonjour", "la", "réunion", "est", "demain", "à", "3h", "merci"
            ]
        );
        assert_eq!(
            wer("Please bring your notes.", "please, bring YOUR notes"),
            0.
        );
    }

    #[test]
    fn counts_word_edits() {
        assert_eq!(wer("a b c d", "a x c d"), 0.25);
        assert_eq!(wer("a b c d", "a c d"), 0.25);
        assert_eq!(wer("a b c d", "a b b c d"), 0.25);
        assert_eq!(wer("a b c d", ""), 1.);
        assert_eq!(wer("a b", "c d e f"), 2.);
        assert_eq!(wer("at three", "at 3"), 0.5);
    }
}
