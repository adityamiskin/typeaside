use crate::config::AppConfig;
use anyhow::{Context, Result, bail};
use evdev::{EventSummary, KeyCode};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use std::{
    collections::HashSet,
    io::Write,
    os::{fd::AsFd, unix::net::UnixStream},
    thread::{self, JoinHandle},
};

#[derive(Debug, Clone, Copy)]
pub enum HotkeyEvent {
    Main(bool),
    Cancel(bool),
    Command(bool),
    Translate(bool),
    PasteLast(bool),
}

pub struct HotkeyService {
    wake: UnixStream,
    thread: Option<JoinHandle<()>>,
}

struct Combo {
    keys: Vec<Vec<KeyCode>>,
    active: bool,
    event: fn(bool) -> HotkeyEvent,
}

impl Combo {
    fn parse(value: &str, event: fn(bool) -> HotkeyEvent) -> Result<Self> {
        let keys = value
            .split('+')
            .map(|part| {
                let upper = part.trim().to_ascii_uppercase();
                let alternatives = match upper.as_str() {
                    "CTRL" | "CONTROL" => vec![KeyCode::KEY_LEFTCTRL, KeyCode::KEY_RIGHTCTRL],
                    "ALT" => vec![KeyCode::KEY_LEFTALT, KeyCode::KEY_RIGHTALT],
                    "SUPER" | "META" => vec![KeyCode::KEY_LEFTMETA, KeyCode::KEY_RIGHTMETA],
                    "SHIFT" => vec![KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_RIGHTSHIFT],
                    "ESCAPE" | "ESC" => vec![KeyCode::KEY_ESC],
                    "RETURN" => vec![KeyCode::KEY_ENTER],
                    _ => vec![
                        format!("KEY_{}", upper.trim_start_matches("KEY_"))
                            .parse::<KeyCode>()
                            .with_context(|| format!("invalid hotkey key: {part}"))?,
                    ],
                };
                Ok(alternatives)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            keys,
            active: false,
            event,
        })
    }
}

impl HotkeyService {
    pub fn start(config: &AppConfig, emit: impl Fn(HotkeyEvent) + Send + 'static) -> Result<Self> {
        let mut combos = vec![Combo::parse(&config.hotkey, HotkeyEvent::Main)?];
        if let Some(cancel) = &config.cancel_hotkey {
            combos.push(Combo::parse(cancel, HotkeyEvent::Cancel)?);
        }
        for (value, event) in [
            (
                &config.command_hotkey,
                HotkeyEvent::Command as fn(bool) -> HotkeyEvent,
            ),
            (&config.translate_hotkey, HotkeyEvent::Translate),
            (
                &config.paste_last_hotkey,
                HotkeyEvent::PasteLast as fn(bool) -> HotkeyEvent,
            ),
        ] {
            if let Some(value) = value {
                combos.push(Combo::parse(value, event)?);
            }
        }
        let mut devices: Vec<_> = evdev::enumerate()
            .map(|(_, device)| device)
            .filter(|device| {
                device.name() != Some("TypeAside paste keyboard")
                    && device.supported_keys().is_some_and(|keys| {
                        keys.contains(KeyCode::KEY_A)
                            && combos.iter().any(|combo| {
                                combo
                                    .keys
                                    .iter()
                                    .all(|group| group.iter().any(|key| keys.contains(*key)))
                            })
                    })
            })
            .collect();
        if devices.is_empty() {
            bail!("no readable keyboards under /dev/input");
        }
        for device in &mut devices {
            device.set_nonblocking(true)?;
        }
        let (wake, reader) = UnixStream::pair()?;
        let thread = thread::Builder::new()
            .name("typeaside-hotkeys".into())
            .spawn(move || {
                let mut pressed = vec![HashSet::new(); devices.len()];
                while !devices.is_empty() {
                    let mut fds = vec![PollFd::new(reader.as_fd(), PollFlags::POLLIN)];
                    fds.extend(
                        devices
                            .iter()
                            .map(|device| PollFd::new(device.as_fd(), PollFlags::POLLIN)),
                    );
                    match poll(&mut fds, PollTimeout::NONE) {
                        Err(nix::errno::Errno::EINTR) => continue,
                        Err(_) => break,
                        Ok(_) => {}
                    }
                    if fds[0].revents().is_some_and(|events| !events.is_empty()) {
                        break;
                    }
                    let ready: Vec<_> = fds
                        .iter()
                        .skip(1)
                        .map(|fd| fd.revents().unwrap_or_else(PollFlags::empty))
                        .collect();
                    drop(fds);
                    for (index, flags) in ready.iter().enumerate().rev() {
                        if flags.intersects(
                            PollFlags::POLLHUP | PollFlags::POLLERR | PollFlags::POLLNVAL,
                        ) {
                            devices.swap_remove(index);
                            pressed.swap_remove(index);
                            continue;
                        }
                        if !flags.contains(PollFlags::POLLIN) {
                            continue;
                        }
                        let Ok(events) = devices[index].fetch_events() else {
                            continue;
                        };
                        for event in events {
                            if let EventSummary::Key(_, code, value) = event.destructure() {
                                match value {
                                    1 => {
                                        pressed[index].insert(code);
                                    }
                                    0 => {
                                        pressed[index].remove(&code);
                                    }
                                    _ => continue,
                                }
                                for combo in &mut combos {
                                    let active = combo.keys.iter().all(|group| {
                                        group.iter().any(|key| {
                                            pressed.iter().any(|keys| keys.contains(key))
                                        })
                                    });
                                    if active != combo.active {
                                        combo.active = active;
                                        emit((combo.event)(active));
                                    }
                                }
                            }
                        }
                    }
                }
            })?;
        Ok(Self {
            wake,
            thread: Some(thread),
        })
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        let _ = self.wake.write_all(&[1]);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
