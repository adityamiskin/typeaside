use anyhow::{Context, Result};
use std::path::Path;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub(super) struct Model {
    state: WhisperState,
    language: Option<String>,
}
impl Model {
    pub fn load(path: &Path, language: Option<&str>) -> Result<Self> {
        // The worker answers over stdout, so whisper.cpp must not log there.
        whisper_rs::install_logging_hooks();
        let path = path.to_str().context("Whisper model path is not UTF-8")?;
        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())?;
        Ok(Self {
            state: context.create_state()?,
            language: language.map(str::to_owned),
        })
    }
    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(self.language.as_deref().unwrap_or("auto")));
        params.set_n_threads(
            std::thread::available_parallelism().map_or(4, |n| n.get().min(8)) as i32,
        );
        params.set_no_timestamps(true);
        params.set_suppress_nst(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        self.state.full(params, samples)?;
        let mut text = String::new();
        for segment in self.state.as_iter() {
            text.push_str(&segment.to_str_lossy()?);
        }
        Ok(text.trim().to_owned())
    }
}
