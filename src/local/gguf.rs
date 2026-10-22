// Session rotation, GPU decoder threading, minimum input padding, the GPU-only streaming probe
// and the stream finalize deadline are adapted from HEX
// (https://github.com/anomalyco/hex, commit 7160492a4d415c24184f57012ce3a6dae1af4e99),
// MIT, Copyright 2026 Christopher Langton. See licenses/hex-MIT.txt.
use anyhow::{Context, Result};
use std::{path::Path, sync::mpsc, time::Duration};
use transcribe_cpp::{
    Backend, CancelToken, DeviceType, ModelOptions, RunOptions, Session, SessionOptions,
    StreamOptions,
};

const MINIMUM_SAMPLES: usize = 16_000 * 3 / 2;
/// Proves the GPU can stream. HEX warms up with 8 s; on a Radeon 780M that loaded 2.5 s
/// slower without a faster first dictation.
const STREAM_WARM_UP: usize = 16_000;
const FINALIZE_DEADLINE: Duration = Duration::from_millis(1500);

pub(super) enum StreamInput {
    Feed(Vec<f32>),
    End,
    Cancel,
}

pub(super) struct Model {
    model: transcribe_cpp::Model,
    session: Option<Session>,
    session_options: SessionOptions,
    options: RunOptions,
    max_samples: Option<usize>,
    streams: bool,
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
            streams: false,
        };
        // Compiles the GPU kernels now instead of during the first dictation.
        model.run(&[])?;
        // A CPU stream re-encodes every overlapping window, several times the offline cost.
        if capabilities.supports_streaming
            && matches!(
                model.model.device()?.device_type,
                DeviceType::Gpu | DeviceType::Igpu
            )
        {
            let mut warm_up = [
                StreamInput::Feed(vec![0.0; STREAM_WARM_UP]),
                StreamInput::End,
            ]
            .into_iter();
            model.streams = model
                .stream(|| warm_up.next().context("warm-up ended"))
                .is_ok();
        }
        Ok(model)
    }

    pub fn streams(&self) -> bool {
        self.streams
    }

    /// Feeds a live stream from `next` until it ends; `None` when it was cancelled.
    pub fn stream(
        &mut self,
        mut next: impl FnMut() -> Result<StreamInput>,
    ) -> Result<Option<String>> {
        let cancel = CancelToken::new();
        let options = self.options.clone();
        self.with_session(|session| {
            session.set_cancel_token(&cancel);
            let mut stream = session.stream(&options, &StreamOptions::default())?;
            let mut failed = None;
            loop {
                match next()? {
                    StreamInput::Feed(samples) => {
                        if failed.is_none() {
                            failed = stream.feed(&samples).err();
                        }
                    }
                    StreamInput::Cancel => return Ok(None),
                    StreamInput::End => {
                        if let Some(error) = failed {
                            return Err(error.into());
                        }
                        let (finished, deadline) = mpsc::channel::<()>();
                        let timer = cancel.clone();
                        std::thread::spawn(move || {
                            if deadline.recv_timeout(FINALIZE_DEADLINE)
                                == Err(mpsc::RecvTimeoutError::Timeout)
                            {
                                timer.cancel();
                            }
                        });
                        let result = stream.finalize();
                        drop(finished);
                        result?;
                        return Ok(Some(stream.text().full.trim().to_owned()));
                    }
                }
            }
        })
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
        let options = self.options.clone();
        self.with_session(|session| Ok(session.run(&padded, &options)?.text.trim().to_owned()))
    }

    fn with_session<T>(&mut self, run: impl FnOnce(&mut Session) -> Result<T>) -> Result<T> {
        let mut session = match self.session.take() {
            Some(session) => session,
            None => self.model.session_with(&self.session_options)?,
        };
        let result = run(&mut session);
        // A session keeps input-sized scratch and stream state until dropped; a fresh one
        // releases a long recording's high-water mark without reloading the weights.
        drop(session);
        self.session = Some(self.model.session_with(&self.session_options)?);
        result
    }
}
