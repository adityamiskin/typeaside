use crate::engine::Event;

#[cfg(target_os = "macos")]
#[path = "learn/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "learn/windows.rs"]
mod platform;

pub struct Watch {
    stop: std::sync::mpsc::Sender<()>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

/// Starts before the paste: the caret is the only reliable way to find the inserted text
/// when the same words already appear elsewhere in the field.
pub fn watch(inserted: String, events: async_channel::Sender<Event>) -> Option<Watch> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        if inserted.is_empty() {
            return None;
        }
        let (stop, stopped) = std::sync::mpsc::channel();
        let (ready, captured) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("typeaside-learn".into())
            .spawn(move || {
                let Some((element, prefix, suffix)) = capture() else {
                    return;
                };
                let _ = ready.send(());
                observe(element, prefix, suffix, inserted, events, stopped);
            })
            .map_err(|error| eprintln!("typeaside: cannot watch edits: {error}"))
            .ok()?;
        // A hung app can stall accessibility calls; never hold up the paste for one.
        captured
            .recv_timeout(std::time::Duration::from_millis(200))
            .ok()?;
        Some(Watch { stop })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (inserted, events);
        None
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn capture() -> Option<(platform::FocusedElement, String, String)> {
    let element = platform::FocusedElement::capture()?;
    let text = element.value()?;
    let (start, length) = element.selection()?;
    let byte = |utf16: usize| {
        let mut offset = 0;
        for (at, ch) in text.char_indices() {
            if offset == utf16 {
                return Some(at);
            }
            offset += ch.len_utf16();
        }
        (offset == utf16).then_some(text.len())
    };
    let (start, end) = (byte(start)?, byte(start + length)?);
    Some((element, text[..start].into(), text[end..].into()))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn observe(
    element: platform::FocusedElement,
    prefix: String,
    suffix: String,
    inserted: String,
    events: async_channel::Sender<Event>,
    stopped: std::sync::mpsc::Receiver<()>,
) {
    use std::time::{Duration, Instant};

    let mut edited = None;
    let started = Instant::now();
    loop {
        let stopping = !matches!(
            stopped.recv_timeout(Duration::from_millis(500)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        );
        if stopping || started.elapsed() >= Duration::from_secs(120) || events.is_closed() {
            break;
        }
        if !element.is_focused() {
            break;
        }
        let Some(value) = element.value() else { break };
        match value
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix(&suffix))
        {
            Some(value) if edited.is_some() || value == inserted => edited = Some(value.to_owned()),
            // Sending a chat message clears the field, so keep the last edit seen intact.
            None if edited.is_some() => break,
            _ => {}
        }
    }
    if let Some(edited) = edited.filter(|edited| *edited != inserted) {
        let _ = events.try_send(Event::Edited { inserted, edited });
    }
}
