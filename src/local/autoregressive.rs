// Decoder and KV-cache handling adapted from voxtype's moonshine.rs and cohere.rs.
// MIT, Copyright 2025 Peter Jackson. See licenses/voxtype-MIT.txt.
use super::{ModelId, features::Features};
use anyhow::{Context, Result, ensure};
use ort::{
    inputs,
    session::{Session, SessionInputValue},
    value::{DynValue, Tensor},
};
use std::{borrow::Cow, collections::BTreeMap, path::Path};
use tokenizers::Tokenizer;

pub(super) struct Model {
    id: ModelId,
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
    features: Option<Features>,
    prefix: Vec<i64>,
    eos: i64,
    suppress: Vec<usize>,
    languages: Vec<i64>,
    caches: Vec<(String, [usize; 4])>,
}
impl Model {
    pub fn load(id: ModelId, dir: &Path, language: Option<&str>) -> Result<Self> {
        let session = |name: &str| -> Result<Session> {
            Ok(Session::builder()?
                .with_intra_threads(4)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?
                .commit_from_file(dir.join(name))?)
        };
        let cohere = id == ModelId::Cohere;
        let whisper = id == ModelId::WhisperTurbo;
        let (enc, dec) = if cohere {
            ("encoder_model_q4.onnx", "decoder_model_merged_q4.onnx")
        } else {
            (
                "encoder_model_quantized.onnx",
                "decoder_model_merged_quantized.onnx",
            )
        };
        let tokenizer =
            Tokenizer::from_file(dir.join("tokenizer.json")).map_err(anyhow::Error::msg)?;
        let token = |text: &str| {
            tokenizer
                .token_to_id(text)
                .map(i64::from)
                .with_context(|| format!("Missing token {text}"))
        };
        let mut languages = Vec::new();
        let mut suppress = Vec::new();
        let (prefix, eos) = if cohere {
            let lang = language.unwrap_or("en");
            ensure!(
                [
                    "ar", "de", "el", "en", "es", "fr", "it", "ja", "ko", "nl", "pl", "pt", "vi",
                    "zh"
                ]
                .contains(&lang),
                "Cohere does not support language {lang}; choose a language in Settings"
            );
            (vec![4, token(&format!("<|{lang}|>"))?, 5, 8, 11, 13], 3)
        } else if whisper {
            let config: serde_json::Value =
                serde_json::from_slice(&std::fs::read(dir.join("generation_config.json"))?)?;
            languages = config["lang_to_id"]
                .as_object()
                .context("Missing Whisper languages")?
                .values()
                .filter_map(|v| v.as_i64())
                .collect();
            suppress = config["suppress_tokens"]
                .as_array()
                .context("Missing Whisper suppression")?
                .iter()
                .filter_map(|v| v.as_u64().map(|v| v as usize))
                .collect();
            let prefix = if let Some(language) = language {
                vec![50258, token(&format!("<|{language}|>"))?, 50360, 50364]
            } else {
                vec![50258]
            };
            (prefix, 50257)
        } else {
            (vec![1], 2)
        };
        let decoder = session(dec)?;
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
                    [
                        1,
                        shape[1] as usize,
                        if cohere { 0 } else { 1 },
                        shape[3] as usize,
                    ],
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            id,
            encoder: session(enc)?,
            decoder,
            tokenizer,
            features: (cohere || whisper).then(|| Features::new(cohere)),
            prefix,
            eos,
            suppress,
            languages,
            caches,
        })
    }
    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }
        let chunk_size = if self.id == ModelId::WhisperTurbo {
            30 * 16000
        } else {
            25 * 16000
        };
        let mut parts = Vec::new();
        for chunk in samples.chunks(chunk_size) {
            parts.push(self.chunk(chunk)?);
        }
        Ok(parts.join(" "))
    }
    fn chunk(&mut self, samples: &[f32]) -> Result<String> {
        let cohere = self.id == ModelId::Cohere;
        let whisper = self.id == ModelId::WhisperTurbo;
        let audio = if let Some(features) = &self.features {
            Tensor::from_array(features.extract(samples))?
        } else {
            Tensor::from_array(([1, samples.len()], samples.to_vec()))?
        };
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
        let max = ((samples.len() / 16000 + 1) * 10).clamp(
            32,
            if whisper {
                448
            } else if cohere {
                1024
            } else {
                512
            },
        );
        let mut next_input = tokens.clone();
        for step in 0..max {
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
            if cohere {
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
            } else {
                inputs.push((
                    "use_cache_branch".into(),
                    Tensor::from_array(([1], vec![step > 0]))?.into(),
                ));
            }
            if whisper {
                inputs.push((
                    "cache_position".into(),
                    Tensor::from_array((
                        [next_input.len()],
                        ((tokens.len() - next_input.len()) as i64..tokens.len() as i64)
                            .collect::<Vec<_>>(),
                    ))?
                    .into(),
                ));
            }
            let mut outputs = self.decoder.run(inputs)?;
            let (shape, logits) = outputs["logits"].try_extract_tensor::<f32>()?;
            let vocab = *shape.last().context("Missing logit shape")? as usize;
            ensure!(vocab > 0 && logits.len() >= vocab, "Empty logits");
            let scores = &logits[logits.len() - vocab..];
            let detect_language = whisper && tokens.len() == 1;
            let best = scores
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    if detect_language {
                        self.languages.contains(&(*i as i64))
                    } else {
                        !self.suppress.contains(i)
                            && (!whisper || *i < 50365)
                            && !(whisper && generated.is_empty() && [220, 50257].contains(i))
                    }
                })
                .max_by(|a, b| a.1.total_cmp(b.1))
                .context("No decoder token")?
                .0 as i64;
            if best == self.eos {
                break;
            }
            for (name, _) in &self.caches {
                // Merged Moonshine/Whisper graphs emit empty cross-attention caches after step zero.
                if !cohere && step > 0 && name.contains(".encoder.") {
                    continue;
                }
                let output_name = name.replacen("past_key_values", "present", 1);
                caches.insert(
                    name.clone(),
                    outputs
                        .remove(&output_name)
                        .with_context(|| format!("Missing {output_name}"))?,
                );
            }
            if detect_language {
                tokens.extend([best, 50360, 50364]);
                next_input = vec![best, 50360, 50364];
            } else {
                tokens.push(best);
                generated.push(best as u32);
                next_input = vec![best];
            }
        }
        Ok(self
            .tokenizer
            .decode(&generated, true)
            .map_err(anyhow::Error::msg)?
            .trim()
            .to_owned())
    }
}
