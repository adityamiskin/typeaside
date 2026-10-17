#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;

use super::AppState;
use gpui::{App, Entity};
use std::time::Duration;
use typeaside::engine::Command;

/// Reloads the engine as grants change, so new permissions apply without a restart.
pub fn watch(state: &Entity<AppState>, cx: &mut App) {
    let mut last = statuses();
    if last.into_iter().all(|granted| granted) {
        return;
    }
    let state = state.downgrade();
    cx.spawn(async move |cx| {
        while !last.into_iter().all(|granted| granted) {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let now = statuses();
            if now == last {
                continue;
            }
            last = now;
            let updated = state.update(cx, |s, cx| {
                s.permission_notice = notice();
                s.command(Command::Reload(Box::new(s.config.clone())));
                cx.notify();
            });
            if updated.is_err() {
                break;
            }
        }
    })
    .detach();
}
