mod assets;
mod catalog;
mod cleanup;
pub use cleanup::{CleanupModel, CleanupSettings};
mod gguf;
mod vad;
pub use catalog::ModelId;

use anyhow::{Context, Result, ensure};
use assets::SILERO_VAD;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub const AVAILABLE: bool = cfg!(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
));

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
fn inventory(model: ModelId, dir: &Path) -> Result<String> {
    inventory_assets(
        dir,
        model
            .assets()
            .iter()
            .map(|asset| asset.name)
            .chain([SILERO_VAD.name]),
    )
}

fn inventory_assets(dir: &Path, names: impl Iterator<Item = &'static str>) -> Result<String> {
    let mut entries = Vec::new();
    for name in names {
        let path = dir.join(name);
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
    let dir = model.directory()?;
    let assets: Vec<_> = model
        .assets()
        .iter()
        .chain([&SILERO_VAD])
        .copied()
        .collect();
    download_assets(&dir, &assets, cancel, progress).await
}

async fn download_assets(
    dir: &Path,
    assets: &[assets::Asset],
    cancel: &AtomicBool,
    progress: impl Fn(u8),
) -> Result<()> {
    fs::create_dir_all(dir)?;
    let _ = fs::remove_file(dir.join("installed"));
    let total: u64 = assets.iter().map(|a| a.size).sum();
    let mut completed = 0;
    let mut last = std::time::Instant::now() - Duration::from_secs(1);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(1800))
        .build()?;
    for asset in assets {
        let target = &dir;
        ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
        let final_path = target.join(asset.name);
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
        ensure!(
            fs::metadata(&part)?.len() == asset.size,
            "Download size mismatch"
        );
        verify(&part, asset.sha256, cancel)?;
        fs::rename(part, final_path)?;
    }
    fs::write(
        dir.join("installed.part"),
        inventory_assets(dir, assets.iter().map(|a| a.name))?,
    )?;
    fs::rename(dir.join("installed.part"), dir.join("installed"))?;
    progress(100);
    Ok(())
}

struct Worker {
    child: Child,
    model: ModelId,
    language: Option<String>,
    requests: Option<mpsc::Sender<Job>>,
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
        let (requests, input) = mpsc::channel::<Job>();
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
            for job in input {
                let answered = matches!(job, Job::Transcribe(_) | Job::End | Job::Clean(..));
                let result = (|| -> Result<String> {
                    job.write(&mut stdin)?;
                    stdin.flush()?;
                    if answered {
                        response(&mut stdout)
                    } else {
                        Ok(String::new())
                    }
                })()
                .map_err(|e| e.to_string());
                if (answered || result.is_err()) && output.send(result).is_err() {
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
        self.answer(timeout)?.map_err(anyhow::Error::msg)
    }
    /// The outer error means the worker is gone or out of step; the inner one is its answer.
    fn answer(&self, timeout: Duration) -> Result<Result<String, String>> {
        self.responses
            .recv_timeout(timeout)
            .context("On-device transcription worker timed out or disconnected")
    }
    fn send(&self, job: Job) -> Result<()> {
        self.requests
            .as_ref()
            .context("worker requests unavailable")?
            .send(job)?;
        Ok(())
    }
    fn transcribe(&mut self, audio: &[f32]) -> Result<String> {
        let timeout = 30 + (audio.len() as u64).div_ceil(16_000) * 2;
        self.send(Job::Transcribe(audio.to_vec()))?;
        self.response(Duration::from_secs(timeout))
    }
}

const TRANSCRIBE: u8 = 0;
const BEGIN: u8 = 1;
const FEED: u8 = 2;
const END: u8 = 3;
const CANCEL: u8 = 4;
const CLEAN: u8 = 5;

/// One message to the worker. Transcription, stream end and cleanup are answered.
enum Job {
    Transcribe(Vec<f32>),
    Begin,
    Feed(Vec<f32>),
    End,
    Cancel,
    Clean(String, CleanupSettings),
}
impl Job {
    fn write(&self, output: &mut impl Write) -> Result<()> {
        let (tag, samples) = match self {
            Self::Transcribe(samples) => (TRANSCRIBE, Some(samples)),
            Self::Begin => (BEGIN, None),
            Self::Feed(samples) => (FEED, Some(samples)),
            Self::End => (END, None),
            Self::Cancel => (CANCEL, None),
            Self::Clean(text, settings) => {
                let bytes = serde_json::to_vec(&(text, settings))?;
                ensure!(bytes.len() <= 1024 * 1024, "Cleanup request too large");
                output.write_all(&[CLEAN])?;
                output.write_all(&(bytes.len() as u64).to_le_bytes())?;
                output.write_all(&bytes)?;
                return Ok(());
            }
        };
        output.write_all(&[tag])?;
        if let Some(samples) = samples {
            output.write_all(&(samples.len() as u64).to_le_bytes())?;
            let bytes: Vec<_> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            output.write_all(&bytes)?;
        }
        Ok(())
    }
}

fn read_job(input: &mut impl Read) -> Result<Option<(u8, Vec<f32>)>> {
    let mut tag = [0];
    if input.read_exact(&mut tag).is_err() {
        return Ok(None);
    }
    if !matches!(tag[0], TRANSCRIBE | FEED) {
        return Ok(Some((tag[0], Vec::new())));
    }
    let mut len = [0; 8];
    input.read_exact(&mut len)?;
    let len = usize::try_from(u64::from_le_bytes(len))?;
    ensure!(len <= 16_000 * 60 * 60, "Recording exceeds one hour");
    let mut bytes = vec![0; len * 4];
    input.read_exact(&mut bytes)?;
    let samples = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    Ok(Some((tag[0], samples)))
}

type Reply = mpsc::SyncSender<std::result::Result<String, String>>;
enum Request {
    Idle(Option<Duration>),
    Warm(ModelId, Option<String>),
    Transcribe(ModelId, Option<String>, Vec<f32>, Reply),
    Begin(ModelId, Option<String>),
    Feed(Vec<f32>),
    End(Reply),
    Cancel,
    Clean(String, CleanupSettings, Arc<AtomicBool>, Reply),
    Unload(Option<mpsc::SyncSender<()>>),
}
fn ensure(
    worker: &mut Option<Worker>,
    model: ModelId,
    language: Option<String>,
) -> Result<&mut Worker> {
    if worker
        .as_ref()
        .is_none_or(|w| w.model != model || w.language != language)
    {
        drop(worker.take());
        *worker = Some(Worker::start(model, language)?);
    }
    worker.as_mut().context("missing worker")
}
fn sender() -> &'static mpsc::Sender<Request> {
    static SENDER: OnceLock<mpsc::Sender<Request>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut worker: Option<Worker> = None;
            let mut streaming = false;
            let mut idle = crate::config::LocalUnload::default().after();
            loop {
                let request = match idle.filter(|_| worker.is_some() && !streaming) {
                    Some(idle) => rx.recv_timeout(idle),
                    None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
                };
                match request {
                    Ok(Request::Idle(after)) => idle = after,
                    Ok(Request::Warm(model, language)) => {
                        if !streaming && ensure(&mut worker, model, language).is_err() {
                            worker = None;
                        }
                    }
                    Ok(Request::Transcribe(model, language, audio, reply)) => {
                        let result = ensure(&mut worker, model, language)
                            .and_then(|worker| worker.transcribe(&audio));
                        if result.is_err() {
                            worker = None;
                        }
                        let _ = reply.send(result.map_err(|e: anyhow::Error| e.to_string()));
                    }
                    Ok(Request::Begin(model, language)) => {
                        streaming = ensure(&mut worker, model, language)
                            .and_then(|worker| worker.send(Job::Begin))
                            .is_ok();
                    }
                    Ok(Request::Feed(audio)) => {
                        if streaming && let Some(worker) = &worker {
                            let _ = worker.send(Job::Feed(audio));
                        }
                    }
                    Ok(Request::End(reply)) => {
                        let result = match worker.as_ref().filter(|_| streaming).map(|live| {
                            live.send(Job::End)
                                .and_then(|()| live.answer(Duration::from_secs(30)))
                        }) {
                            Some(Ok(answer)) => answer,
                            // The worker is gone or still mid-stream, out of step with its answers.
                            Some(Err(error)) => {
                                worker = None;
                                Err(error.to_string())
                            }
                            None => Err("No stream".into()),
                        };
                        streaming = false;
                        let _ = reply.send(result);
                    }
                    Ok(Request::Cancel) => {
                        if streaming && let Some(worker) = &worker {
                            let _ = worker.send(Job::Cancel);
                        }
                        streaming = false;
                    }
                    Ok(Request::Clean(text, settings, cancel, reply)) => {
                        let result = (|| -> Result<Result<String, String>> {
                            ensure!(!cancel.load(Ordering::Relaxed), "Cleanup cancelled");
                            let live = worker.as_ref().context("Speech worker is not loaded")?;
                            live.send(Job::Clean(text, settings))?;
                            let started = std::time::Instant::now();
                            loop {
                                ensure!(!cancel.load(Ordering::Relaxed), "Cleanup cancelled");
                                ensure!(
                                    started.elapsed() < Duration::from_secs(30),
                                    "Cleanup timed out"
                                );
                                match live.responses.recv_timeout(Duration::from_millis(20)) {
                                    Ok(result) => return Ok(result),
                                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                                    Err(error) => return Err(error.into()),
                                }
                            }
                        })();
                        if result.is_err() {
                            worker = None;
                        }
                        let _ = reply.send(result.unwrap_or_else(|e| Err(e.to_string())));
                    }
                    Ok(Request::Unload(reply)) => {
                        worker = None;
                        streaming = false;
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

/// Feeds the recorder's audio to the worker while the shortcut is held. Dropping it before
/// `release` abandons the stream.
pub struct Stream {
    released: Option<mpsc::Receiver<std::result::Result<String, String>>>,
}

type Feed = Arc<dyn Fn(&[f32]) + Send + Sync>;

/// Sends recorder samples to a live consumer in fixed-size blocks.
#[derive(Clone)]
pub struct AudioSink {
    pub block: usize,
    send: Feed,
}

impl AudioSink {
    pub(crate) fn new(block: usize, send: impl Fn(&[f32]) + Send + Sync + 'static) -> Self {
        Self {
            block,
            send: Arc::new(send),
        }
    }

    pub fn send(&self, samples: &[f32]) {
        (self.send)(samples);
    }
}

impl Stream {
    pub fn start(config: &crate::config::AppConfig) -> Option<Self> {
        let model = config.local_model.available()?;
        sender()
            .send(Request::Begin(model, config.language.clone()))
            .ok()?;
        Some(Self { released: None })
    }
    pub fn audio(&self) -> AudioSink {
        AudioSink::new(16_000 * 160 / 1000, |samples| {
            let _ = sender().send(Request::Feed(samples.to_vec()));
        })
    }
    /// Finalizes at release, so this stream ends before a later capture begins its own.
    pub fn release(&mut self) {
        let (tx, rx) = mpsc::sync_channel(1);
        if self.released.is_none() && sender().send(Request::End(tx)).is_ok() {
            self.released = Some(rx);
        }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        if self.released.is_none() {
            let _ = sender().send(Request::Cancel);
        }
    }
}

/// The stream's text, or the whole clip offline when the stream failed, heard nothing or
/// missed its finalize deadline. CPU workers never stream. True when the stream's text is used.
pub fn finish(
    config: &crate::config::AppConfig,
    audio: &[f32],
    stream: Option<Stream>,
) -> Result<(String, bool)> {
    let started = std::time::Instant::now();
    let streamed = stream.and_then(|mut stream| {
        stream.release();
        stream.released.as_ref()?.recv().ok()
    });
    if std::env::var_os("TYPEASIDE_TRACE").is_some_and(|value| value == "1") {
        eprintln!(
            "typeaside local: stream={} finish={} ms",
            match &streamed {
                Some(Ok(text)) if !text.is_empty() => "text".to_owned(),
                Some(Ok(_)) => "empty".to_owned(),
                Some(Err(error)) => error.clone(),
                None => "none".to_owned(),
            },
            started.elapsed().as_millis()
        );
    }
    match streamed {
        Some(Ok(text)) if !text.is_empty() => Ok((text, true)),
        _ => Ok((transcribe(config, audio)?, false)),
    }
}

/// Applies the same on-device cleanup policy used by the dictation pipeline.
pub fn cleanup_text(
    config: &crate::config::AppConfig,
    text: &str,
    app_id: Option<&str>,
    cancel: Arc<AtomicBool>,
) -> String {
    if let Some(settings) = CleanupSettings::for_app(config, app_id) {
        crate::cleanup::local_output(text, clean(text, settings, cancel))
    } else {
        crate::cleanup::remove_fillers(text, config.language.as_deref())
    }
}

fn clean(text: &str, settings: CleanupSettings, cancel: Arc<AtomicBool>) -> Result<String> {
    let (tx, rx) = mpsc::sync_channel(1);
    sender().send(Request::Clean(text.to_owned(), settings, cancel, tx))?;
    rx.recv()?.map_err(anyhow::Error::msg)
}

pub fn worker_main() -> Result<()> {
    let option = |flag: &str| std::env::args().skip_while(|arg| arg != flag).nth(1);
    let model: ModelId = option("--local-model")
        .map(|key| key.parse())
        .transpose()?
        .unwrap_or_default();
    let language = option("--local-language");
    let language = if model == ModelId::Cohere {
        let language = language.unwrap_or_else(|| "en".to_owned());
        ensure!(
            [
                "ar", "de", "el", "en", "es", "fr", "it", "ja", "ko", "nl", "pl", "pt", "vi", "zh"
            ]
            .contains(&language.as_str()),
            "Cohere does not support language {language}; choose a language in Settings"
        );
        Some(language)
    } else {
        language
    };
    let dir = model.directory()?;
    ensure!(
        model.installed(),
        "On-device model is missing or has changed; download it again"
    );
    let mut model = gguf::Model::load(&dir.join(model.assets()[0].name), language.as_deref())?;
    if std::env::var_os("TYPEASIDE_TRACE").is_some_and(|value| value == "1") {
        eprintln!("typeaside local: device {}", model.device());
    }
    let mut vad = vad::Vad::load(&dir.join(SILERO_VAD.name))?;
    let mut output = std::io::stdout().lock();
    writeln!(
        output,
        "{}",
        serde_json::to_string(&Ok::<_, String>("ready"))?
    )?;
    output.flush()?;
    let mut cleanup = None;
    let mut input = std::io::stdin().lock();
    while let Some((tag, samples)) = read_job(&mut input)? {
        let result = match tag {
            TRANSCRIBE => Some(transcribe_speech(&mut model, &mut vad, &samples)),
            BEGIN => stream(&mut model, &mut input, &mut cleanup, &mut output),
            CLEAN => Some(cleanup_job(&mut input, &mut cleanup)),
            _ => anyhow::bail!("Unexpected message outside a stream"),
        };
        if let Some(result) = result {
            let result = result.map_err(|e| e.to_string());
            writeln!(output, "{}", serde_json::to_string(&result)?)?;
            output.flush()?;
        }
    }
    Ok(())
}

// No speech yields no chunks and an empty transcript, which the engine discards quietly.
fn transcribe_speech(
    model: &mut gguf::Model,
    vad: &mut vad::Vad,
    samples: &[f32],
) -> Result<String> {
    let started = std::time::Instant::now();
    let chunks = vad.chunks(samples)?;
    if std::env::var_os("TYPEASIDE_TRACE").is_some_and(|value| value == "1") {
        eprintln!(
            "typeaside: VAD samples={} chunks={chunks:?} elapsed={} ms",
            samples.len(),
            started.elapsed().as_millis()
        );
    }
    let mut parts = Vec::new();
    for chunk in chunks {
        let text = model.transcribe(&samples[chunk])?;
        let text = text.trim();
        if !text.is_empty() {
            parts.push(text.to_owned());
        }
    }
    Ok(parts.join(" "))
}

fn cleanup_job(input: &mut impl Read, model: &mut Option<cleanup::Model>) -> Result<String> {
    let mut size = [0; 8];
    input.read_exact(&mut size)?;
    let size = usize::try_from(u64::from_le_bytes(size))?;
    ensure!(size <= 1024 * 1024, "Cleanup request too large");
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes)?;
    let (text, settings): (String, CleanupSettings) = serde_json::from_slice(&bytes)?;
    if model.is_none() {
        *model = Some(cleanup::Model::load()?);
    }
    model
        .as_mut()
        .context("Missing cleanup model")?
        .clean(&text, settings)
}

/// Runs one stream to its end; `None` when it was cancelled and owes no answer.
fn stream(
    model: &mut gguf::Model,
    input: &mut impl Read,
    cleanup: &mut Option<cleanup::Model>,
    output: &mut impl Write,
) -> Option<Result<String>> {
    let mut next = || -> Result<gguf::StreamInput> {
        loop {
            return Ok(match read_job(input)? {
                Some((FEED, samples)) => gguf::StreamInput::Feed(samples),
                Some((END, _)) => gguf::StreamInput::End,
                Some((CANCEL, _)) => gguf::StreamInput::Cancel,
                // The next capture may already be streaming when the previous dictation cleans up.
                Some((CLEAN, _)) => {
                    let result = cleanup_job(input, cleanup).map_err(|e| e.to_string());
                    writeln!(output, "{}", serde_json::to_string(&result)?)?;
                    output.flush()?;
                    continue;
                }
                _ => anyhow::bail!("Stream interrupted"),
            });
        }
    };
    if model.streams() {
        model.stream(next).transpose()
    } else {
        loop {
            match next() {
                Ok(gguf::StreamInput::Feed(_)) => {}
                Ok(gguf::StreamInput::End) => {
                    break Some(Err(anyhow::anyhow!("This model does not stream here")));
                }
                Ok(gguf::StreamInput::Cancel) => break None,
                Err(error) => break Some(Err(error)),
            }
        }
    }
}
