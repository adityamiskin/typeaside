use anyhow::{Context, Result};

pub fn set_enabled(enabled: bool) -> Result<()> {
    let dirs = directories::BaseDirs::new().context("no home directory")?;
    let path = dirs
        .home_dir()
        .join("Library/LaunchAgents/app.hermes.plist");
    if !enabled {
        return match std::fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        };
    }
    let exe = std::env::current_exe()?;
    let exe = exe
        .to_str()
        .context("executable path is not UTF-8")?
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    std::fs::create_dir_all(path.parent().context("autostart path has no parent")?)?;
    std::fs::write(
        path,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>app.hermes</string>
<key>ProgramArguments</key><array><string>{exe}</string><string>--hidden</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>
"#
        ),
    )?;
    Ok(())
}
