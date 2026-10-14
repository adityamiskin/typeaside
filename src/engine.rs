//! Contract between the dictation engine (background threads) and the GPUI app.
//! The UI only depends on what is declared here.

use crate::config::AppConfig;
use crate::{
    audio::AudioRecorder,
    config::RecordingMode,
    hotkeys::{HotkeyEvent, HotkeyService},
    paths::AppPaths,
    text_injector::{TextInjector, focused_app_id},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Dictate,
    /// Voice instruction applied to the selected text.
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Idle,
    Listening(Mode),
    Transcribing,
    Cleaning,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dictation {
    pub at: SystemTime,
    pub raw: String,
    pub cleaned: String,
    /// niri app_id of the window the text was pasted into.
    pub app_id: Option<String>,
    pub audio_ms: u32,
    pub latency_ms: u32,
    #[serde(default)]
    pub mode: Mode,
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
    Reload(Box<AppConfig>),
    /// Re-pastes the latest dictation into the focused app.
    PasteLast,
    /// Streams Event::Level from the mic without recording, for settings and onboarding.
    MicTest(bool),
}

pub struct Engine {
    commands: async_channel::Sender<Command>,
}

impl Engine {
    /// Starts hotkey listening and the pipeline on background threads.
    pub fn spawn(config: AppConfig) -> anyhow::Result<(Self, async_channel::Receiver<Event>)> {
        let paths = AppPaths::discover()?;
        paths.ensure()?;
        let (commands, incoming) = async_channel::unbounded();
        let (events, receiver) = async_channel::unbounded();
        let (tx, rx) = mpsc::channel();
        let commands_tx = tx.clone();
        thread::Builder::new()
            .name("hermes-commands".into())
            .spawn(move || {
                while let Ok(command) = incoming.recv_blocking() {
                    if commands_tx.send(Message::Command(command)).is_err() {
                        return;
                    }
                }
                let _ = commands_tx.send(Message::Shutdown);
            })?;
        thread::Builder::new()
            .name("hermes-engine".into())
            .spawn(move || {
                run(config, paths, tx, rx, events);
            })?;
        Ok((Self { commands }, receiver))
    }

    pub fn send(&self, command: Command) {
        let _ = self.commands.try_send(command);
    }
}

pub fn load_history(paths: &AppPaths) -> Vec<Dictation> {
    let Ok(file) = std::fs::File::open(&paths.history_file) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(std::result::Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

/// Rewrites history atomically, e.g. after the user deletes an entry.
pub fn save_history(paths: &AppPaths, history: &[Dictation]) -> Result<()> {
    let temporary = paths.history_file.with_extension("jsonl.tmp");
    let mut bytes = Vec::new();
    for dictation in history {
        serde_json::to_writer(&mut bytes, dictation)?;
        bytes.push(b'\n');
    }
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, &paths.history_file).context("failed to save history")
}

enum Message {
    Command(Command),
    Hotkey(u64, HotkeyEvent),
    Cleaning(u64),
    Finished(u64, Result<Option<Dictation>>),
    Shutdown,
}

fn phase(events: &async_channel::Sender<Event>, phase: Phase) {
    let _ = events.try_send(Event::Phase(phase));
}

fn report_error(events: &async_channel::Sender<Event>, error: impl std::fmt::Display) {
    phase(events, Phase::Error(error.to_string()));
    phase(events, Phase::Idle);
}

fn open_hotkeys(
    config: &AppConfig,
    tx: &mpsc::Sender<Message>,
    epoch: u64,
    events: &async_channel::Sender<Event>,
) -> Option<HotkeyService> {
    let tx = tx.clone();
    match HotkeyService::start(config, move |event| {
        let _ = tx.send(Message::Hotkey(epoch, event));
    }) {
        Ok(service) => Some(service),
        Err(error) => {
            phase(events, Phase::Error(error.to_string()));
            None
        }
    }
}

fn run(
    mut config: AppConfig,
    paths: AppPaths,
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
    events: async_channel::Sender<Event>,
) {
    phase(&events, Phase::Idle);
    let mut epoch = 0;
    let mut hotkeys = open_hotkeys(&config, &tx, epoch, &events);
    let mut recorder: Option<AudioRecorder> = None;
    let mut generation = 0u64;
    let mut busy = None;
    let mut injector: Option<TextInjector> = None;
    loop {
        if events.is_closed() {
            break;
        }
        let message = match rx.recv_timeout(Duration::from_millis(33)) {
            Ok(message) => message,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(recorder) = &recorder {
                    let _ = events.try_send(Event::Level(recorder.current_level()));
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let command = match message {
            Message::Shutdown => break,
            Message::Command(command) => command,
            Message::Hotkey(source, event) if source == epoch => match event {
                HotkeyEvent::Cancel(true) => Command::Cancel,
                HotkeyEvent::Main(true) => match config.mode {
                    RecordingMode::Hold => Command::Start,
                    RecordingMode::Toggle if recorder.is_some() => Command::Stop,
                    RecordingMode::Toggle => Command::Start,
                },
                HotkeyEvent::Main(false) if config.mode == RecordingMode::Hold => Command::Stop,
                _ => continue,
            },
            Message::Hotkey(_, _) => continue,
            Message::Cleaning(id) => {
                if busy == Some(id) {
                    phase(&events, Phase::Cleaning);
                }
                continue;
            }
            Message::Finished(id, result) => {
                if busy != Some(id) {
                    continue;
                }
                busy = None;
                match result {
                    Ok(Some(mut dictation)) => {
                        let completed = (|| -> Result<()> {
                            let started = Instant::now();
                            let injector = match &mut injector {
                                Some(injector) => injector,
                                slot => slot.insert(TextInjector::new()?),
                            };
                            dictation.app_id = focused_app_id();
                            injector.inject_text(
                                &dictation.cleaned,
                                dictation.app_id.as_deref(),
                                config.auto_submit,
                            )?;
                            dictation.latency_ms = dictation.latency_ms.saturating_add(
                                started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
                            );
                            let mut file = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&paths.history_file)
                                .context("cannot append dictation history")?;
                            let mut line = serde_json::to_vec(&dictation)?;
                            line.push(b'\n');
                            file.write_all(&line)?;
                            Ok(())
                        })();
                        match completed {
                            Ok(()) => {
                                let _ = events.try_send(Event::Done(dictation));
                            }
                            Err(error) => {
                                report_error(&events, error);
                                continue;
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        report_error(&events, error);
                        continue;
                    }
                }
                phase(&events, Phase::Idle);
                continue;
            }
        };
        match command {
            Command::Start if recorder.is_none() && busy.is_none() => {
                match AudioRecorder::start(&config) {
                    Ok(capture) => {
                        recorder = Some(capture);
                        phase(&events, Phase::Listening(Mode::Dictate));
                    }
                    Err(error) => report_error(&events, error),
                }
            }
            Command::Stop => {
                let Some(capture) = recorder.take() else {
                    continue;
                };
                let audio = match capture.stop() {
                    Ok(audio) => audio,
                    Err(error) => {
                        report_error(&events, error);
                        continue;
                    }
                };
                if crate::transcribe::is_tap(&audio) {
                    phase(&events, Phase::Idle);
                    continue;
                }
                if !crate::transcribe::has_speech(&audio) {
                    report_error(&events, "No speech detected");
                    continue;
                }
                generation = generation.wrapping_add(1);
                let id = generation;
                busy = Some(id);
                phase(&events, Phase::Transcribing);
                let config = config.clone();
                let paths = paths.clone();
                let tx = tx.clone();
                let spawned = thread::Builder::new()
                    .name("hermes-transcription".into())
                    .spawn(move || {
                        let result = (|| -> Result<Option<Dictation>> {
                            let started = Instant::now();
                            let raw = crate::transcribe::transcribe(&config, &paths, &audio)?;
                            if raw.is_empty() {
                                return Ok(None);
                            }
                            if config.cleanup.enabled {
                                let _ = tx.send(Message::Cleaning(id));
                            }
                            let cleaned = crate::cleanup::cleanup(&config, &paths, &raw);
                            Ok(Some(Dictation {
                                at: SystemTime::now(),
                                raw,
                                cleaned,
                                app_id: None,
                                audio_ms: (audio.len() as u64 / 16).min(u64::from(u32::MAX)) as u32,
                                latency_ms: started.elapsed().as_millis().min(u128::from(u32::MAX))
                                    as u32,
                                mode: Mode::Dictate,
                            }))
                        })();
                        let _ = tx.send(Message::Finished(id, result));
                    });
                if let Err(error) = spawned {
                    busy = None;
                    report_error(&events, error);
                }
            }
            Command::Cancel => {
                recorder = None;
                busy = None;
                phase(&events, Phase::Idle);
            }
            Command::Reload(next) => {
                recorder = None;
                busy = None;
                drop(hotkeys.take());
                epoch = epoch.wrapping_add(1);
                config = *next;
                phase(&events, Phase::Idle);
                hotkeys = open_hotkeys(&config, &tx, epoch, &events);
            }
            Command::Start | Command::PasteLast | Command::MicTest(_) => {}
        }
    }
}
