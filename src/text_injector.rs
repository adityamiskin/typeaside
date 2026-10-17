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
            Ok(()) => {
                if std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1") {
                    eprintln!("typeaside: inject={}ms", started.elapsed().as_millis());
                }
                return Ok(Delivery::Pasted);
            }
            Err(error) => eprintln!("typeaside: paste failed: {error:#}"),
        }
    }
    set_clipboard(text)?;
    Ok(Delivery::Copied)
}
