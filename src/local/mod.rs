mod assets;
mod catalog;
mod cohere;
mod features;
mod inference;
mod whisper;
pub use catalog::ModelId;

use anyhow::{Context, Result, ensure};
use assets::RUNTIME;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub const AVAILABLE: bool = RUNTIME.is_some();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    NotInstalled,
    Downloading(u8),
    Installed,
    Failed,
}

impl Status {
    pub fn current(model: ModelId) -> Self {
        if model.installed() {
            Self::Installed
        } else {
            Self::NotInstalled
        }
    }
}

fn models_directory() -> Result<PathBuf> {
    Ok(crate::paths::AppPaths::discover()?.data_dir.join("models"))
}
fn runtime_directory() -> Result<PathBuf> {
    Ok(models_directory()?.join("runtime"))
}
fn runtime_library() -> Result<PathBuf> {
    let shared = runtime_directory()?.join(library_name());
    if shared.exists() {
        return Ok(shared);
    }
    // Existing v3 installations keep their runtime until a new download shares it.
    Ok(ModelId::ParakeetV3.directory()?.join(library_name()))
}

fn library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "onnxruntime.dll"
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.1.24.4.dylib"
    } else {
        "libonnxruntime.so.1.24.4"
    }
}

fn inventory(model: ModelId, dir: &Path) -> Result<String> {
    let mut entries = Vec::new();
    for name in model
        .assets()
        .iter()
        .map(|asset| asset.name)
        .chain(std::iter::once(library_name()))
    {
        let path = if name == library_name() {
            runtime_library()?
        } else {
            dir.join(name)
        };
        let metadata = fs::metadata(path)?;
        entries.push((
            name,
            metadata.len(),
            metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos(),
        ));
    }
    Ok(serde_json::to_string(&entries)?)
}

pub fn installed() -> bool {
    ModelId::ALL.into_iter().any(ModelId::installed)
}

fn verify(path: &Path, expected: &str, cancel: &AtomicBool) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut chunk = [0; 65536];
    loop {
        ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        hash.update(&chunk[..n]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == expected,
        "Download checksum mismatch"
    );
    Ok(())
}

pub fn download(model: ModelId, cancel: &AtomicBool, progress: impl Fn(u8)) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(download_async(model, cancel, progress))
}

async fn cancelled(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn download_async(model: ModelId, cancel: &AtomicBool, progress: impl Fn(u8)) -> Result<()> {
    let Some(runtime) = &RUNTIME else {
        anyhow::bail!("On-device transcription is not available on this platform");
    };
    let dir = model.directory()?;
    fs::create_dir_all(&dir)?;
    let _ = fs::remove_file(dir.join("installed"));
    let total: u64 = model.assets().iter().chain(&RUNTIME).map(|a| a.size).sum();
    let mut completed = 0;
    let mut last = std::time::Instant::now() - Duration::from_secs(1);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(1800))
        .build()?;
    for asset in model.assets().iter().chain(&RUNTIME) {
        let target = if asset.name == runtime.name {
            runtime_directory()?
        } else {
            dir.clone()
        };
        fs::create_dir_all(&target)?;
        ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
        let final_path = target.join(asset.name);
        if asset.name == runtime.name && runtime_library()?.exists() {
            let shared = runtime_directory()?.join(library_name());
            if !shared.exists() {
                let legacy = runtime_library()?;
                fs::copy(&legacy, &shared)?;
                // Preserve the inventory recorded by single-model installations.
                File::options().write(true).open(&shared)?.set_times(
                    fs::FileTimes::new().set_modified(fs::metadata(legacy)?.modified()?),
                )?;
            }
            completed += asset.size;
            continue;
        }
        if verify(&final_path, asset.sha256, cancel).is_ok() {
            completed += asset.size;
            continue;
        }
        ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
        let part = target.join(format!("{}.part", asset.name));
        let mut output = File::create(&part)?;
        let mut response = tokio::select! {
            result = client.get(asset.url).send() => result?.error_for_status()?,
            _ = cancelled(cancel) => anyhow::bail!("Download cancelled"),
        };
        loop {
            let chunk = tokio::select! {
                result = tokio::time::timeout(Duration::from_secs(15), response.chunk()) => result.context("Download stalled")??,
                _ = cancelled(cancel) => anyhow::bail!("Download cancelled"),
            };
            let Some(chunk) = chunk else {
                break;
            };
            output.write_all(&chunk)?;
            completed += chunk.len() as u64;
            if last.elapsed() >= Duration::from_millis(250) {
                progress(((completed * 100 / total).min(99)) as u8);
                last = std::time::Instant::now();
            }
        }
        output.sync_all()?;
        drop(output);
        verify(&part, asset.sha256, cancel)?;
        fs::rename(part, final_path)?;
    }
    let runtime_dir = runtime_directory()?;
    let archive = runtime_dir.join(runtime.name);
    if archive.exists() {
        if runtime.name.ends_with(".zip") {
            let mut zip = zip::ZipArchive::new(File::open(&archive)?)?;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i)?;
                if Path::new(entry.name())
                    .file_name()
                    .is_some_and(|n| n == library_name())
                {
                    let mut out =
                        File::create(runtime_dir.join(format!("{}.part", library_name())))?;
                    std::io::copy(&mut entry, &mut out)?;
                }
            }
        } else {
            let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(File::open(&archive)?));
            for entry in tar.entries()? {
                let mut entry = entry?;
                if entry.header().entry_type().is_file()
                    && entry
                        .path()?
                        .file_name()
                        .is_some_and(|n| n == library_name())
                {
                    entry.unpack(runtime_dir.join(format!("{}.part", library_name())))?;
                }
            }
        }
        ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
        fs::rename(
            runtime_dir.join(format!("{}.part", library_name())),
            runtime_dir.join(library_name()),
        )?;
        fs::remove_file(archive)?;
    }
    fs::write(dir.join("installed.part"), inventory(model, &dir)?)?;
    fs::rename(dir.join("installed.part"), dir.join("installed"))?;
    progress(100);
    Ok(())
}

struct Worker {
    child: Child,
    model: ModelId,
    language: Option<String>,
    requests: Option<mpsc::Sender<Vec<f32>>>,
    responses: mpsc::Receiver<Result<String, String>>,
    io: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.requests.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(io) = self.io.take() {
            let _ = io.join();
        }
    }
}
impl Worker {
    fn start(model: ModelId, language: Option<String>) -> Result<Self> {
        let started = std::time::Instant::now();
        let executable = std::env::current_exe()?;
        let mut child = Command::new(executable)
            .arg("--local-worker")
            .args(["--local-model", model.key()])
            .args(
                language
                    .iter()
                    .flat_map(|lang| ["--local-language", lang.as_str()]),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let (requests, input) = mpsc::channel::<Vec<f32>>();
        let (output, responses) = mpsc::channel();
        let mut stdin = child.stdin.take().context("worker stdin")?;
        let mut stdout = BufReader::new(child.stdout.take().context("worker stdout")?);
        let io = std::thread::spawn(move || {
            let response = |stdout: &mut BufReader<std::process::ChildStdout>| -> Result<String> {
                let mut line = String::new();
                ensure!(
                    stdout.read_line(&mut line)? > 0,
                    "On-device transcription process exited"
                );
                let response: std::result::Result<String, String> = serde_json::from_str(&line)?;
                response.map_err(anyhow::Error::msg)
            };
            let ready = response(&mut stdout).map_err(|e| e.to_string());
            let failed = ready.is_err();
            if output.send(ready).is_err() || failed {
                return;
            }
            for audio in input {
                let result = (|| -> Result<String> {
                    stdin.write_all(&(audio.len() as u64).to_le_bytes())?;
                    let bytes: Vec<_> = audio
                        .iter()
                        .flat_map(|sample| sample.to_le_bytes())
                        .collect();
                    stdin.write_all(&bytes)?;
                    stdin.flush()?;
                    response(&mut stdout)
                })()
                .map_err(|e| e.to_string());
                if output.send(result).is_err() {
                    break;
                }
            }
        });
        let worker = Self {
            child,
            model,
            language,
            requests: Some(requests),
            responses,
            io: Some(io),
        };
        worker.response(Duration::from_secs(120))?;
        if std::env::var_os("TYPEASIDE_TRACE").is_some_and(|value| value == "1") {
            eprintln!(
                "typeaside local: loaded {} in {} ms",
                model.key(),
                started.elapsed().as_millis()
            );
        }
        Ok(worker)
    }
    fn response(&self, timeout: Duration) -> Result<String> {
        self.responses
            .recv_timeout(timeout)
            .context("On-device transcription worker timed out or disconnected")?
            .map_err(anyhow::Error::msg)
    }
    fn transcribe(&mut self, audio: &[f32]) -> Result<String> {
        self.requests
            .as_ref()
            .context("worker requests unavailable")?
            .send(audio.to_vec())?;
        self.response(Duration::from_secs(
            30 + (audio.len() as u64).div_ceil(16_000) * 2,
        ))
    }
}

type Reply = mpsc::SyncSender<std::result::Result<String, String>>;
enum Request {
    Idle(Option<Duration>),
    Warm(ModelId, Option<String>),
    Transcribe(ModelId, Option<String>, Vec<f32>, Reply),
    Unload(Option<mpsc::SyncSender<()>>),
}
fn sender() -> &'static mpsc::Sender<Request> {
    static SENDER: OnceLock<mpsc::Sender<Request>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut worker: Option<Worker> = None;
            let mut idle = crate::config::LocalUnload::default().after();
            loop {
                let request = match idle.filter(|_| worker.is_some()) {
                    Some(idle) => rx.recv_timeout(idle),
                    None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
                };
                match request {
                    Ok(Request::Idle(after)) => idle = after,
                    Ok(Request::Warm(model, language)) => {
                        if worker
                            .as_ref()
                            .is_none_or(|w| w.model != model || w.language != language)
                        {
                            drop(worker.take());
                            worker = Worker::start(model, language).ok();
                        }
                    }
                    Ok(Request::Transcribe(model, language, audio, reply)) => {
                        let result = (|| {
                            if worker
                                .as_ref()
                                .is_none_or(|w| w.model != model || w.language != language)
                            {
                                drop(worker.take());
                                worker = Some(Worker::start(model, language)?);
                            }
                            worker
                                .as_mut()
                                .context("missing worker")?
                                .transcribe(&audio)
                        })();
                        if result.is_err() {
                            worker = None;
                        }
                        let _ = reply.send(result.map_err(|e: anyhow::Error| e.to_string()));
                    }
                    Ok(Request::Unload(reply)) => {
                        worker = None;
                        if let Some(reply) = reply {
                            let _ = reply.send(());
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        worker = None;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        tx
    })
}

pub fn prewarm(config: &crate::config::AppConfig) {
    if let Some(model) = config.local_model.available() {
        let _ = sender().send(Request::Warm(model, config.language.clone()));
    }
}
pub fn apply(config: &crate::config::AppConfig) {
    let _ = sender().send(Request::Idle(config.local_unload.after()));
    if config.transcription == crate::config::Source::Local {
        prewarm(config);
    } else {
        let _ = sender().send(Request::Unload(None));
    }
}
pub fn unload() {
    let (tx, rx) = mpsc::sync_channel(1);
    if sender().send(Request::Unload(Some(tx))).is_ok() {
        let _ = rx.recv();
    }
}
pub fn remove(model: ModelId) -> Result<()> {
    unload();
    let dir = model.directory()?;
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}
pub fn transcribe(config: &crate::config::AppConfig, audio: &[f32]) -> Result<String> {
    let model = config
        .local_model
        .available()
        .context("Download an on-device model in Settings first")?;
    transcribe_model(model, config.language.clone(), audio)
}
pub fn transcribe_model(model: ModelId, language: Option<String>, audio: &[f32]) -> Result<String> {
    ensure!(model.installed(), "Model is not installed");
    let (tx, rx) = mpsc::sync_channel(1);
    sender().send(Request::Transcribe(model, language, audio.to_vec(), tx))?;
    rx.recv()?.map_err(anyhow::Error::msg)
}

pub fn worker_main() -> Result<()> {
    let option = |flag: &str| std::env::args().skip_while(|arg| arg != flag).nth(1);
    let model: ModelId = option("--local-model")
        .map(|key| key.parse())
        .transpose()?
        .unwrap_or_default();
    let language = option("--local-language");
    let dir = model.directory()?;
    ensure!(
        model.installed(),
        "On-device model is missing or has changed; download it again"
    );
    ort::init_from(runtime_library()?)?.commit();
    let mut model = match model {
        ModelId::ParakeetV3 => Engine::Parakeet(Box::new(inference::Model::load(
            &dir,
            inference::Layout::Joint,
            "encoder-model.int8.onnx",
            0,
        )?)),
        // Each Bark is fed the leading silence it was evaluated with; it keeps the first word.
        ModelId::BarkClean | ModelId::Bark | ModelId::BarkPlus => {
            Engine::Parakeet(Box::new(inference::Model::load(
                &dir,
                inference::Layout::Split,
                "encoder.int8.onnx",
                if model == ModelId::BarkClean {
                    0
                } else {
                    16_000 / 4
                },
            )?))
        }
        ModelId::Cohere => {
            Engine::Cohere(Box::new(cohere::Model::load(&dir, language.as_deref())?))
        }
        ModelId::WhisperTurbo => Engine::Whisper(Box::new(whisper::Model::load(
            &dir.join(assets::WHISPER_TURBO[0].name),
            language.as_deref(),
        )?)),
    };
    let mut output = std::io::stdout().lock();
    writeln!(
        output,
        "{}",
        serde_json::to_string(&Ok::<_, String>("ready"))?
    )?;
    output.flush()?;
    let mut input = std::io::stdin().lock();
    loop {
        let mut len = [0; 8];
        if input.read_exact(&mut len).is_err() {
            break;
        }
        let len = usize::try_from(u64::from_le_bytes(len))?;
        ensure!(len <= 16_000 * 60 * 60, "Recording exceeds one hour");
        let mut bytes = vec![0; len * 4];
        input.read_exact(&mut bytes)?;
        let samples: Vec<_> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        let result = model.transcribe(samples).map_err(|e| e.to_string());
        writeln!(output, "{}", serde_json::to_string(&result)?)?;
        output.flush()?;
    }
    Ok(())
}

enum Engine {
    Parakeet(Box<inference::Model>),
    Cohere(Box<cohere::Model>),
    Whisper(Box<whisper::Model>),
}
impl Engine {
    fn transcribe(&mut self, samples: Vec<f32>) -> Result<String> {
        match self {
            Self::Parakeet(model) => model.transcribe(samples),
            Self::Cohere(model) => model.transcribe(&samples),
            Self::Whisper(model) => model.transcribe(&samples),
        }
    }
}
