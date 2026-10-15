#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::primary_selection;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod desktop;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) use desktop::primary_selection;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use desktop::*;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use desktop::INJECTED_EVENT_TAG;
