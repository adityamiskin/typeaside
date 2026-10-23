use anyhow::Result;

pub fn web_url() -> String {
    std::env::var("TYPEASIDE_WEB_URL")
        .unwrap_or_else(|_| {
            option_env!("TYPEASIDE_WEB_URL")
                .unwrap_or("https://typeaside.app")
                .into()
        })
        .trim_end_matches('/')
        .into()
}

#[cfg(not(target_os = "windows"))]
pub fn open(target: impl AsRef<std::ffi::OsStr>) -> Result<()> {
    std::process::Command::new(if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    })
    .arg(target)
    .spawn()?;
    Ok(())
}

// explorer.exe opens Documents instead of some URLs, so let the shell pick the handler.
#[cfg(target_os = "windows")]
pub fn open(target: impl AsRef<std::ffi::OsStr>) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::{PCWSTR, w},
    };
    let target: Vec<u16> = target.as_ref().encode_wide().chain([0]).collect();
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(target.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    // Values above 32 mean success.
    if result.0 as isize <= 32 {
        anyhow::bail!(
            "cannot open {}",
            String::from_utf16_lossy(&target[..target.len() - 1])
        );
    }
    Ok(())
}
