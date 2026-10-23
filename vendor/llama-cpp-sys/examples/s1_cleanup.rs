use std::{error::Error, ffi::CString, ptr::NonNull, time::Instant};
use typeaside_llama_cpp_sys::*;

include!("cases.rs.inc");

const SYSTEM: &str = "You are a text normalizer for speech-to-text transcripts. The input begins with a control line specifying the styling, structure, and context settings; clean the transcript to match those settings and output only the cleaned text.";
const CONTEXT: usize = 1024;
const MAX_OUTPUT: usize = 512;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Model(NonNull<llama_model>);
impl Drop for Model {
    fn drop(&mut self) {
        unsafe { llama_model_free(self.0.as_ptr()) };
    }
}
struct Context(NonNull<llama_context>);
impl Drop for Context {
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

fn clean(model: &Model, ctx: &mut Context, settings: (&str, &str, &str, &str)) -> Result<String> {
    let (styling, structure, context, raw) = settings;
    let prompt = format!(
        "<|im_start|>system\n{SYSTEM}<|im_end|>\n<|im_start|>user\n[Styling: {styling}] [Structure: {structure}] [Context: {context}]\n{raw}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
    );
    unsafe {
        let vocab = llama_model_get_vocab(model.0.as_ptr());
        let mut tokens = vec![0; CONTEXT];
        let count = llama_tokenize(
            vocab,
            prompt.as_ptr().cast(),
            prompt.len().try_into()?,
            tokens.as_mut_ptr(),
            tokens.len().try_into()?,
            false,
            true,
        );
        if count <= 0 || count as usize + MAX_OUTPUT > CONTEXT {
            return Err(format!("prompt exceeds context budget: {count}").into());
        }
        tokens.truncate(count as usize);
        llama_memory_clear(llama_get_memory(ctx.0.as_ptr()), true);
        let sampler = Sampler(NonNull::new(llama_sampler_init_greedy()).ok_or("create sampler")?);
        let mut output = Vec::new();
        for _ in 0..MAX_OUTPUT {
            let batch = llama_batch_get_one(tokens.as_mut_ptr(), tokens.len().try_into()?);
            let status = llama_decode(ctx.0.as_ptr(), batch);
            if status != 0 {
                return Err(format!("llama_decode failed: {status}").into());
            }
            let token = llama_sampler_sample(sampler.0.as_ptr(), ctx.0.as_ptr(), -1);
            if llama_vocab_is_eog(vocab, token) {
                return Ok(String::from_utf8(output)?);
            }
            let mut piece = vec![0_u8; 128];
            let mut n =
                llama_token_to_piece(vocab, token, piece.as_mut_ptr().cast(), 128, 0, false);
            if n < 0 {
                piece.resize(
                    n.checked_neg().ok_or("piece length overflow")?.try_into()?,
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
            if n < 0 {
                return Err("token detokenization failed".into());
            }
            output.extend_from_slice(&piece[..n as usize]);
            tokens.clear();
            tokens.push(token);
        }
        Err("generation reached 512 tokens without EOS".into())
    }
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 2 && args[1] == "--help" {
        println!(
            "S1-mini by Superwhisper\nUsage: s1_cleanup <s1-mini-q4_k_m.gguf> <silero-vad-v6.2-F32.gguf> [threads=4] [gpu-layers=99]"
        );
        return Ok(());
    }
    if !(3..=5).contains(&args.len()) {
        return Err("expected S1-mini and Silero VAD model paths; see --help".into());
    }
    let threads = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(4);
    let gpu_layers = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(99);
    transcribe_cpp::init_backends_default()?;
    let vad_model = transcribe_cpp::Model::load(&args[2])?;
    let mut vad = vad_model.vad_session()?;
    let silence = vec![0.0; 16000];
    let before = vad.run(&silence, &Default::default())?;
    println!(
        "transcribe.cpp {}: Silero VAD loaded, {} frames, {} segments",
        transcribe_cpp::version(),
        before.probs.len(),
        before.segments.len()
    );
    unsafe {
        llama_backend_init();
        let path = CString::new(args[1].as_str())?;
        let mut params = llama_model_default_params();
        params.n_gpu_layers = gpu_layers;
        let model = Model(
            NonNull::new(llama_model_load_from_file(path.as_ptr(), params))
                .ok_or("load S1-mini by Superwhisper")?,
        );
        let mut params = llama_context_default_params();
        params.n_ctx = CONTEXT as u32;
        params.n_batch = CONTEXT as u32;
        params.n_ubatch = 512;
        params.n_threads = threads;
        params.n_threads_batch = threads;
        let mut ctx = Context(
            NonNull::new(llama_init_from_model(model.0.as_ptr(), params))
                .ok_or("create context")?,
        );
        println!(
            "S1-mini by Superwhisper; context={CONTEXT}; greedy (temperature 0); threads={threads}"
        );
        for (index, &case) in CASES.iter().enumerate() {
            let start = Instant::now();
            let output = clean(&model, &mut ctx, case)?;
            println!(
                "--- case {} [{}/{}/{}] {:.1} ms\nIN : {}\nOUT: {}\n",
                index + 1,
                case.0,
                case.1,
                case.2,
                start.elapsed().as_secs_f64() * 1000.0,
                case.3,
                output
            );
        }
        let after = vad.run(&silence, &Default::default())?;
        assert_eq!(before.probs, after.probs);
        assert_eq!(before.segments.len(), after.segments.len());
        println!("Silero VAD rerun while S1-mini is resident: identical probabilities");
    }
    Ok(())
}
