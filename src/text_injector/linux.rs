use anyhow::{Context, Result, bail};
use atspi::{
    Role, State,
    proxy::{
        accessible::{AccessibleProxy, ObjectRefExt},
        bus::BusProxy,
    },
    zbus::{self, proxy::CacheProperties},
};
use evdev::{AttributeSet, EventType, InputEvent, KeyCode, uinput::VirtualDevice};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};

mod app_name;

pub fn focused_app() -> Option<super::FocusedApp> {
    focused_window().flatten().map(|mut app| {
        app.name = app.id.as_deref().map(app_name::resolve);
        app
    })
}

pub fn has_focused_window() -> bool {
    focused_window().is_none_or(|window| window.is_some())
}

/// Unknown focus stays `None`; only niri can establish that no window has focus.
fn focused_window() -> Option<Option<super::FocusedApp>> {
    niri_focused_window().or_else(|| atspi_focused_app().map(Some))
}

fn niri_focused_window() -> Option<Option<super::FocusedApp>> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-window"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn atspi_focused_app() -> Option<super::FocusedApp> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .ok()?;
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(150), query_atspi_focused_app())
            .await
            .ok()?
            .ok()?
    })
}

pub(crate) async fn atspi_connection() -> Result<&'static zbus::Connection> {
    static CONNECTION: OnceLock<zbus::Connection> = OnceLock::new();
    if let Some(connection) = CONNECTION.get() {
        return Ok(connection);
    }
    let session = zbus::Connection::session().await?;
    let address = BusProxy::new(&session).await?.get_address().await?;
    let connection = zbus::connection::Builder::address(address.as_str())?
        .build()
        .await?;
    Ok(CONNECTION.get_or_init(|| connection))
}

async fn query_atspi_focused_app() -> Result<Option<super::FocusedApp>> {
    let connection = atspi_connection().await?;
    let registry = AccessibleProxy::builder(connection)
        .destination("org.a11y.atspi.Registry")?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    for application in registry.get_children().await? {
        let Ok(application) = application.as_accessible_proxy(connection).await else {
            continue;
        };
        let Ok(windows) = application.get_children().await else {
            continue;
        };
        for window in windows {
            let Ok(window) = window.as_accessible_proxy(connection).await else {
                continue;
            };
            if !window
                .get_state()
                .await
                .is_ok_and(|state| state.contains(State::Active))
            {
                continue;
            }
            if !matches!(
                window.get_role().await,
                Ok(Role::Frame | Role::Window | Role::Dialog)
            ) {
                continue;
            }
            return Ok(Some(super::FocusedApp {
                id: application
                    .name()
                    .await
                    .ok()
                    .filter(|name| !name.is_empty()),
                title: window.name().await.ok().filter(|name| !name.is_empty()),
                name: None,
            }));
        }
    }
    Ok(None)
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
            KeyCode::KEY_C,
            KeyCode::KEY_INSERT,
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
            let mut modifiers = vec![KeyCode::KEY_LEFTCTRL];
            if app_id.is_some_and(is_terminal) {
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
        result.and(restore_clipboard(&previous, text))
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

fn restore_clipboard(previous: &str, transcript: &str) -> Result<()> {
    if read_clipboard().is_ok_and(|current| current.as_deref() == Some(transcript)) {
        set_clipboard(previous)?;
    }
    Ok(())
}

fn key_event(code: KeyCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code.code(), value)
}

/// `None` when the clipboard is empty or holds non-text data, which wl-copy cannot restore.
pub(crate) fn read_clipboard() -> Result<Option<String>> {
    let types = Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .context("cannot run wl-paste")?;
    if !types.status.success() {
        return Ok(None);
    }
    let types = String::from_utf8_lossy(&types.stdout);
    if types
        .lines()
        .any(|line| line == "x-kde-passwordManagerHint")
        || !types.lines().any(|line| line.starts_with("text/"))
    {
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

pub(crate) fn selected_text(app_id: Option<&str>) -> String {
    // A fresh virtual keyboard needs 100 ms before the compositor sees it.
    static INJECTOR: Mutex<Option<TextInjector>> = Mutex::new(None);
    fn copy_selection(app_id: Option<&str>) -> Result<String> {
        let mut injector = INJECTOR.lock().unwrap_or_else(|error| error.into_inner());
        let injector = match &mut *injector {
            Some(injector) => injector,
            slot => slot.insert(TextInjector::new()?),
        };
        let terminal = app_id.is_some_and(is_terminal);
        let previous = read_clipboard()?;
        // Clearing is unreliable on some compositors, so detect the copy as a selection change.
        let mut watch = Command::new("wl-paste")
            .args(["--watch", "echo"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("cannot run wl-paste")?;
        let stdout = watch.stdout.take().context("wl-paste stdout unavailable")?;
        let (changes, changed) = mpsc::channel();
        thread::spawn(move || {
            for _ in BufReader::new(stdout).lines() {
                if changes.send(()).is_err() {
                    break;
                }
            }
        });
        let result = (|| {
            changed
                .recv_timeout(Duration::from_millis(300))
                .context("wl-paste --watch did not start")?;
            if terminal {
                injector.combo(
                    &[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_LEFTSHIFT],
                    KeyCode::KEY_C,
                )?;
            } else {
                injector.combo(&[KeyCode::KEY_LEFTCTRL], KeyCode::KEY_INSERT)?;
            }
            if changed.recv_timeout(Duration::from_millis(120)).is_err() {
                return Ok(String::new());
            }
            let text = read_clipboard()?.unwrap_or_default();
            if let Some(previous) = previous {
                set_clipboard(&previous)?;
            }
            Ok(text)
        })();
        let _ = watch.kill();
        let _ = watch.wait();
        result
    }
    copy_selection(app_id).unwrap_or_else(|error| {
        eprintln!("typeaside: cannot copy selection: {error:#}");
        String::new()
    })
}

/// Held shortcuts would turn a simulated copy into a different chord, so these flows read PRIMARY.
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

fn is_terminal(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    matches!(id.as_str(), "org.gnome.terminal" | "org.gnome.console")
        || [
            "ghostty",
            "kitty",
            "foot",
            "alacritty",
            "wezterm",
            "konsole",
            "gnome-terminal",
            "gnome-terminal-server",
            "ptyxis",
            "kgx",
            "tilix",
            "terminator",
            "xfce4-terminal",
            "blackbox",
        ]
        .iter()
        .any(|name| id.split('.').any(|part| part == *name))
}
