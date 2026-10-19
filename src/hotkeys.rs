mod double_tap;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub mod globe;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod desktop;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use desktop::{HotkeyEvent, chord};

#[cfg(target_os = "windows")]
pub(crate) use windows::{held_modifiers, set_cancelable};

static SHORTCUT_RECORDING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
pub fn set_shortcut_recording(recording: bool) {
    SHORTCUT_RECORDING.store(recording, std::sync::atomic::Ordering::Relaxed);
}
pub(crate) fn shortcut_recording() -> bool {
    SHORTCUT_RECORDING.load(std::sync::atomic::Ordering::Relaxed)
}
