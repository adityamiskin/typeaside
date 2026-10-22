// Session rotation, GPU decoder threading and minimum input padding are adapted from HEX
// (https://github.com/anomalyco/hex, commit 7160492a4d415c24184f57012ce3a6dae1af4e99),
// MIT, Copyright 2026 Christopher Langton. See licenses/hex-MIT.txt.
use anyhow::{Context, Result};
use std::path::Path;
use transcribe_cpp::{Backend, DeviceType, ModelOptions, RunOptions, Session, SessionOptions};

const MINIMUM_SAMPLES: usize = 16_000 * 3 / 2;

pub(super) struct Model {
    model: transcribe_cpp::Model,
    session: Option<Session>,
    session_options: SessionOptions,
    options: RunOptions,
    max_samples: Option<usize>,
}

impl Model {
    /// Falls back to the CPU when the GPU cannot load or run the model.
    pub fn load(path: &Path, language: Option<&str>) -> Result<Self> {
        // ggml logs every buffer and kernel to stderr, which the worker shares with the app.
        transcribe_cpp::disable_logging();
        // A discrete laptop GPU sleeps between dictations and takes about a second to wake.
        let preferred: &[DeviceType] = match std::env::var("TYPEASIDE_LOCAL_DEVICE").as_deref() {
            Ok("cpu") => &[],
            Ok("gpu") => &[DeviceType::Gpu, DeviceType::Igpu],
            _ => &[DeviceType::Igpu, DeviceType::Gpu],
        };
        let devices = transcribe_cpp::devices();
        let cpu = ModelOptions {
            backend: Backend::Cpu,
            device: None,
        };
        let options = preferred
            .iter()
            .find_map(|kind| devices.iter().find(|d| d.device_type == *kind))
            .map_or_else(
                || cpu.clone(),
                |device| ModelOptions {
                    backend: if device.kind == "metal" {
                        Backend::Metal
                    } else {
                        Backend::Vulkan
                    },
                    device: Some(device.clone()),
                },
            );
        match Self::open(path, language, &options) {
            Ok(model) => Ok(model),
            Err(error) if options.backend != Backend::Cpu => {
                eprintln!("typeaside local: GPU unavailable, using CPU: {error:#}");
                Self::open(path, language, &cpu)
            }
            Err(error) => Err(error),
        }
    }

    fn open(path: &Path, language: Option<&str>, options: &ModelOptions) -> Result<Self> {
        let model = transcribe_cpp::Model::load_with(path, options)
            .with_context(|| format!("Could not load {}", path.display()))?;
        // Only the per-token decoder runs on the CPU; ggml's spin-waiting worker pool stalls it.
        let session_options = match model.device()?.device_type {
            DeviceType::Gpu | DeviceType::Igpu => SessionOptions {
                n_threads: 1,
                ..Default::default()
            },
            _ => SessionOptions::default(),
        };
        let capabilities = model.capabilities();
        let language = language
            .filter(|language| capabilities.languages.iter().any(|l| l == language))
            .map(str::to_owned);
        let mut model = Self {
            session: Some(model.session_with(&session_options)?),
            model,
            session_options,
            options: RunOptions {
                language,
                timestamps: transcribe_cpp::TimestampKind::Segment,
                ..Default::default()
            },
            max_samples: usize::try_from(capabilities.max_audio_ms)
                .ok()
                .filter(|ms| *ms > 0)
                .map(|ms| ms * 16),
        };
        // Compiles the GPU kernels now instead of during the first dictation.
        model.run(&[])?;
        Ok(model)
    }

    pub fn device(&self) -> String {
        self.model.device().map_or_else(
            |_| self.model.backend(),
            |d| format!("{} {}", d.kind, d.description),
        )
    }

    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        let mut text = Vec::new();
        for chunk in samples.chunks(self.max_samples.unwrap_or(usize::MAX).max(1)) {
            let chunk = self.run(chunk)?;
            if !chunk.is_empty() {
                text.push(chunk);
            }
        }
        Ok(text.join(" "))
    }

    fn run(&mut self, samples: &[f32]) -> Result<String> {
        let mut padded = samples.to_vec();
        padded.resize(padded.len().max(MINIMUM_SAMPLES), 0.0);
        let mut session = match self.session.take() {
            Some(session) => session,
            None => self.model.session_with(&self.session_options)?,
        };
        let result = session.run(&padded, &self.options);
        // A session keeps input-sized scratch until dropped; a fresh one releases a long
        // recording's high-water mark without reloading the weights.
        drop(session);
        self.session = Some(self.model.session_with(&self.session_options)?);
        Ok(result?.text.trim().to_owned())
    }
}
