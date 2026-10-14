//! Contract between the dictation engine (background threads) and the GPUI app.
//! The UI only depends on what is declared here.

use crate::config::AppConfig;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Idle,
    Listening,
    Transcribing,
    Cleaning,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Dictation {
    pub at: SystemTime,
    pub raw: String,
    pub cleaned: String,
    /// niri app_id of the window the text was pasted into.
    pub app_id: Option<String>,
    pub audio_ms: u32,
    pub latency_ms: u32,
}

#[derive(Debug, Clone)]
pub enum Event {
    Phase(Phase),
    /// Mic level in 0.0..=1.0, sent at ~30 Hz while listening.
    Level(f32),
    Done(Dictation),
}

#[derive(Debug, Clone)]
pub enum Command {
    Start,
    Stop,
    Cancel,
    Reload(AppConfig),
}

pub struct Engine {
    commands: async_channel::Sender<Command>,
}

impl Engine {
    /// Starts hotkey listening and the pipeline on background threads.
    pub fn spawn(config: AppConfig) -> anyhow::Result<(Self, async_channel::Receiver<Event>)> {
        todo!("core agent")
    }

    pub fn send(&self, command: Command) {
        let _ = self.commands.try_send(command);
    }
}
