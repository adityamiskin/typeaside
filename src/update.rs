use crate::account;
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub file: String,
    pub sha256: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Available,
    Installing,
    Failed,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    files: HashMap<String, String>,
}

fn version(value: &str) -> Result<[u64; 3]> {
    let parts = value.split('.').collect::<Vec<_>>();
    ensure!(parts.len() == 3, "Invalid release version");
    let mut result = [0; 3];
    for (index, part) in parts.into_iter().enumerate() {
        ensure!(
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()),
            "Invalid release version"
        );
        result[index] = part.parse()?;
    }
    Ok(result)
}

fn parse_manifest(body: &str, current: &str, file: &str) -> Result<Option<Release>> {
    let manifest: Manifest = serde_json::from_str(body)?;
    let newer = version(&manifest.version)? > version(current)?;
    let sha256 = manifest
        .files
        .get(file)
        .context("Missing release installer")?;
    ensure!(
        sha256.len() == 64 && sha256.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid release checksum"
    );
    if !newer {
        return Ok(None);
    }
    Ok(Some(Release {
        version: manifest.version,
        file: file.into(),
        sha256: sha256.to_ascii_lowercase(),
    }))
}

pub fn check() -> Result<Option<Release>> {
    let body = crate::http::client()
        .get(format!("{}/releases/latest.json", account::web_url()))
        .send()?
        .error_for_status()?
        .text()?;
    let file = if cfg!(target_os = "macos") {
        "TypeAside.dmg"
    } else if cfg!(target_os = "windows") {
        "TypeAside-setup.exe"
    } else {
        "TypeAside.AppImage"
    };
    parse_manifest(&body, env!("CARGO_PKG_VERSION"), file)
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

pub fn install(release: &Release) -> Result<()> {
    // Inno's uninstaller distinguishes installed Windows copies from developer binaries.
    #[cfg(target_os = "windows")]
    if !std::env::current_exe()?
        .parent()
        .is_some_and(|path| path.join("unins000.exe").is_file())
    {
        return account::open(format!("{}/download", account::web_url()));
    }
    #[cfg(target_os = "linux")]
    let destination = match std::env::var_os("APPIMAGE").filter(|path| !path.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => return account::open(format!("{}/download", account::web_url())),
    };
    #[cfg(target_os = "macos")]
    let destination = match writable_bundle() {
        Some(path) => path,
        None => return account::open(format!("{}/download", account::web_url())),
    };
    #[cfg(target_os = "windows")]
    let directory = TemporaryDirectory::new(&std::env::temp_dir())?;
    #[cfg(not(target_os = "windows"))]
    let directory = TemporaryDirectory::new(
        destination
            .parent()
            .context("Installer has no parent directory")?,
    )?;
    // The server's latest aliases can change during a download. The checksum makes that a retryable failure.
    let installer = directory.0.join(&release.file);
    {
        let mut response = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30 * 60))
            .build()?
            .get(format!("{}/releases/{}", account::web_url(), release.file))
            .send()?
            .error_for_status()?;
        let mut output = File::create(&installer)?;
        std::io::copy(&mut response, &mut output)?;
        output.sync_all()?;
        verify_sha256(&mut File::open(&installer)?, &release.sha256)?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(&installer)
            .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
            .spawn()?;
        std::process::exit(0);
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&installer, fs::Permissions::from_mode(0o755))?;
        fs::rename(&installer, &destination)?;
        if std::env::var_os("TYPEASIDE_SERVICE").is_some() {
            // The service's cgroup dies with this process, taking a relaunch script with it.
            // systemd restarts the new binary instead; the marker reopens its window.
            fs::write(restart_marker(), b"")?;
            drop(directory);
            std::process::exit(75);
        }
        relaunch(&destination, None)?;
        drop(directory);
        std::process::exit(0);
    }
    #[cfg(target_os = "macos")]
    {
        install_bundle(&installer, &destination, &directory.0)?;
        drop(directory);
        std::process::exit(0);
    }
}

#[cfg(target_os = "linux")]
fn restart_marker() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("typeaside-updated")
}

/// True once after systemd restarts typeaside into an update the user asked for.
#[cfg(target_os = "linux")]
pub fn restarted_for_update() -> bool {
    fs::remove_file(restart_marker()).is_ok()
}

#[cfg(unix)]
fn relaunch(destination: &Path, old_bundle: Option<&Path>) -> Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
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

#[cfg(target_os = "macos")]
fn writable_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bundle = exe
        .ancestors()
        .find(|path| path.extension().is_some_and(|ext| ext == "app"))?;
    for parent in [bundle, bundle.parent()?] {
        let probe = TemporaryDirectory::new(parent).ok()?;
        drop(probe);
    }
    Some(bundle.to_owned())
}

#[cfg(target_os = "macos")]
fn install_bundle(installer: &Path, destination: &Path, directory: &Path) -> Result<()> {
    use std::process::Command;
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
    let staged = directory.join("TypeAside.app");
    let copied = Command::new("ditto")
        .arg(mount.join("TypeAside.app"))
        .arg(&staged)
        .status();
    let detached = Command::new("hdiutil").arg("detach").arg(&mount).status();
    ensure!(copied?.success(), "Could not copy update");
    ensure!(detached?.success(), "Could not detach update");
    let backup = directory.with_extension("app");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_numeric_versions_and_rejects_malformed_manifests() {
        assert!(version("0.10.0").unwrap() > version("0.9.9").unwrap());
        for manifest in [
            "{}",
            r#"{"version":"0.no.0","files":{}}"#,
            r#"{"version":"0.3.0","files":{}}"#,
            r#"{"version":"0.3.0","files":{"TypeAside.AppImage":"invalid"}}"#,
        ] {
            assert!(parse_manifest(manifest, "0.2.0", "TypeAside.AppImage").is_err());
        }
        assert!(version("0.3.0.1").is_err());
        let manifest = serde_json::json!({
            "version": "0.10.0",
            "files": {"TypeAside.AppImage": "a".repeat(64)},
        })
        .to_string();
        assert!(
            parse_manifest(&manifest, "0.10.0", "TypeAside.AppImage")
                .unwrap()
                .is_none()
        );
        let release = parse_manifest(&manifest, "0.9.9", "TypeAside.AppImage")
            .unwrap()
            .unwrap();
        assert_eq!(release.version, "0.10.0");
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_sha256(&mut &b"abc"[..], expected).is_ok());
        assert!(verify_sha256(&mut &b"corrupt"[..], expected).is_err());
    }
}
