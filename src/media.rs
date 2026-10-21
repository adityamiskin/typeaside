use crate::cues::{self, Cue};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};
use std::{
    sync::{Mutex, mpsc},
    thread::{self, JoinHandle},
};

#[cfg(target_os = "linux")]
struct Volume(String);

#[cfg(target_os = "linux")]
impl Volume {
    fn duck() -> Option<Self> {
        let output = Command::new("timeout")
            .args(["0.5s", "wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"])
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8(output.stdout).ok()?;
        if text.contains("MUTED") {
            return None;
        }
        let original = text.split_whitespace().nth(1)?.to_owned();
        let volume = original.parse::<f64>().ok()?;
        if !volume.is_finite() || volume < 0. {
            return None;
        }
        let guard = Self(original);
        set_volume(&(volume * 0.3).to_string());
        Some(guard)
    }
}

#[cfg(target_os = "linux")]
fn set_volume(value: &str) {
    let _ = Command::new("timeout")
        .args(["0.5s", "wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", value])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(target_os = "linux")]
impl Drop for Volume {
    fn drop(&mut self) {
        set_volume(&self.0);
    }
}

#[cfg(not(target_os = "linux"))]
struct Volume;

#[cfg(not(target_os = "linux"))]
impl Volume {
    fn duck() -> Option<Self> {
        None
    }
}

#[cfg(target_os = "linux")]
fn bluetooth_output() -> bool {
    Command::new("timeout")
        .args(["0.5s", "wpctl", "inspect", "@DEFAULT_AUDIO_SINK@"])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("bluez_output.")
        })
}

#[cfg(not(target_os = "linux"))]
fn bluetooth_output() -> bool {
    false
}

pub(crate) struct ListeningEffects {
    stop: mpsc::Sender<()>,
    thread: JoinHandle<()>,
}

impl ListeningEffects {
    pub(crate) fn start(sounds: bool, duck: bool) -> Option<Self> {
        if !sounds && !duck {
            return None;
        }
        let (stop, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("typeaside-listening-effects".into())
            .spawn(move || {
                static SERIAL: Mutex<()> = Mutex::new(());
                let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
                // Playback during Bluetooth's A2DP/HFP switch can stall capture. Even
                // silent playback keeps multipoint headphones on this device.
                let sounds = sounds && !bluetooth_output();
                // Keep the codec awake between cues on wired/internal outputs.
                let player = sounds.then(cues::Player::open).and_then(Result::ok);
                if let Some(player) = &player {
                    player.play(Cue::Start);
                }
                if matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                    // Restores the volume when it goes out of scope, before the stop cue.
                    let _volume = if duck { Volume::duck() } else { None };
                    let _ = receiver.recv();
                }
                if let Some(player) = &player {
                    player.play(Cue::Stop);
                }
            })
            .ok()?;
        Some(Self { stop, thread })
    }

    pub(crate) fn finish(self) -> JoinHandle<()> {
        let _ = self.stop.send(());
        self.thread
    }
}
