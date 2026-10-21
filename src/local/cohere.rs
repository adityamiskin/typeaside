// Decoder and KV-cache handling adapted from voxtype's moonshine.rs and cohere.rs.
// MIT, Copyright 2025 Peter Jackson. See licenses/voxtype-MIT.txt.
use super::features::Features;
use anyhow::{Context, Result, ensure};
use ort::{
    inputs,
    session::{Session, SessionInputValue},
    value::{DynValue, Tensor},
};
use std::{borrow::Cow, collections::BTreeMap, path::Path};
use tokenizers::Tokenizer;

const LANGUAGES: [&str; 14] = [
    "ar", "de", "el", "en", "es", "fr", "it", "ja", "ko", "nl", "pl", "pt", "vi", "zh",
];
const EOS: i64 = 3;

pub(super) struct Model {
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
    features: Features,
    prefix: Vec<i64>,
    caches: Vec<(String, [usize; 4])>,
}
impl Model {
    pub fn load(dir: &Path, language: Option<&str>) -> Result<Self> {
        let session = |name: &str| -> Result<Session> {
            Ok(Session::builder()?
                .with_intra_threads(4)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?
                .commit_from_file(dir.join(name))?)
        };
        let tokenizer =
            Tokenizer::from_file(dir.join("tokenizer.json")).map_err(anyhow::Error::msg)?;
        let lang = language.unwrap_or("en");
        ensure!(
            LANGUAGES.contains(&lang),
            "Cohere does not support language {lang}; choose a language in Settings"
        );
        let language_token = tokenizer
            .token_to_id(&format!("<|{lang}|>"))
            .map(i64::from)
            .with_context(|| format!("Missing token for {lang}"))?;
        let decoder = session("decoder_model_merged_q4.onnx")?;
        let caches = decoder
            .inputs()
            .iter()
            .filter(|input| input.name().starts_with("past_key_values"))
            .map(|input| {
                let shape = input.dtype().tensor_shape().context("Missing KV shape")?;
                ensure!(
                    shape.len() == 4 && shape[1] > 0 && shape[3] > 0,
                    "Invalid KV shape"
                );
                Ok((
                    input.name().to_owned(),
                    [1, shape[1] as usize, 0, shape[3] as usize],
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            encoder: session("encoder_model_q4.onnx")?,
            decoder,
            tokenizer,
            features: Features::new(),
            prefix: vec![4, language_token, 5, 8, 11, 13],
            caches,
        })
    }
    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        let mut parts = Vec::new();
        let mut rest = samples;
        while !rest.is_empty() {
            let cut = quiet_cut(rest, 25 * 16000);
            let speech = trim_quiet(&rest[..cut]);
            if !speech.is_empty() {
                parts.push(self.chunk(speech)?);
            }
            rest = &rest[cut..];
        }
        Ok(parts.join(" "))
    }
    fn chunk(&mut self, samples: &[f32]) -> Result<String> {
        let audio = Tensor::from_array(self.features.extract(samples))?;
        let mut encoded = self.encoder.run(inputs![audio])?;
        let hidden = encoded
            .remove("last_hidden_state")
            .context("Missing encoder hidden state")?;
        drop(encoded);
        let mut caches = BTreeMap::<String, DynValue>::new();
        for (name, shape) in &self.caches {
            let tensor = ort::value::DynTensor::new(
                &ort::memory::Allocator::default(),
                ort::value::TensorElementType::Float32,
                *shape,
            )?;
            caches.insert(name.clone(), tensor.into_dyn());
        }
        let mut tokens = self.prefix.clone();
        let mut generated = Vec::new();
        let max = ((samples.len() / 16000 + 1) * 10).clamp(32, 1024);
        let mut next_input = tokens.clone();
        for _ in 0..max {
            let mut inputs: Vec<(Cow<'_, str>, SessionInputValue<'_>)> = vec![
                (
                    "input_ids".into(),
                    Tensor::from_array(([1, next_input.len()], next_input.clone()))?.into(),
                ),
                ("encoder_hidden_states".into(), (&hidden).into()),
            ];
            for (name, value) in &caches {
                inputs.push((name.as_str().into(), value.into()));
            }
            inputs.push((
                "attention_mask".into(),
                Tensor::from_array(([1, tokens.len()], vec![1_i64; tokens.len()]))?.into(),
            ));
            inputs.push((
                "position_ids".into(),
                Tensor::from_array((
                    [1, next_input.len()],
                    ((tokens.len() - next_input.len()) as i64..tokens.len() as i64)
                        .collect::<Vec<_>>(),
                ))?
                .into(),
            ));
            inputs.push((
                "num_logits_to_keep".into(),
                Tensor::from_array(([] as [usize; 0], vec![1_i64]))?.into(),
            ));
            let mut outputs = self.decoder.run(inputs)?;
            let (shape, logits) = outputs["logits"].try_extract_tensor::<f32>()?;
            let vocab = *shape.last().context("Missing logit shape")? as usize;
            ensure!(vocab > 0 && logits.len() >= vocab, "Empty logits");
            let best = logits[logits.len() - vocab..]
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .context("No decoder token")?
                .0 as i64;
            if best == EOS {
                break;
            }
            for (name, _) in &self.caches {
                let output_name = name.replacen("past_key_values", "present", 1);
                caches.insert(
                    name.clone(),
                    outputs
                        .remove(&output_name)
                        .with_context(|| format!("Missing {output_name}"))?,
                );
            }
            tokens.push(best);
            generated.push(best as u32);
            next_input = vec![best];
        }
        Ok(self
            .tokenizer
            .decode(&generated, true)
            .map_err(anyhow::Error::msg)?
            .trim()
            .to_owned())
    }
}

const FRAME: usize = 160;

fn energy(frame: &[f32]) -> f32 {
    frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32
}

/// Fixed-length cuts split words, so end each chunk at the quietest 10 ms in its last 3 s.
fn quiet_cut(samples: &[f32], max: usize) -> usize {
    if samples.len() <= max {
        return samples.len();
    }
    ((max - 3 * 16000)..max)
        .step_by(FRAME)
        .min_by(|&a, &b| energy(&samples[a..a + FRAME]).total_cmp(&energy(&samples[b..b + FRAME])))
        .map_or(max, |at| at + FRAME / 2)
}

/// Autoregressive decoders can return nothing or hallucinate across long quiet stretches, so drop quiet edges.
fn trim_quiet(samples: &[f32]) -> &[f32] {
    const PAD: usize = 16000 / 5;
    let frames: Vec<f32> = samples.chunks(FRAME).map(energy).collect();
    let peak = frames.iter().copied().fold(0., f32::max);
    let loud = |e: &f32| *e > peak * 0.01;
    let (Some(first), Some(last)) = (frames.iter().position(loud), frames.iter().rposition(loud))
    else {
        return &[];
    };
    let start = (first * FRAME).saturating_sub(PAD);
    let end = ((last + 1) * FRAME + PAD).min(samples.len());
    &samples[start..end]
}

#[cfg(test)]
mod tests {
    use super::{quiet_cut, trim_quiet};

    #[test]
    fn long_audio_is_cut_in_the_quietest_gap() {
        let mut samples = vec![0.5; 40 * 16000];
        samples[23 * 16000..23 * 16000 + 1600].fill(0.);
        let cut = quiet_cut(&samples, 25 * 16000);
        assert!((23 * 16000..23 * 16000 + 1600).contains(&cut));
        assert_eq!(quiet_cut(&samples[..1000], 25 * 16000), 1000);
    }

    #[test]
    fn quiet_edges_are_trimmed_with_padding() {
        let mut samples = vec![0.001; 3 * 16000];
        samples[16000..32000].fill(0.3);
        assert_eq!(trim_quiet(&samples).len(), 16000 + 2 * 3200);
        assert!(trim_quiet(&[0.; 1600]).is_empty());
    }
}
