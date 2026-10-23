use anyhow::{Context, Result, ensure};
use ndarray::{Array1, Array2, Array3, ArrayD, Axis, s};
use ort::{inputs, session::Session, value::TensorRef};
use std::path::Path;

pub(super) struct Model {
    preprocessor: Session,
    encoder: Session,
    /// One graph runs prediction and joint per step (istupakov's export).
    decoder_joint: Session,
    vocab: Vec<String>,
    blank: usize,
    state_shape: (usize, usize, usize),
}

impl Model {
    pub fn load(dir: &Path) -> Result<Self> {
        let session = |name: &str| -> Result<Session> {
            Ok(Session::builder()?
                .with_intra_threads(4)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?
                .commit_from_file(dir.join(name))?)
        };
        let decoder_joint = session("decoder_joint-model.int8.onnx")?;
        let shape = decoder_joint
            .inputs()
            .iter()
            .find(|i| i.name() == "input_states_1")
            .and_then(|i| i.dtype().tensor_shape())
            .context("Missing decoder state shape")?;
        ensure!(
            shape.len() == 3 && shape[0] > 0 && shape[2] > 0,
            "Invalid decoder state shape"
        );
        let state_shape = (shape[0] as usize, 1, shape[2] as usize);
        let mut vocab = Vec::new();
        for line in std::fs::read_to_string(dir.join("vocab.txt"))?.lines() {
            let (token, id) = line.rsplit_once(' ').context("Invalid vocabulary")?;
            let id: usize = id.parse()?;
            vocab.resize(vocab.len().max(id + 1), String::new());
            vocab[id] = token.replace('▁', " ");
        }
        let blank = vocab
            .iter()
            .position(|t| t == "<blk>")
            .context("Missing blank token")?;
        Ok(Self {
            preprocessor: session("nemo128.onnx")?,
            encoder: session("encoder-model.int8.onnx")?,
            decoder_joint,
            vocab,
            blank,
            state_shape,
        })
    }

    /// Token and duration logits for one encoder frame, plus the states after `last`.
    fn step(
        &mut self,
        frame: &Array3<f32>,
        last: i32,
        state1: &Array3<f32>,
        state2: &Array3<f32>,
    ) -> Result<(Vec<f32>, ArrayD<f32>, ArrayD<f32>)> {
        let target = Array2::from_shape_vec((1, 1), vec![last])?;
        let target_length = Array1::from_vec(vec![1_i32]);
        let out = self.decoder_joint.run(inputs![
            "encoder_outputs" => TensorRef::from_array_view(frame.view())?,
            "targets" => TensorRef::from_array_view(target.view())?,
            "target_length" => TensorRef::from_array_view(target_length.view())?,
            "input_states_1" => TensorRef::from_array_view(state1.view())?,
            "input_states_2" => TensorRef::from_array_view(state2.view())?,
        ])?;
        Ok((
            out["outputs"].try_extract_tensor::<f32>()?.1.to_vec(),
            out["output_states_1"]
                .try_extract_array::<f32>()?
                .to_owned(),
            out["output_states_2"]
                .try_extract_array::<f32>()?
                .to_owned(),
        ))
    }

    pub fn transcribe(&mut self, samples: Vec<f32>) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }
        let lengths = Array1::from_vec(vec![samples.len() as i64]);
        let wave = Array2::from_shape_vec((1, samples.len()), samples)?;
        let features = self.preprocessor.run(inputs![
            "waveforms" => TensorRef::from_array_view(wave.view())?,
            "waveforms_lens" => TensorRef::from_array_view(lengths.view())?,
        ])?;
        let signal = features["features"].try_extract_array::<f32>()?;
        let lengths = features["features_lens"].try_extract_array::<i64>()?;
        let encoded = self.encoder.run(inputs![
            "audio_signal" => TensorRef::from_array_view(signal)?,
            "length" => TensorRef::from_array_view(lengths)?,
        ])?;
        let frames = encoded["outputs"].try_extract_array::<f32>()?.to_owned();
        let length = encoded["encoded_lengths"].try_extract_tensor::<i64>()?.1[0] as usize;
        drop(encoded);
        drop(features);
        let mut state1 = Array3::<f32>::zeros(self.state_shape);
        let mut state2 = Array3::<f32>::zeros(self.state_shape);
        let mut last = self.blank as i32;
        let mut text = String::new();
        let mut t = 0;
        let mut emitted = 0;
        while t < length {
            let frame = frames
                .slice(s![0, .., t])
                .to_owned()
                .insert_axis(Axis(0))
                .insert_axis(Axis(2));
            let (logits, next1, next2) = self.step(&frame, last, &state1, &state2)?;
            ensure!(logits.len() > self.vocab.len(), "Missing TDT durations");
            let argmax = |values: &[f32]| {
                values
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            };
            let token = argmax(&logits[..self.vocab.len()]);
            let duration = argmax(&logits[self.vocab.len()..]);
            if token != self.blank {
                text.push_str(&self.vocab[token]);
                last = token as i32;
                state1 = next1.into_dimensionality()?;
                state2 = next2.into_dimensionality()?;
                emitted += 1;
            }
            if duration > 0 {
                t += duration;
                emitted = 0;
            } else if token == self.blank || emitted >= 10 {
                t += 1;
                emitted = 0;
            }
        }
        Ok(text.trim().to_owned())
    }
}
