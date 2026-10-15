use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const UNIT: &str = "hermes.service";

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

/// A systemd user service tied to the graphical session, so a crash restarts hermes and the
/// hotkey keeps working with no window open. Enabling is a symlink, which needs no running
/// systemctl and takes effect at the next login.
pub fn set_enabled(enabled: bool) -> Result<()> {
    let config = directories::BaseDirs::new()
        .context("no home directory")?
        .config_dir()
        .to_owned();
    let units = config.join("systemd/user");
    let wants = units.join("graphical-session.target.wants").join(UNIT);
    remove(&config.join("autostart/hermes.desktop"))?;
    remove(&wants)?;
    if !enabled {
        return remove(&units.join(UNIT));
    }
    // Inside an AppImage, current_exe is a mount that disappears when hermes exits.
    let exe = std::env::var_os("APPIMAGE")
        .filter(|path| !path.is_empty())
        .map_or_else(std::env::current_exe, |path| Ok(PathBuf::from(path)))?;
    let exe = exe.to_str().context("executable path is not UTF-8")?;
    std::fs::create_dir_all(wants.parent().context("unit path has no parent")?)?;
    std::fs::write(
        units.join(UNIT),
        format!(
            "[Unit]\nDescription=Hermes voice dictation\nPartOf=graphical-session.target\nAfter=graphical-session.target\n\n\
             [Service]\nExecStart=\"{}\" --hidden\nEnvironment=HERMES_SERVICE=1\nRestart=on-failure\nRestartSec=1\n\n\
             [Install]\nWantedBy=graphical-session.target\n",
            exe.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('%', "%%")
        ),
    )?;
    std::os::unix::fs::symlink(Path::new("..").join(UNIT), wants)?;
    Ok(())
}
