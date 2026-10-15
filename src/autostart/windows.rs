use anyhow::Result;
use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, KEY_SET_VALUE},
};

pub fn set_enabled(enabled: bool) -> Result<()> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = root.create_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_SET_VALUE,
    )?;
    if enabled {
        let exe = std::env::current_exe()?;
        key.set_value("Hermes", &format!("\"{}\" --hidden", exe.display()))?;
    } else if let Err(error) = key.delete_value("Hermes")
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(error.into());
    }
    Ok(())
}
