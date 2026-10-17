pub const TRAY_RGBA: &[u8; 32 * 32 * 4] = include_bytes!("../assets/icons/typeaside-tray.rgba");

#[cfg(target_os = "linux")]
pub fn install() {
    let Some(dirs) = directories::BaseDirs::new() else {
        return;
    };
    let path = dirs
        .data_dir()
        .join("icons/hicolor/scalable/apps/typeaside.svg");
    let icon = include_bytes!("../assets/typeaside.svg");
    if std::fs::read(&path).is_ok_and(|current| current == icon) {
        return;
    }
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_ok()
    {
        let _ = std::fs::write(path, icon);
    }
}
