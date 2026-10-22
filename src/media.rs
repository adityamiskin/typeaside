use crate::config::MediaControl;
use crate::cues::{self, Cue};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

#[cfg(target_os = "linux")]
mod linux;

pub(crate) fn recover() {
    #[cfg(target_os = "linux")]
    linux::recover();
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
    stopped: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl ListeningEffects {
    pub(crate) fn start(sounds: bool, media: MediaControl) -> Option<Self> {
        if !sounds && media == MediaControl::Off {
            return None;
        }
        let stopped = Arc::new(AtomicBool::new(false));
        let stopping = stopped.clone();
        let thread = thread::Builder::new()
            .name("typeaside-listening-effects".into())
            .spawn(move || {
                static SERIAL: Mutex<()> = Mutex::new(());
                let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
                let stopped = || stopping.load(Ordering::Acquire);
                // Playback during Bluetooth's A2DP/HFP switch can stall capture. Even
                // silent playback keeps multipoint headphones on this device.
                let sounds = sounds && !bluetooth_output();
                // Keep the codec awake between cues on wired/internal outputs.
                let player = sounds.then(cues::Player::open).and_then(Result::ok);
                if let Some(player) = &player {
                    player.play(Cue::Start);
                }
                if !stopped() {
                    // Media is restored when these go out of scope, before the stop cue.
                    #[cfg(target_os = "linux")]
                    let _volume = (media == MediaControl::LowerVolume).then(linux::Volume::duck);
                    #[cfg(target_os = "linux")]
                    let _paused =
                        (media == MediaControl::PauseMedia).then(|| linux::Paused::start(stopped));
                    while !stopped() {
                        thread::park();
                    }
                }
                if let Some(player) = &player {
                    player.play(Cue::Stop);
                }
            })
            .ok()?;
        Some(Self { stopped, thread })
    }

    pub(crate) fn finish(self) -> JoinHandle<()> {
        self.stopped.store(true, Ordering::Release);
        self.thread.thread().unpark();
        self.thread
    }
}
