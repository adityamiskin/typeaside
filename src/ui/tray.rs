#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(not(target_os = "linux"))]
mod desktop;
#[cfg(not(target_os = "linux"))]
pub use desktop::*;

const ICON_RGBA: &[u8; 32 * 32 * 4] = include_bytes!("../../assets/icons/typeaside-tray.rgba");
