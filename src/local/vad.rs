use anyhow::Result;
use std::{ops::Range, path::Path};
use transcribe_cpp::{Backend, Model, ModelOptions, VadOptions, VadSession, VadSessionOptions};

const RATE: usize = 16_000;
const FRAME: usize = 512;
const PAD: usize = RATE / 5;
const MAX: usize = 25 * RATE;

pub(super) struct Vad {
    session: VadSession,
}
impl Vad {
    pub fn load(path: &Path) -> Result<Self> {
        let model = Model::load_with(
            path,
            &ModelOptions {
                backend: Backend::Cpu,
                ..Default::default()
            },
        )?;
        Ok(Self {
            session: model.vad_session_with(&VadSessionOptions { n_threads: 1 })?,
        })
    }

    pub fn chunks(&mut self, samples: &[f32]) -> Result<Vec<Range<usize>>> {
        let result = self.session.run(samples, &VadOptions::default())?;
        Ok(chunks(
            &segments(&result.probs, samples.len()),
            &result.probs,
            samples.len(),
        ))
    }
}

fn segments(probabilities: &[f32], len: usize) -> Vec<Range<usize>> {
    let mut speech = Vec::new();
    let mut start = None;
    let mut quiet = None;
    let mut finish = |start, end| {
        if end - start >= RATE / 10 {
            speech.push(start..end);
        }
    };
    for (index, &probability) in probabilities.iter().enumerate() {
        let at = index * FRAME;
        if probability >= 0.5 {
            start.get_or_insert(at);
            quiet = None;
        } else if probability < 0.35
            && let Some(begin) = start
        {
            let end = *quiet.get_or_insert(at);
            if at + FRAME - end >= RATE / 5 {
                finish(begin, end);
                start = None;
                quiet = None;
            }
        }
    }
    if let Some(begin) = start {
        finish(begin, quiet.unwrap_or(len).min(len));
    }
    speech
}

fn chunks(speech: &[Range<usize>], probabilities: &[f32], len: usize) -> Vec<Range<usize>> {
    let (Some(first), Some(last)) = (speech.first(), speech.last()) else {
        return Vec::new();
    };
    let mut start = first.start.saturating_sub(PAD);
    let end = (last.end + PAD).min(len);
    let mut result = Vec::new();
    while end - start > MAX {
        let boundary = start + MAX;
        let window = boundary - 10 * RATE;
        let cut = speech
            .windows(2)
            .map(|pair| pair[0].end.max(window)..pair[1].start.min(boundary))
            .filter(|pause| pause.end > pause.start)
            .max_by_key(|pause| (pause.len(), pause.end))
            .map_or_else(
                // Without a pause, cut where speech is least likely so words stay whole.
                || {
                    (window / FRAME..boundary / FRAME)
                        .filter_map(|frame| Some((frame, *probabilities.get(frame)?)))
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .map_or(boundary, |(frame, _)| frame * FRAME + FRAME / 2)
                },
                |pause| pause.start + pause.len() / 2,
            );
        result.push(start..cut);
        start = cut;
    }
    result.push(start..end);
    result
        .into_iter()
        .filter_map(|chunk| {
            let first = speech
                .iter()
                .find(|speech| speech.end > chunk.start && speech.start < chunk.end)?;
            let last = speech
                .iter()
                .rev()
                .find(|speech| speech.end > chunk.start && speech.start < chunk.end)?;
            Some(chunk.start.max(first.start.saturating_sub(PAD))..chunk.end.min(last.end + PAD))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothing_bridges_short_dips_and_rejects_clicks() {
        let mut probabilities = vec![0.0; 40];
        probabilities[2] = 0.9;
        probabilities[12..24].fill(0.9);
        probabilities[16..19].fill(0.1);
        probabilities[24..27].fill(0.4);
        assert_eq!(
            segments(&probabilities, 40 * FRAME),
            vec![12 * FRAME..27 * FRAME]
        );
        assert!(segments(&[0.0; 40], 40 * FRAME).is_empty());
    }

    #[test]
    fn padding_preserves_edges_and_release_tail_without_exceeding_audio() {
        assert_eq!(
            chunks(std::slice::from_ref(&(16000..32000)), &[], 48000),
            vec![12800..35200]
        );
        assert_eq!(
            chunks(std::slice::from_ref(&(0..15900)), &[], 16000),
            vec![0..16000]
        );
        assert!(chunks(&[], &[], 48000).is_empty());
    }

    #[test]
    fn split_uses_longest_pause_near_boundary() {
        let speech = [
            0..22 * RATE,
            23 * RATE..24 * RATE,
            24 * RATE + 1600..40 * RATE,
        ];
        let result = chunks(&speech, &[], 41 * RATE);
        assert_eq!(result[0].end, 22 * RATE + PAD);
        assert_eq!(result[1].start, 23 * RATE - PAD);
        assert_eq!(result.last().unwrap().end, 40 * RATE + PAD);
    }

    #[test]
    fn long_silent_gaps_do_not_create_empty_decoder_chunks() {
        assert_eq!(
            chunks(&[0..RATE, 90 * RATE..91 * RATE], &[], 92 * RATE),
            vec![0..RATE + PAD, 90 * RATE - PAD..91 * RATE + PAD]
        );
    }

    #[test]
    fn continuous_speech_is_cut_at_its_least_likely_frame() {
        let mut probabilities = vec![0.9; (61 * RATE).div_ceil(FRAME)];
        probabilities[23 * RATE / FRAME] = 0.6;
        let cut = 23 * RATE / FRAME * FRAME + FRAME / 2;
        let result = chunks(
            std::slice::from_ref(&(0..61 * RATE)),
            &probabilities,
            61 * RATE,
        );
        assert_eq!(result[0], 0..cut);
        assert_eq!(result[1].start, cut);
        assert!(result.windows(2).all(|pair| pair[0].end == pair[1].start));
        assert!(result.iter().all(|chunk| chunk.len() <= MAX));
        assert_eq!(result.last().unwrap().end, 61 * RATE);
    }
}
