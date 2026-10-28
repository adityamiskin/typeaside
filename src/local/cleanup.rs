use super::{AVAILABLE, ModelId, assets::S1_MINI, download_assets, inventory_assets};
use crate::config::{AppCategory, AppConfig, SpeechSource, Tone};
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
    SemiFormal,
    Formal,
}
impl Styling {
    fn key(self) -> &'static str {
        match self {
            Self::Casual => "casual",
            Self::SemiFormal => "semi-formal",
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
    /// Whether a dictation into this app can reach S1-mini, before its transcript is known.
    pub fn may_apply(config: &AppConfig, app_id: Option<&str>) -> bool {
        config.editing()
            && config.language.as_deref().is_none_or(english)
            && AppCategory::of(app_id) != AppCategory::Code
            && CleanupModel::installed()
    }
    pub fn for_app(config: &AppConfig, app_id: Option<&str>, text: &str) -> Option<Self> {
        // S1-mini is English-only. Parakeet v3 and API speech report no detected language, so
        // Auto checks the transcript itself; Cohere's Auto and the English-only models
        // always transcribe English.
        let english = config.language.is_some()
            || (config.transcription == SpeechSource::Local
                && matches!(
                    config.local_model,
                    ModelId::Cohere | ModelId::ParakeetUnifiedEn | ModelId::NemotronEn
                ))
            || whatlang::detect_lang(text) == Some(whatlang::Lang::Eng);
        let category = AppCategory::of(app_id);
        (english && Self::may_apply(config, app_id)).then(|| Self {
            styling: match config.styles.get(category) {
                Tone::Formal => Styling::Formal,
                Tone::Casual => Styling::SemiFormal,
                Tone::VeryCasual => Styling::Casual,
            },
            email: category == AppCategory::Email,
        })
    }
}

fn english(language: &str) -> bool {
    language
        .split(['-', '_'])
        .next()
        .is_some_and(|s| s.eq_ignore_ascii_case("en"))
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
    prompt: Vec<llama_token>,
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
            Ok(Self {
                context,
                weights,
                prompt: Vec::new(),
            })
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
            let previous = std::mem::take(&mut self.prompt);
            // Keep matching prompt KV, but decode the last token again to refresh logits.
            let shared = previous
                .iter()
                .zip(&tokens)
                .take_while(|(a, b)| a == b)
                .count()
                .min(tokens.len() - 1);
            let memory = llama_get_memory(ctx);
            let shared = if shared > 0 && llama_memory_seq_rm(memory, 0, shared as i32, -1) {
                shared
            } else {
                llama_memory_clear(memory, true);
                0
            };
            let prompt = tokens.clone();
            tokens.drain(..shared);
            let sampler = Sampler(
                NonNull::new(llama_sampler_init_greedy()).context("Create greedy sampler")?,
            );
            // English averages about 4 bytes per token and the output guard rejects anything
            // past 1.3x the input's length, so longer generation is wasted latency.
            let limit = (raw.len() / 2 + 32).min(MAX_OUTPUT);
            let input_chars = raw.chars().count().max(1) as f64;
            let mut output = Vec::new();
            for _ in 0..limit {
                let batch = llama_batch_get_one(tokens.as_mut_ptr(), tokens.len().try_into()?);
                let status = llama_decode(ctx, batch);
                ensure!(
                    status == 0,
                    "S1-mini by Superwhisper decode failed: {status}"
                );
                if self.prompt.is_empty() {
                    self.prompt.clone_from(&prompt);
                }
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
                // An immutable prefix past the caller's length guard cannot become valid later.
                ensure!(
                    !std::str::from_utf8(&output).is_ok_and(|text| {
                        text.trim().chars().count() as f64 / input_chars
                            > crate::cleanup::MAX_LENGTH_RATIO
                    }),
                    "Cleanup output exceeds length guard"
                );
                tokens.clear();
                tokens.push(token);
            }
            anyhow::bail!("Cleanup output reached its token limit")
        }
    }
}
