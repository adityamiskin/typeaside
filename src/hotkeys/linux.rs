use crate::config::AppConfig;
use anyhow::{Context, Result, bail};
use evdev::{EventSummary, KeyCode};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use std::{
    collections::HashSet,
    io::Write,
    os::{fd::AsFd, unix::net::UnixStream},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

static CANCELABLE: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_cancelable(cancelable: bool, _recording: bool) {
    CANCELABLE.store(cancelable, Ordering::Relaxed);
}

/// Side-specific keys of the latest chord, in press order. The shortcut recorder reads
/// it because toolkit modifier events cannot tell Right Shift from Left Shift.
static CHORD: Mutex<Vec<KeyCode>> = Mutex::new(Vec::new());

pub fn chord() -> String {
    CHORD
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(|code| format!("{code:?}").trim_start_matches("KEY_").to_owned())
        .collect::<Vec<_>>()
        .join("+")
}

#[derive(Debug, Clone, Copy)]
pub enum HotkeyEvent {
    Main(bool),
    Cancel(bool),
    Command(bool),
    Translate(bool),
    Pause(bool),
    Note(bool),
}

pub struct HotkeyService {
    wake: UnixStream,
    thread: Option<JoinHandle<()>>,
}

struct Combo {
    keys: Vec<Vec<KeyCode>>,
    active: bool,
    tap: Option<super::double_tap::DoubleTap>,
    event: fn(bool) -> HotkeyEvent,
}

impl Combo {
    fn parse(value: &str, event: fn(bool) -> HotkeyEvent) -> Result<Self> {
        let double = value.ends_with("*2");
        let value = value.strip_suffix("*2").unwrap_or(value);
        if double && value.contains('+') {
            bail!("double tap requires one key");
        }
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
            tap: double.then(super::double_tap::DoubleTap::default),
            event,
        })
    }
}

impl HotkeyService {
    pub fn start(config: &AppConfig, emit: impl Fn(HotkeyEvent) + Send + 'static) -> Result<Self> {
        let mut combos = vec![
            Combo::parse(&config.hotkey, HotkeyEvent::Main)?,
            Combo::parse("ESCAPE", HotkeyEvent::Cancel)?,
        ];
        for (value, event) in [
            (
                &config.command_hotkey,
                HotkeyEvent::Command as fn(bool) -> HotkeyEvent,
            ),
            (&config.translate_hotkey, HotkeyEvent::Translate),
            (&config.pause_hotkey, HotkeyEvent::Pause),
            (&config.note_hotkey, HotkeyEvent::Note),
        ] {
            if let Some(value) = value {
                combos.push(Combo::parse(value, event)?);
            }
        }
        let initial = keyboards(&combos);
        if initial.is_empty() {
            bail!("no readable keyboards under /dev/input");
        }
        let (mut paths, mut devices): (Vec<_>, Vec<_>) = initial.into_iter().unzip();
        for device in &mut devices {
            device.set_nonblocking(true)?;
        }
        let (wake, reader) = UnixStream::pair()?;
        let thread = thread::Builder::new()
            .name("typeaside-hotkeys".into())
            .spawn(move || {
                let mut pressed = vec![HashSet::new(); devices.len()];
                let mut scanned = Instant::now();
                let mut missing = false;
                loop {
                    let mut fds = vec![PollFd::new(reader.as_fd(), PollFlags::POLLIN)];
                    fds.extend(
                        devices
                            .iter()
                            .map(|device| PollFd::new(device.as_fd(), PollFlags::POLLIN)),
                    );
                    match poll(&mut fds, PollTimeout::from(250u16)) {
                        Err(nix::errno::Errno::EINTR) => continue,
                        Err(error) => {
                            eprintln!("typeaside: keyboard polling failed: {error}");
                            break;
                        },
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
                            paths.swap_remove(index);
                            devices.swap_remove(index);
                            pressed.swap_remove(index);
                            continue;
                        }
                        if !flags.contains(PollFlags::POLLIN) {
                            continue;
                        }
                        let events = match devices[index].fetch_events().map(|events| events.collect::<Vec<_>>()) {
                            Ok(events) => events,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                            Err(error) => {
                                eprintln!("typeaside: keyboard read failed: {error}");
                                paths.swap_remove(index);
                                devices.swap_remove(index);
                                pressed.swap_remove(index);
                                continue;
                            }
                        };
                        for event in events {
                            if let EventSummary::Key(_, code, value) = event.destructure() {
                                match value {
                                    1 => {
                                        let mut chord =
                                            CHORD.lock().unwrap_or_else(|e| e.into_inner());
                                        if pressed.iter().all(HashSet::is_empty) {
                                            chord.clear();
                                        }
                                        if !chord.contains(&code) {
                                            chord.push(code);
                                        }
                                        pressed[index].insert(code);
                                    }
                                    0 => {
                                        pressed[index].remove(&code);
                                    }
                                    _ => continue,
                                }
                                for combo in &mut combos {
                                    if let Some(tap) = &mut combo.tap
                                        && tap.key(combo.keys[0].contains(&code), value == 1, Instant::now()) {
                                            emit((combo.event)(true));
                                            emit((combo.event)(false));
                                    }
                                }
                                update_combos(&mut combos, &pressed, &emit);
                            }
                        }
                    }
                    update_combos(&mut combos, &pressed, &emit);
                    if scanned.elapsed() >= Duration::from_secs(1) {
                        scanned = Instant::now();
                        for (path, device) in keyboards(&combos) {
                            if paths.contains(&path) {
                                continue;
                            }
                            if let Err(error) = device.set_nonblocking(true) {
                                eprintln!("typeaside: cannot watch keyboard {}: {error}", path.display());
                                continue;
                            }
                            paths.push(path);
                            devices.push(device);
                            pressed.push(HashSet::new());
                        }
                    }
                    if devices.is_empty() && !missing {
                        eprintln!("typeaside: no readable keyboards remain; waiting for /dev/input devices");
                    }
                    missing = devices.is_empty();
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

fn keyboards(combos: &[Combo]) -> Vec<(PathBuf, evdev::Device)> {
    evdev::enumerate()
        .filter(|(_, device)| {
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
        .collect()
}

fn update_combos(combos: &mut [Combo], pressed: &[HashSet<KeyCode>], emit: &impl Fn(HotkeyEvent)) {
    for combo in combos {
        if combo.tap.is_some() {
            continue;
        }
        let active = combo.keys.iter().all(|group| {
            group
                .iter()
                .any(|key| pressed.iter().any(|keys| keys.contains(key)))
        });
        if active != combo.active {
            combo.active = active;
            let event = (combo.event)(active);
            if !matches!(event, HotkeyEvent::Cancel(_)) || CANCELABLE.load(Ordering::Relaxed) {
                emit(event);
            }
        }
    }
}
