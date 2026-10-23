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
