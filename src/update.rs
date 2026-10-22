use crate::account;

use anyhow::{Context, Result, ensure};
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::Duration,
};

#[cfg(unix)]
use std::process::Stdio;

#[cfg(test)]
mod tests;

const MANIFEST_LIMIT: u64 = 64 * 1024;
const SIGNATURE_LIMIT: u64 = 8 * 1024;
const INSTALLER_LIMIT: u64 = 512 * 1024 * 1024;
const RELEASE_KEY: &str = include_str!("../packaging/minisign.pub");

#[derive(Clone, Debug)]
pub enum InstallKind {
    #[cfg(target_os = "linux")]
    AppImage(PathBuf),
    #[cfg(target_os = "linux")]
    Deb,
    #[cfg(target_os = "linux")]
    Rpm,
    #[cfg(target_os = "macos")]
    Bundle(PathBuf),
    #[cfg(target_os = "windows")]
    Windows,
    Managed,
}

impl InstallKind {
    pub fn detected() -> &'static Self {
        static KIND: OnceLock<InstallKind> = OnceLock::new();
        KIND.get_or_init(|| {
            #[cfg(target_os = "linux")]
            {
                if let Some(path) = std::env::var_os("APPIMAGE") {
                    return Self::AppImage(PathBuf::from(path));
                }
                if std::env::current_exe().is_ok_and(|path| path == Path::new("/usr/bin/typeaside"))
                {
                    for (command, flag, kind) in
                        [("dpkg", "-S", Self::Deb), ("rpm", "-qf", Self::Rpm)]
                    {
                        if Command::new(command)
                            .args([flag, "/usr/bin/typeaside"])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .is_ok_and(|status| status.success())
                        {
                            return kind;
                        }
                    }
                }
            }
            #[cfg(target_os = "macos")]
            if let Ok(exe) = std::env::current_exe()
                && let Some(bundle) = exe
                    .ancestors()
                    .find(|path| path.extension().is_some_and(|ext| ext == "app"))
            {
                return Self::Bundle(bundle.to_owned());
            }
            #[cfg(target_os = "windows")]
            {
                Self::Windows
            }
            #[cfg(not(target_os = "windows"))]
            {
                Self::Managed
            }
        })
    }

    fn file(&self) -> Option<&'static str> {
        match self {
            #[cfg(target_os = "linux")]
            Self::AppImage(_) => Some("TypeAside.AppImage"),
            #[cfg(target_os = "linux")]
            Self::Deb => Some("typeaside.deb"),
            #[cfg(target_os = "linux")]
            Self::Rpm => Some("typeaside.rpm"),
            #[cfg(target_os = "macos")]
            Self::Bundle(_) => Some("TypeAside.dmg"),
            #[cfg(target_os = "windows")]
            Self::Windows => Some("TypeAside-setup.exe"),
            Self::Managed => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub installer: Option<Installer>,
}

#[derive(Clone, Debug)]
pub struct Installer {
    pub file: &'static str,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub enum Progress {
    Downloading(u8),
    Installing,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    files: HashMap<String, String>,
}

fn version(value: &str) -> Result<semver::Version> {
    semver::Version::parse(value).context("Invalid release version")
}

fn parse_manifest(
    body: &str,
    current: &str,
    file: Option<&'static str>,
) -> Result<Option<Release>> {
    let manifest: Manifest = serde_json::from_str(body)?;
    if !version(&manifest.version)?
        .cmp_precedence(&version(current)?)
        .is_gt()
    {
        return Ok(None);
    }
    let installer = match file {
        None => None,
        Some(file) => {
            let checksum = manifest
                .files
                .get(file)
                .context("Missing release installer")?;
            ensure!(
                checksum.len() == 64 && checksum.bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid release checksum"
            );
            Some(Installer {
                file,
                sha256: checksum.to_ascii_lowercase(),
            })
        }
    };
    Ok(Some(Release {
        version: manifest.version,
        installer,
    }))
}

fn verified_manifest(
    body: &[u8],
    signature: &str,
    key: &str,
    current: &str,
    file: Option<&'static str>,
) -> Result<Option<Release>> {
    verify_signature(&mut &*body, signature, key)?;
    parse_manifest(std::str::from_utf8(body)?, current, file)
}

fn copy_capped(
    reader: &mut impl Read,
    writer: &mut impl Write,
    limit: u64,
    mut progress: impl FnMut(u64),
) -> Result<()> {
    let mut received = 0u64;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).context("Download interrupted")?;
        if count == 0 {
            return Ok(());
        }
        received += count as u64;
        ensure!(
            received <= limit,
            "Download exceeds size limit ({limit} bytes)"
        );
        writer
            .write_all(&buffer[..count])
            .context("Cannot save download")?;
        progress(received);
    }
}

fn read_capped(reader: &mut impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    copy_capped(reader, &mut body, limit, |_| {})?;
    Ok(body)
}

fn download_body(url: &str, limit: u64) -> Result<Vec<u8>> {
    let mut response = crate::http::client()
        .get(url)
        .timeout(Duration::from_secs(15))
        .send()?
        .error_for_status()?;
    read_capped(&mut response, limit)
}

fn check_at(
    base: &str,
    key: &str,
    current: &str,
    file: Option<&'static str>,
) -> Result<Option<Release>> {
    let body = download_body(&format!("{base}/releases/latest.json"), MANIFEST_LIMIT)?;
    let signature = download_body(
        &format!("{base}/releases/latest.json.minisig"),
        SIGNATURE_LIMIT,
    )?;
    verified_manifest(&body, std::str::from_utf8(&signature)?, key, current, file)
}

pub fn check() -> Result<Option<Release>> {
    check_at(
        &account::web_url(),
        RELEASE_KEY,
        env!("CARGO_PKG_VERSION"),
        InstallKind::detected().file(),
    )
}

fn verify_sha256(reader: &mut impl Read, expected: &str) -> Result<()> {
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == expected,
        "Release checksum mismatch"
    );
    Ok(())
}

fn verify_signature(reader: &mut impl Read, signature: &str, key: &str) -> Result<()> {
    let key = PublicKey::decode(key).context("Invalid embedded release key")?;
    let signature = Signature::decode(signature).context("Invalid release signature")?;
    let mut verifier = key
        .verify_stream(&signature)
        .context("Invalid release signature")?;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        verifier.update(&buffer[..count]);
    }
    verifier
        .finalize()
        .context("Release signature verification failed")
}

struct TemporaryDirectory(PathBuf);
impl TemporaryDirectory {
    fn new(parent: &Path) -> Result<Self> {
        let mut random = [0; 16];
        getrandom::fill(&mut random)
            .map_err(|_| anyhow::anyhow!("Could not create update directory"))?;
        let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let path = parent.join(format!(".typeaside-update-{suffix}"));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn install(release: &Release, mut progress: impl FnMut(Progress)) -> Result<()> {
    let kind = InstallKind::detected();
    let artifact = release
        .installer
        .as_ref()
        .context("Update with your package manager")?;
    ensure!(
        kind.file() == Some(artifact.file),
        "No installer for this installation"
    );
    let directory = TemporaryDirectory::new(&std::env::temp_dir())?;
    let installer = directory.0.join(artifact.file);
    let mut response = crate::http::client()
        .get(format!(
            "{}/releases/v{}/{}",
            account::web_url(),
            release.version,
            artifact.file
        ))
        .timeout(Duration::from_secs(30 * 60))
        .send()
        .context("Download failed")?
        .error_for_status()
        .context("Download failed")?;
    let total = response.content_length();
    let mut output = File::create(&installer).context("Cannot save download")?;
    let mut previous = 0;
    progress(Progress::Downloading(0));
    copy_capped(&mut response, &mut output, INSTALLER_LIMIT, |received| {
        let percent = total
            .filter(|n| *n > 0)
            .map_or(0, |n| (received.saturating_mul(100) / n).min(99) as u8);
        if percent != previous {
            progress(Progress::Downloading(percent));
            previous = percent;
        }
    })?;
    output.sync_all()?;
    drop(output);
    verify_sha256(&mut File::open(&installer)?, &artifact.sha256)?;
    let signature = download_body(
        &format!(
            "{}/releases/v{}/{}.minisig",
            account::web_url(),
            release.version,
            artifact.file
        ),
        SIGNATURE_LIMIT,
    )?;
    verify_signature(
        &mut File::open(&installer)?,
        std::str::from_utf8(&signature)?,
        RELEASE_KEY,
    )?;
    progress(Progress::Downloading(100));
    progress(Progress::Installing);
    match kind {
        #[cfg(target_os = "linux")]
        InstallKind::AppImage(destination) => {
            replace_appimage(&installer, destination, &release.version)?;
            relaunch(destination, None)?;
        }
        #[cfg(target_os = "linux")]
        InstallKind::Deb | InstallKind::Rpm => {
            let manager = if matches!(kind, InstallKind::Deb) {
                // apt drops privileges to _apt while reading local packages.
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o755))?;
                fs::set_permissions(&installer, fs::Permissions::from_mode(0o644))?;
                "apt-get"
            } else {
                "dnf"
            };
            ensure!(
                Command::new("pkexec")
                    .args([manager, "install", "-y"])
                    .arg(&installer)
                    .status()
                    .context("Could not start package installer")?
                    .success(),
                "Package installation failed"
            );
            relaunch(&std::env::current_exe()?, None)?;
        }
        #[cfg(target_os = "macos")]
        InstallKind::Bundle(destination) => install_bundle(&installer, destination, &directory.0)?,
        #[cfg(target_os = "windows")]
        InstallKind::Windows => {
            Command::new(&installer)
                .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
                .spawn()
                .context("Could not start installer")?;
            // The installer must remain on disk until its child process has read it.
            std::mem::forget(directory);
        }
        InstallKind::Managed => anyhow::bail!("Update with your package manager"),
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn check_version_output(output: &[u8], expected: &str) -> Result<()> {
    ensure!(
        std::str::from_utf8(output)?.trim() == expected,
        "Staged AppImage version mismatch"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_appimage(
    staged: &Path,
    output: &Path,
    expected: &str,
    timeout: Duration,
) -> Result<()> {
    let mut child = Command::new(staged)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(File::create(output)?)
        .stderr(Stdio::null())
        .spawn()
        .context("Could not run staged AppImage")?;
    let started = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(25))
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return match result {
                    Err(error) => Err(error.into()),
                    _ => anyhow::bail!("Staged AppImage version check timed out"),
                };
            }
        }
    };
    ensure!(status.success(), "Staged AppImage version check failed");
    check_version_output(&read_capped(&mut File::open(output)?, 128)?, expected)
}

#[cfg(target_os = "linux")]
fn replace_appimage(installer: &Path, destination: &Path, expected: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let staging = TemporaryDirectory::new(
        destination
            .parent()
            .context("AppImage has no parent directory")?,
    )?;
    let staged = staging.0.join("TypeAside.AppImage");
    fs::copy(installer, &staged)?;
    let target = File::open(&staged)?;
    target.set_permissions(fs::Permissions::from_mode(0o755))?;
    target.sync_all()?;
    validate_appimage(
        &staged,
        &staging.0.join("version"),
        expected,
        Duration::from_secs(10),
    )?;
    let backup = staging.0.join("previous");
    fs::hard_link(destination, &backup)
        .or_else(|_| fs::copy(destination, &backup).map(drop))
        .context("Cannot preserve previous AppImage")?;
    let mut previous = destination.as_os_str().to_os_string();
    previous.push(".previous");
    fs::rename(&backup, PathBuf::from(previous))?;
    fs::rename(&staged, destination).context("Cannot replace AppImage")?;
    File::open(
        destination
            .parent()
            .context("AppImage has no parent directory")?,
    )?
    .sync_all()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn restart_marker() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("typeaside-updated")
}

#[cfg(target_os = "linux")]
pub fn restarted_for_update() -> bool {
    fs::remove_file(restart_marker()).is_ok()
}

#[cfg(unix)]
fn relaunch(destination: &Path, old_bundle: Option<&Path>) -> Result<()> {
    use std::os::unix::process::CommandExt;
    #[cfg(target_os = "linux")]
    if std::env::var_os("TYPEASIDE_SERVICE").is_some() {
        // systemd kills this cgroup at exit, so its restart policy must relaunch the app.
        fs::write(restart_marker(), b"")?;
        return Ok(());
    }
    let script = if cfg!(target_os = "macos") {
        "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; if [ -n \"$3\" ]; then rm -rf -- \"$3\"; fi; exec open \"$2\""
    } else {
        "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; exec \"$2\""
    };
    Command::new("sh")
        .args([
            "-c",
            script,
            "typeaside-update",
            &std::process::id().to_string(),
        ])
        .arg(destination)
        .arg(old_bundle.unwrap_or_else(|| Path::new("")))
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn quit_after_install() -> ! {
    #[cfg(target_os = "linux")]
    if std::env::var_os("TYPEASIDE_SERVICE").is_some() {
        std::process::exit(75);
    }
    std::process::exit(0)
}

#[cfg(target_os = "macos")]
fn install_bundle(installer: &Path, destination: &Path, directory: &Path) -> Result<()> {
    let staging = TemporaryDirectory::new(
        destination
            .parent()
            .context("Application has no parent directory")?,
    )?;
    let mount = directory.join("mount");
    fs::create_dir(&mount)?;
    ensure!(
        Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
            .arg(&mount)
            .arg(installer)
            .status()?
            .success(),
        "Could not mount update"
    );
    let staged = staging.0.join("TypeAside.app");
    let copied = Command::new("ditto")
        .arg(mount.join("TypeAside.app"))
        .arg(&staged)
        .status();
    let detached = Command::new("hdiutil").arg("detach").arg(&mount).status();
    ensure!(copied?.success(), "Could not copy update");
    ensure!(detached?.success(), "Could not detach update");
    let backup = staging.0.with_extension("app");
    fs::rename(destination, &backup)?;
    if let Err(error) = fs::rename(&staged, destination) {
        fs::rename(&backup, destination).context("Could not restore previous application")?;
        return Err(error.into());
    }
    if let Err(error) = relaunch(destination, Some(&backup)) {
        fs::rename(destination, &staged)?;
        fs::rename(&backup, destination)?;
        return Err(error);
    }
    Ok(())
}
