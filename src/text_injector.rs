#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct FocusedApp {
    #[serde(rename = "app_id")]
    pub id: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "linux")]
pub(crate) use linux::{atspi_connection, primary_selection, selected_text};
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod desktop;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use desktop::*;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) use desktop::{primary_selection, selected_text};
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use desktop::INJECTED_EVENT_TAG;

mod verification;

pub enum Delivery {
    Pasted,
    Copied,
}

/// Falls back to the clipboard so a dictation is never lost when paste can't land.
pub fn deliver_text(
    injector: &mut Option<TextInjector>,
    text: &str,
    app_id: Option<&str>,
    auto_submit: bool,
) -> anyhow::Result<Delivery> {
    if has_focused_window() {
        let started = std::time::Instant::now();
        let pasted = (|| {
            let injector = match &mut *injector {
                Some(injector) => injector,
                slot => slot.insert(TextInjector::new()?),
            };
            injector.inject_text(text, app_id, auto_submit)
        })();
        match pasted {
            Ok(delivery) => {
                if std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1") {
                    eprintln!("typeaside: inject={}ms", started.elapsed().as_millis());
                }
                return Ok(delivery);
            }
            Err(error) => eprintln!("typeaside: paste failed: {error:#}"),
        }
    }
    set_clipboard(text)?;
    Ok(Delivery::Copied)
}

#[cfg(not(target_os = "macos"))]
fn fallback_app_name(id: &str) -> String {
    id.rsplit('.')
        .next()
        .unwrap_or(id)
        .split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
