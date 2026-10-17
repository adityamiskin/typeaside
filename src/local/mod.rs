mod assets;
mod inference;

use anyhow::{Context, Result, ensure};
use assets::{MODEL, RUNTIME};
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
    pub fn current() -> Self {
        if installed() {
            Self::Installed
        } else {
            Self::NotInstalled
        }
    }
}

pub fn directory() -> Result<PathBuf> {
    Ok(crate::paths::AppPaths::discover()?
        .data_dir
        .join("models/parakeet-v3"))
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

fn inventory(dir: &Path) -> Result<String> {
    let mut entries = Vec::new();
    for name in MODEL
        .iter()
        .map(|asset| asset.name)
        .chain(std::iter::once(library_name()))
    {
        let metadata = fs::metadata(dir.join(name))?;
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
    AVAILABLE
        && directory().is_ok_and(|dir| {
            fs::read_to_string(dir.join("installed"))
                .ok()
                .zip(inventory(&dir).ok())
                .is_some_and(|(saved, current)| saved == current)
        })
}

pub fn download_size_mb() -> u64 {
    MODEL
        .iter()
        .chain(&RUNTIME)
        .map(|a| a.size)
        .sum::<u64>()
        .div_ceil(1_000_000)
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

pub fn download(cancel: &AtomicBool, progress: impl Fn(u8)) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(download_async(cancel, progress))
}

async fn cancelled(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn download_async(cancel: &AtomicBool, progress: impl Fn(u8)) -> Result<()> {
    let Some(runtime) = &RUNTIME else {
        anyhow::bail!("On-device transcription is not available on this platform");
    };
    let dir = directory()?;
    fs::create_dir_all(&dir)?;
    let _ = fs::remove_file(dir.join("installed"));
    let total: u64 = MODEL.iter().chain(&RUNTIME).map(|a| a.size).sum();
    let mut completed = 0;
    let mut last = std::time::Instant::now() - Duration::from_secs(1);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(1800))
        .build()?;
    for asset in MODEL.iter().chain(&RUNTIME) {
        ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
        let final_path = dir.join(asset.name);
        if verify(&final_path, asset.sha256, cancel).is_ok() {
            completed += asset.size;
            continue;
        }
        ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
        let part = dir.join(format!("{}.part", asset.name));
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
    let archive = dir.join(runtime.name);
    if runtime.name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(File::open(&archive)?)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if Path::new(entry.name())
                .file_name()
                .is_some_and(|n| n == library_name())
            {
                let mut out = File::create(dir.join(format!("{}.part", library_name())))?;
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
                entry.unpack(dir.join(format!("{}.part", library_name())))?;
            }
        }
    }
    ensure!(!cancel.load(Ordering::Relaxed), "Download cancelled");
    fs::rename(
        dir.join(format!("{}.part", library_name())),
        dir.join(library_name()),
    )?;
    fs::remove_file(archive)?;
    fs::write(dir.join("installed.part"), inventory(&dir)?)?;
    fs::rename(dir.join("installed.part"), dir.join("installed"))?;
    progress(100);
    Ok(())
}

struct Worker {
    child: Child,
    output: BufReader<std::process::ChildStdout>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Worker {
    fn start() -> Result<Self> {
        let executable = std::env::current_exe()?;
        let mut child = Command::new(executable)
            .arg("--local-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let output = BufReader::new(child.stdout.take().context("worker stdout")?);
        let mut worker = Self { child, output };
        worker.response()?;
        Ok(worker)
    }
    fn response(&mut self) -> Result<String> {
        let mut line = String::new();
        ensure!(
            self.output.read_line(&mut line)? > 0,
            "On-device transcription process exited"
        );
        let response: std::result::Result<String, String> = serde_json::from_str(&line)?;
        response.map_err(anyhow::Error::msg)
    }
    fn transcribe(&mut self, audio: &[f32]) -> Result<String> {
        let input = self.child.stdin.as_mut().context("worker stdin")?;
        input.write_all(&(audio.len() as u64).to_le_bytes())?;
        let bytes: Vec<_> = audio
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
        input.write_all(&bytes)?;
        input.flush()?;
        self.response()
    }
}

type Reply = mpsc::SyncSender<std::result::Result<String, String>>;
enum Request {
    Warm,
    Transcribe(Vec<f32>, Reply),
    Unload(mpsc::SyncSender<()>),
}
fn sender() -> &'static mpsc::Sender<Request> {
    static SENDER: OnceLock<mpsc::Sender<Request>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut worker = None;
            loop {
                match rx.recv_timeout(Duration::from_secs(300)) {
                    Ok(Request::Warm) => {
                        if worker.is_none() {
                            worker = Worker::start().ok();
                        }
                    }
                    Ok(Request::Transcribe(audio, reply)) => {
                        let result = (|| {
                            if worker.is_none() {
                                worker = Some(Worker::start()?);
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
                        let _ = reply.send(());
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

pub fn prewarm() {
    if installed() {
        let _ = sender().send(Request::Warm);
    }
}
pub fn unload() {
    let (tx, rx) = mpsc::sync_channel(1);
    if sender().send(Request::Unload(tx)).is_ok() {
        let _ = rx.recv();
    }
}
pub fn remove() -> Result<()> {
    unload();
    let dir = directory()?;
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}
pub fn transcribe(audio: &[f32]) -> Result<String> {
    ensure!(
        installed(),
        "Download the on-device model in Settings first"
    );
    let (tx, rx) = mpsc::sync_channel(1);
    sender().send(Request::Transcribe(audio.to_vec(), tx))?;
    rx.recv()?.map_err(anyhow::Error::msg)
}

pub fn worker_main() -> Result<()> {
    let dir = directory()?;
    ensure!(
        installed(),
        "On-device model is missing or has changed; download it again"
    );
    ort::init_from(dir.join(library_name()))?.commit();
    let mut model = inference::Model::load(&dir)?;
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
