use anyhow::{Context, Result};
use std::path::PathBuf;

fn entry() -> Result<PathBuf> {
    let dirs = directories::BaseDirs::new().context("no home directory")?;
    Ok(dirs.config_dir().join("autostart/hermes.desktop"))
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    let path = entry()?;
    if !enabled {
        return match std::fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        };
    }
    let exe = std::env::current_exe()?;
    std::fs::create_dir_all(path.parent().context("autostart path has no parent")?)?;
    std::fs::write(
        &path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Hermes\nExec=\"{}\" --hidden\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        ),
    )?;
    Ok(())
}
