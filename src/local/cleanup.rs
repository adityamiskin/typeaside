use super::{AVAILABLE, assets::S1_MINI, download_assets, inventory_assets};
use crate::config::{AppCategory, AppConfig, Tone};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{CString, c_char, c_void},
    fs,
    path::PathBuf,
    ptr::NonNull,
    sync::atomic::AtomicBool,
};
use typeaside_llama_cpp_sys::*;

pub struct CleanupModel;
impl CleanupModel {
    pub const LABEL: &str = "S1-mini by Superwhisper";
    pub fn size_mb() -> u64 {
        S1_MINI
            .iter()
            .map(|a| a.size)
            .sum::<u64>()
            .div_ceil(1_000_000)
    }
    pub fn directory() -> Result<PathBuf> {
        Ok(super::models_directory()?.join("s1-mini"))
    }
    pub fn installed() -> bool {
        AVAILABLE
            && Self::directory().is_ok_and(|dir| {
                fs::read_to_string(dir.join("installed"))
                    .ok()
                    .zip(inventory_assets(&dir, S1_MINI.iter().map(|a| a.name)).ok())
                    .is_some_and(|(saved, current)| saved == current)
            })
    }
    pub fn download(cancel: &AtomicBool, progress: impl Fn(u8)) -> Result<()> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(download_assets(
                &Self::directory()?,
                &S1_MINI,
                cancel,
                progress,
            ))
    }
    pub fn remove() -> Result<()> {
        super::unload();
        let dir = Self::directory()?;
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Styling {
    Casual,
    SemiCasual,
    Formal,
}
impl Styling {
    fn key(self) -> &'static str {
        match self {
            Self::Casual => "casual",
            Self::SemiCasual => "semi-casual",
            Self::Formal => "formal",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CleanupSettings {
    styling: Styling,
    email: bool,
}
impl CleanupSettings {
    pub fn for_app(config: &AppConfig, app_id: Option<&str>) -> Option<Self> {
        let english = config
            .language
            .as_deref()
            .and_then(|s| s.split(['-', '_']).next())
            .is_some_and(|s| s.eq_ignore_ascii_case("en"));
        let category = AppCategory::of(app_id);
        // Auto can transcribe any language; require explicit English for an English-only model.
        (config.editing() && english && category != AppCategory::Code && CleanupModel::installed())
            .then(|| Self {
                styling: match config.styles.get(category) {
                    Tone::Formal => Styling::Formal,
                    Tone::Casual => Styling::SemiCasual,
                    Tone::VeryCasual => Styling::Casual,
                },
                email: category == AppCategory::Email,
            })
    }
}

const SYSTEM: &str = "You are a text normalizer for speech-to-text transcripts. The input begins with a control line specifying the styling, structure, and context settings; clean the transcript to match those settings and output only the cleaned text.";
const CONTEXT: usize = 1024;
const MAX_OUTPUT: usize = 512;

struct Weights(NonNull<llama_model>);
impl Drop for Weights {
    fn drop(&mut self) {
        unsafe { llama_model_free(self.0.as_ptr()) };
    }
}
struct ContextHandle(NonNull<llama_context>);
impl Drop for ContextHandle {
    fn drop(&mut self) {
        unsafe { llama_free(self.0.as_ptr()) };
    }
}
struct Sampler(NonNull<llama_sampler>);
impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe { llama_sampler_free(self.0.as_ptr()) };
    }
}

unsafe extern "C" fn discard_log(_: ggml_log_level, _: *const c_char, _: *mut c_void) {}

pub(super) struct Model {
    // Context must be freed before its weights.
    context: ContextHandle,
    weights: Weights,
}
impl Model {
    pub fn load() -> Result<Self> {
        ensure!(
            CleanupModel::installed(),
            "Download S1-mini by Superwhisper in Settings first"
        );
        let path = CString::new(
            CleanupModel::directory()?
                .join(S1_MINI[0].name)
                .to_str()
                .context("Invalid model path")?,
        )?;
        // Speech initializes the shared ggml registry before cleanup can be requested.
        unsafe {
            llama_log_set(Some(discard_log), std::ptr::null_mut());
            llama_backend_init();
            let mut params = llama_model_default_params();
            params.n_gpu_layers = if std::env::var("TYPEASIDE_LOCAL_DEVICE").as_deref() == Ok("cpu")
            {
                0
            } else {
                99
            };
            let weights = Weights(
                NonNull::new(llama_model_load_from_file(path.as_ptr(), params))
                    .context("Load S1-mini by Superwhisper")?,
            );
            let mut params = llama_context_default_params();
            params.n_ctx = CONTEXT as u32;
            params.n_batch = CONTEXT as u32;
            params.n_ubatch = 512;
            params.n_threads = 4;
            params.n_threads_batch = 4;
            let context = ContextHandle(
                NonNull::new(llama_init_from_model(weights.0.as_ptr(), params))
                    .context("Create S1-mini by Superwhisper context")?,
            );
            Ok(Self { context, weights })
        }
    }

    pub fn clean(&mut self, raw: &str, settings: CleanupSettings) -> Result<String> {
        let styling = settings.styling.key();
        let context = if settings.email { "email" } else { "general" };
        let prompt = format!(
            "<|im_start|>system\n{SYSTEM}<|im_end|>\n<|im_start|>user\n[Styling: {styling}] [Structure: lists] [Context: {context}]\n{raw}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        unsafe {
            let vocab = llama_model_get_vocab(self.weights.0.as_ptr());
            let ctx = self.context.0.as_ptr();
            let mut tokens = vec![0; CONTEXT];
            let count = llama_tokenize(
                vocab,
                prompt.as_ptr().cast(),
                prompt.len().try_into()?,
                tokens.as_mut_ptr(),
                CONTEXT as i32,
                false,
                true,
            );
            ensure!(
                count > 0 && count as usize + MAX_OUTPUT <= CONTEXT,
                "Transcript exceeds cleanup context budget"
            );
            tokens.truncate(count as usize);
            llama_memory_clear(llama_get_memory(ctx), true);
            let sampler = Sampler(
                NonNull::new(llama_sampler_init_greedy()).context("Create greedy sampler")?,
            );
            let mut output = Vec::new();
            for _ in 0..MAX_OUTPUT {
                let batch = llama_batch_get_one(tokens.as_mut_ptr(), tokens.len().try_into()?);
                let status = llama_decode(ctx, batch);
                ensure!(
                    status == 0,
                    "S1-mini by Superwhisper decode failed: {status}"
                );
                let token = llama_sampler_sample(sampler.0.as_ptr(), ctx, -1);
                if llama_vocab_is_eog(vocab, token) {
                    return Ok(String::from_utf8(output)?);
                }
                let mut piece = vec![0_u8; 128];
                let mut n = llama_token_to_piece(
                    vocab,
                    token,
                    piece.as_mut_ptr().cast(),
                    piece.len().try_into()?,
                    0,
                    false,
                );
                if n < 0 {
                    piece.resize(
                        n.checked_neg().context("Token size overflow")?.try_into()?,
                        0,
                    );
                    n = llama_token_to_piece(
                        vocab,
                        token,
                        piece.as_mut_ptr().cast(),
                        piece.len().try_into()?,
                        0,
                        false,
                    );
                }
                ensure!(n >= 0, "Token detokenization failed");
                output.extend_from_slice(&piece[..n as usize]);
                tokens.clear();
                tokens.push(token);
            }
            anyhow::bail!("Cleanup output reached its token limit")
        }
    }
}
