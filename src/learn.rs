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

pub fn watch(inserted: String, events: async_channel::Sender<Event>) -> Option<Watch> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        if inserted.is_empty() {
            return None;
        }
        let (stop, stopped) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("typeaside-learn".into())
            .spawn(move || observe(inserted, events, stopped))
            .map_err(|error| eprintln!("typeaside: cannot watch edits: {error}"))
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
fn middle<'a>(text: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)?.strip_suffix(suffix)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn observe(
    inserted: String,
    events: async_channel::Sender<Event>,
    stopped: std::sync::mpsc::Receiver<()>,
) {
    use std::time::{Duration, Instant};

    let Some(element) = platform::FocusedElement::capture() else {
        return;
    };
    let Some(text) = element.value() else {
        return;
    };
    let Some(offset) = text.rfind(&inserted) else {
        return;
    };
    let prefix = &text[..offset];
    let suffix = &text[offset + inserted.len()..];
    let mut edited = inserted.clone();
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
        // Sending a chat message clears the field, so keep the last edit seen intact.
        let Some(value) = element.value() else { break };
        let Some(value) = middle(&value, prefix, suffix) else {
            break;
        };
        edited = value.to_owned();
    }
    if edited != inserted {
        let _ = events.try_send(Event::Edited { inserted, edited });
    }
}
