use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Editability {
    Editable,
    NotEditable,
    #[default]
    Unknown,
}

#[derive(Debug, Default)]
pub(super) struct Focus {
    pub editability: Editability,
    pub identity: String,
    pub value: Option<String>,
    pub caret: Option<i32>,
}

impl Focus {
    pub fn paste_missed(&self, transcript: &str) -> bool {
        self.value.is_some() && self.missed(&capture(), transcript)
    }

    /// Only an unchanged, readable, still-focused field counts as a miss; anything ambiguous is a paste.
    fn missed(&self, after: &Self, transcript: &str) -> bool {
        let Some(value) = &self.value else {
            return false;
        };
        self.editability == Editability::Editable
            && after.editability == Editability::Editable
            && !self.identity.is_empty()
            && self.identity == after.identity
            && self.caret == after.caret
            && !value.contains(transcript.trim())
            && after.value.as_ref() == Some(value)
    }
}

pub(super) fn capture() -> Focus {
    static BUSY: AtomicBool = AtomicBool::new(false);
    if BUSY.swap(true, Ordering::AcqRel) {
        return Focus::default();
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("paste-focus".into())
        .spawn(move || {
            #[cfg(target_os = "linux")]
            let focus = linux::capture();
            #[cfg(target_os = "macos")]
            let focus = macos::capture();
            #[cfg(target_os = "windows")]
            let focus = windows::capture();
            let _ = sender.send(focus.unwrap_or_default());
            BUSY.store(false, Ordering::Release);
        });
    if spawned.is_err() {
        BUSY.store(false, Ordering::Release);
    }
    // A hung accessibility call keeps BUSY set, so later probes fail open instead of piling up threads.
    receiver
        .recv_timeout(Duration::from_millis(65))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(value: &str, caret: Option<i32>) -> Focus {
        Focus {
            editability: Editability::Editable,
            identity: "field".into(),
            value: Some(value.into()),
            caret,
        }
    }

    #[test]
    fn unchanged_field_missed_the_paste() {
        assert!(field("old", Some(3)).missed(&field("old", Some(3)), "new"));
        assert!(field("old", None).missed(&field("old", None), "new"));
    }

    #[test]
    fn ambiguous_or_changed_focus_counts_as_pasted() {
        let before = field("old", Some(3));
        for after in [
            Focus::default(),
            field("old new", Some(7)),
            field("old", Some(4)),
            field("old", None),
            Focus {
                identity: "other".into(),
                ..field("old", Some(3))
            },
            Focus {
                value: None,
                ..field("old", Some(3))
            },
            Focus {
                editability: Editability::Unknown,
                ..field("old", Some(3))
            },
        ] {
            assert!(!before.missed(&after, "new"));
        }
        assert!(!field("new", Some(3)).missed(&field("new", Some(3)), " new "));
        assert!(!Focus::default().missed(&Focus::default(), "new"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "read-only probe of the live desktop accessibility focus"]
    fn live_focus() {
        let started = std::time::Instant::now();
        let focus = capture();
        eprintln!(
            "focus: {:?} identity={} value_chars={:?} caret={:?} elapsed={:?}",
            focus.editability,
            focus.identity,
            focus.value.as_ref().map(|value| value.chars().count()),
            focus.caret,
            started.elapsed()
        );
    }
}
