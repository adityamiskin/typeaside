use anyhow::{Context, Result, bail};
use evdev::{AttributeSet, EventType, InputEvent, KeyCode, uinput::VirtualDevice};
use std::{
    io::Write,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn focused_app_id() -> Option<String> {
    focused_window()?.get("app_id")?.as_str().map(str::to_owned)
}

fn focused_window() -> Option<serde_json::Value> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-window"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

pub fn has_focused_window() -> Option<bool> {
    focused_window().map(|window| !window.is_null())
}

pub struct TextInjector {
    device: VirtualDevice,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        let mut keys = AttributeSet::new();
        for key in [
            KeyCode::KEY_LEFTCTRL,
            KeyCode::KEY_LEFTSHIFT,
            KeyCode::KEY_V,
            KeyCode::KEY_ENTER,
        ] {
            keys.insert(key);
        }
        let device = VirtualDevice::builder()?
            .name("TypeAside paste keyboard")
            .with_keys(&keys)?
            .build()
            .context("cannot open /dev/uinput")?;
        // Give the compositor time to discover the virtual keyboard.
        thread::sleep(Duration::from_millis(100));
        Ok(Self { device })
    }

    pub fn inject_text(
        &mut self,
        text: &str,
        app_id: Option<&str>,
        auto_submit: bool,
    ) -> Result<()> {
        if text.trim().is_empty() {
            return Ok(());
        }
        let previous = read_clipboard()?;
        let result = (|| {
            set_clipboard(text)?;
            let deadline = Instant::now() + Duration::from_millis(350);
            loop {
                if read_clipboard().is_ok_and(|value| value.as_deref() == Some(text)) {
                    break;
                }
                if Instant::now() >= deadline {
                    bail!("clipboard did not publish transcript");
                }
                thread::sleep(Duration::from_millis(10));
            }
            let terminal = app_id.is_some_and(|id| {
                let id = id.to_ascii_lowercase();
                id == "org.gnome.terminal"
                    || [
                        "ghostty",
                        "kitty",
                        "foot",
                        "alacritty",
                        "wezterm",
                        "konsole",
                        "gnome-terminal",
                    ]
                    .iter()
                    .any(|name| id.split('.').any(|part| part == *name))
            });
            let mut modifiers = vec![KeyCode::KEY_LEFTCTRL];
            if terminal {
                modifiers.push(KeyCode::KEY_LEFTSHIFT);
            }
            self.combo(&modifiers, KeyCode::KEY_V)?;
            if auto_submit {
                thread::sleep(Duration::from_millis(20));
                self.combo(&[], KeyCode::KEY_ENTER)?;
            }
            Ok(())
        })();
        let Some(previous) = previous else {
            return result;
        };
        thread::sleep(Duration::from_millis(300));
        result.and(set_clipboard(&previous))
    }

    fn combo(&mut self, modifiers: &[KeyCode], key: KeyCode) -> Result<()> {
        let mut result = Ok(());
        for code in modifiers.iter().chain(std::iter::once(&key)) {
            if let Err(error) = self.device.emit(&[key_event(*code, 1)]) {
                result = Err(error);
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        for code in std::iter::once(&key).chain(modifiers.iter().rev()) {
            let released = self.device.emit(&[key_event(*code, 0)]);
            if result.is_ok() {
                result = released;
            }
            thread::sleep(Duration::from_millis(1));
        }
        result.context("failed to inject keyboard shortcut")
    }
}

fn key_event(code: KeyCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code.code(), value)
}

/// `None` when the clipboard is empty or holds non-text data, which wl-copy cannot restore.
fn read_clipboard() -> Result<Option<String>> {
    let types = Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .context("cannot run wl-paste")?;
    if !types.status.success() {
        return Ok(None);
    }
    let types = String::from_utf8_lossy(&types.stdout);
    if !types.lines().any(|line| line.starts_with("text/")) {
        return Ok(None);
    }
    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", "text"])
        .output()?;
    if !output.status.success() {
        bail!("cannot read clipboard text");
    }
    Ok(Some(String::from_utf8(output.stdout)?))
}

pub fn set_clipboard(text: &str) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .args(["--type", "text/plain;charset=utf-8"])
        .stdin(Stdio::piped())
        .spawn()
        .context("cannot run wl-copy")?;
    let write = child
        .stdin
        .take()
        .context("wl-copy stdin unavailable")?
        .write_all(text.as_bytes());
    let status = child.wait()?;
    write?;
    if !status.success() {
        bail!("wl-copy failed");
    }
    Ok(())
}

pub(crate) fn primary_selection() -> String {
    Command::new("timeout")
        .args([
            "0.3s",
            "wl-paste",
            "--primary",
            "--no-newline",
            "--type",
            "text",
        ])
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_default()
}
