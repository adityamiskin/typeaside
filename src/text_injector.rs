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

#[cfg(test)]
mod tests {
    use super::{DeliveryStep::*, delivery_step};

    #[test]
    fn delivery_falls_back_only_when_needed() {
        for (focused, paste, copy, expected) in [
            (true, None, None, Paste),
            (false, None, None, Copy),
            (true, Some(true), None, Pasted),
            (true, Some(false), None, Copy),
            (true, Some(false), Some(true), Copied),
            (false, None, Some(true), Copied),
            (true, Some(false), Some(false), Failed),
            (false, None, Some(false), Failed),
        ] {
            assert_eq!(delivery_step(focused, paste, copy), expected);
        }
    }
}

#[derive(Debug, PartialEq)]
enum DeliveryStep {
    Paste,
    Copy,
    Pasted,
    Copied,
    Failed,
}

fn delivery_step(focused: bool, pasted: Option<bool>, copied: Option<bool>) -> DeliveryStep {
    if focused && pasted.is_none() {
        DeliveryStep::Paste
    } else if pasted == Some(true) {
        DeliveryStep::Pasted
    } else {
        match copied {
            None => DeliveryStep::Copy,
            Some(true) => DeliveryStep::Copied,
            Some(false) => DeliveryStep::Failed,
        }
    }
}

pub enum Delivery {
    Pasted,
    Copied,
}

pub fn deliver_text(
    injector: &mut Option<TextInjector>,
    text: &str,
    app_id: Option<&str>,
    auto_submit: bool,
) -> anyhow::Result<Delivery> {
    let focused = has_focused_window().unwrap_or(true);
    let mut pasted = None;
    let mut copied = None;
    loop {
        match delivery_step(focused, pasted, copied) {
            DeliveryStep::Paste => {
                let started = std::time::Instant::now();
                let result = (|| {
                    let injector = match &mut *injector {
                        Some(injector) => injector,
                        slot => slot.insert(TextInjector::new()?),
                    };
                    injector.inject_text(text, app_id, auto_submit)
                })();
                if let Err(error) = &result {
                    eprintln!("typeaside: paste failed: {error:#}");
                }
                if result.is_ok() && std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1") {
                    eprintln!("typeaside: inject={}ms", started.elapsed().as_millis());
                }
                pasted = Some(result.is_ok());
            }
            DeliveryStep::Copy => {
                let result = set_clipboard(text);
                if let Err(error) = &result {
                    eprintln!("typeaside: clipboard fallback failed: {error:#}");
                }
                copied = Some(result.is_ok());
            }
            DeliveryStep::Pasted => return Ok(Delivery::Pasted),
            DeliveryStep::Copied => return Ok(Delivery::Copied),
            DeliveryStep::Failed => anyhow::bail!("clipboard write failed"),
        }
    }
}
