use super::{
    Delivery,
    verification::{self, Editability},
};
use anyhow::{Context, Result};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::{thread, time::Duration};

#[cfg(target_os = "macos")]
pub use super::macos::{focused_app, has_focused_window};
#[cfg(target_os = "windows")]
pub use super::windows::{focused_app, has_focused_window};

#[cfg(target_os = "macos")]
pub(crate) const INJECTED_EVENT_TAG: i64 = 0x4845524d4553;

pub struct TextInjector {
    keyboard: Enigo,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        let settings = Settings {
            // Restored physical modifiers must outlive a temporary copy injector.
            #[cfg(target_os = "windows")]
            release_keys_when_dropped: false,
            #[cfg(target_os = "macos")]
            event_source_user_data: Some(INJECTED_EVENT_TAG),
            ..Settings::default()
        };
        Ok(Self {
            keyboard: Enigo::new(&settings)
                .context("cannot initialize keyboard injection; on macOS grant TypeAside Accessibility permission in System Settings")?,
        })
    }

    fn combo(&mut self, key: Key) -> Result<()> {
        let modifier = if cfg!(target_os = "macos") {
            Key::Meta
        } else {
            Key::Control
        };
        self.shortcut(Some(modifier), key)
    }

    fn shortcut(&mut self, modifier: Option<Key>, key: Key) -> Result<()> {
        #[cfg(target_os = "windows")]
        let held: Vec<_> = crate::hotkeys::held_modifiers().collect();
        let result = (|| {
            // Command mode copies while its physical hotkey may still be held.
            #[cfg(target_os = "windows")]
            for code in &held {
                self.keyboard
                    .key(Key::Other(u32::from(*code)), Direction::Release)?;
            }
            if let Some(modifier) = modifier {
                self.keyboard.key(modifier, Direction::Press)?;
            }
            let result = self.keyboard.key(key, Direction::Click);
            let release = modifier.map_or(Ok(()), |modifier| {
                self.keyboard.key(modifier, Direction::Release)
            });
            result.and(release)
        })();
        #[cfg(target_os = "windows")]
        let result = {
            let mut restore = Ok(());
            for code in held {
                if crate::hotkeys::held_modifiers().any(|key| key == code) {
                    let next = self
                        .keyboard
                        .key(Key::Other(u32::from(code)), Direction::Press);
                    restore = restore.and(next);
                }
            }
            result.and(restore)
        };
        result.context("cannot inject keyboard shortcut")
    }

    pub fn inject_text(
        &mut self,
        text: &str,
        _: Option<&str>,
        auto_submit: bool,
    ) -> Result<Delivery> {
        if text.trim().is_empty() {
            if auto_submit {
                self.shortcut(None, Key::Return)?;
            }
            return Ok(Delivery::Pasted);
        }
        let before = verification::capture();
        if before.editability == Editability::NotEditable {
            set_clipboard(text)?;
            return Ok(Delivery::Copied);
        }
        let mut clipboard = Clipboard::new()?;
        let previous = SavedClipboard::read(&mut clipboard)?;
        clipboard.set_text(text)?;
        self.combo(Key::Unicode('v'))?;
        if auto_submit {
            thread::sleep(Duration::from_millis(20));
            self.shortcut(None, Key::Return)?;
        }
        thread::sleep(Duration::from_millis(300));
        // Submitting changes the field anyway, so only plain pastes are checked.
        if !auto_submit && before.paste_missed(text) {
            return Ok(Delivery::Copied);
        }
        // Leave a newer clipboard value alone if the user copied while we were pasting.
        if clipboard.get_text().is_ok_and(|value| value == text) {
            previous.restore(&mut clipboard)?;
        }
        Ok(Delivery::Pasted)
    }
}

enum SavedClipboard {
    Text(String),
    Image(arboard::ImageData<'static>),
    Empty,
}

impl SavedClipboard {
    fn read(clipboard: &mut Clipboard) -> Result<Self> {
        match clipboard.get_text() {
            Ok(text) => return Ok(Self::Text(text)),
            Err(arboard::Error::ContentNotAvailable) => {}
            Err(error) => return Err(error.into()),
        }
        match clipboard.get_image() {
            Ok(image) => Ok(Self::Image(image.to_owned_img())),
            Err(arboard::Error::ContentNotAvailable) => Ok(Self::Empty),
            Err(error) => Err(error.into()),
        }
    }

    fn restore(self, clipboard: &mut Clipboard) -> Result<()> {
        match self {
            Self::Text(text) => clipboard.set_text(text)?,
            Self::Image(image) => clipboard.set_image(image)?,
            Self::Empty => clipboard.clear()?,
        }
        Ok(())
    }
}

pub(crate) fn primary_selection() -> String {
    #[cfg(target_os = "windows")]
    let app_id = focused_app().and_then(|app| app.id);
    #[cfg(target_os = "macos")]
    let app_id: Option<String> = None;
    selected_text(app_id.as_deref())
}

pub(crate) fn selected_text(_app_id: Option<&str>) -> String {
    #[cfg(target_os = "windows")]
    if _app_id.is_some_and(is_terminal) {
        return String::new();
    }
    fn copy_selection() -> Result<String> {
        let mut clipboard = Clipboard::new()?;
        let previous = SavedClipboard::read(&mut clipboard)?;
        clipboard.clear()?;
        let result = (|| {
            TextInjector::new()?.combo(Key::Unicode('c'))?;
            for _ in 0..30 {
                thread::sleep(Duration::from_millis(10));
                match clipboard.get_text() {
                    Ok(text) => return Ok(text),
                    Err(arboard::Error::ContentNotAvailable) => {}
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(String::new())
        })();
        let restored = previous.restore(&mut clipboard);
        result.and_then(|text| restored.map(|()| text))
    }
    copy_selection().unwrap_or_else(|error| {
        eprintln!("typeaside: cannot copy selection: {error:#}");
        String::new()
    })
}

pub(crate) fn read_clipboard() -> Result<Option<String>> {
    Ok(Some(Clipboard::new()?.get_text()?))
}

pub fn set_clipboard(text: &str) -> Result<()> {
    Clipboard::new()?.set_text(text)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_terminal(app_id: &str) -> bool {
    let id = app_id.to_ascii_lowercase();
    matches!(
        id.strip_suffix(".exe").unwrap_or(&id),
        "windowsterminal"
            | "windowsterminalpreview"
            | "wt"
            | "conhost"
            | "openconsole"
            | "cmd"
            | "powershell"
            | "pwsh"
            | "wezterm-gui"
            | "alacritty"
            | "mintty"
            | "bash"
            | "wsl"
            | "ubuntu"
            | "debian"
            | "hyper"
            | "tabby"
            | "ghostty"
    )
}
