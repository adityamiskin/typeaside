//! Contract between the dictation engine (background threads) and the GPUI app.
//! The UI only depends on what is declared here.

use crate::{
    audio::AudioRecorder,
    config::RecordingMode,
    hotkeys::{HotkeyEvent, HotkeyService},
    paths::AppPaths,
    text_injector::{TextInjector, focused_app},
};
use crate::{config::AppConfig, pipeline::Delivery};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant, SystemTime},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Dictate,
    /// Voice instruction or question, optionally applied to selected text.
    Command,
    Translate,
    File,
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
    /// App ID captured when recording started.
    pub app_id: Option<String>,
    pub audio_ms: u32,
    pub latency_ms: u32,
    #[serde(default)]
    pub mode: Mode,
}

#[derive(Debug, Clone)]
pub struct FailedRecording {
    pub at: SystemTime,
    pub audio_ms: u32,
    pub retrying: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Phase(Phase),
    /// Mic level in 0.0..=1.0, sent at ~30 Hz while listening.
    Level(f32),
    Done(Dictation),
    Copied(String),
    Reply {
        prompt: String,
        text: String,
    },
    Open(String),
    DismissReply,
    FileStarted(String),
    Edited {
        inserted: String,
        edited: String,
    },
    Failed(Option<FailedRecording>),
}

#[derive(Debug, Clone)]
pub enum Command {
    Start,
    Stop,
    RetryFailed,
    TranscribeFile(PathBuf),
    DiscardFailed,
    Cancel,
    Reload(Box<AppConfig>),
    Learn(Vec<String>),
    Unlearn(Vec<String>),
    Insert(String),
    DismissReply,
    /// Streams Event::Level from the mic without recording, for settings and onboarding.
    MicTest(bool),
    MicPreview,
}

pub struct Engine {
    commands: async_channel::Sender<Command>,
}

impl Engine {
    /// Starts hotkey listening and the pipeline on background threads.
    pub fn spawn(config: AppConfig) -> anyhow::Result<(Self, async_channel::Receiver<Event>)> {
        let (commands, incoming) = async_channel::unbounded();
        let (events, receiver) = async_channel::unbounded();
        let (tx, rx) = mpsc::channel();
        let commands_tx = tx.clone();
        thread::Builder::new()
            .name("typeaside-commands".into())
            .spawn(move || {
                while let Ok(command) = incoming.recv_blocking() {
                    if commands_tx.send(Message::Command(command)).is_err() {
                        return;
                    }
                }
                let _ = commands_tx.send(Message::Shutdown);
            })?;
        thread::Builder::new()
            .name("typeaside-engine".into())
            .spawn(move || {
                match AppPaths::discover().and_then(|paths| {
                    paths.ensure()?;
                    Ok(paths)
                }) {
                    Ok(paths) => run(config, paths, tx, rx, events),
                    Err(error) => report_error(&events, error),
                }
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

fn failed_recording(paths: &AppPaths, retrying: bool) -> Option<FailedRecording> {
    let metadata = std::fs::metadata(&paths.failed_file).ok()?;
    Some(FailedRecording {
        at: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        audio_ms: (metadata.len().saturating_sub(44) / 32).min(u64::from(u32::MAX)) as u32,
        retrying,
    })
}

fn append_history(paths: &AppPaths, dictation: &Dictation) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.history_file)
        .context("cannot append dictation history")?;
    let mut line = serde_json::to_vec(dictation)?;
    line.push(b'\n');
    file.write_all(&line)?;
    file.sync_data()?;
    Ok(())
}

fn discard_failed(paths: &AppPaths) -> Result<()> {
    match std::fs::remove_file(&paths.failed_file) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result.context("cannot discard failed recording"),
    }
}

enum Message {
    Command(Command),
    Hotkey(u64, HotkeyEvent),
    Cleaning(u64),
    Finished(u64, Result<(Dictation, Delivery)>),
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
            if config.sounds {
                crate::cues::play(crate::cues::Cue::Error);
            }
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
    let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
    let mut epoch = 0;
    let mut hotkeys = open_hotkeys(&config, &tx, epoch, &events);
    let mut recorder: Option<AudioRecorder> = None;
    let mut generation = 0u64;
    let mut busy = None;
    let mut pending_audio: Option<Arc<[f32]>> = None;
    let mut retrying = false;
    let mut injector: Option<TextInjector> = None;
    let mut watch = None;
    let mut context = (Mode::Dictate, None, String::new());
    let mut screen_names = None;
    let mut start_mode = Mode::Dictate;
    let mut held = None;
    let mut monitor: Option<(AudioRecorder, Option<Instant>)> = None;
    let mut effects = None;
    let mut restoring = Vec::new();
    let mut reply_visible = false;
    let mut tick = Instant::now();
    loop {
        if events.is_closed() {
            break;
        }
        #[cfg(target_os = "windows")]
        crate::hotkeys::set_cancelable(recorder.is_some() || busy.is_some() || reply_visible);
        restoring.retain(|thread: &thread::JoinHandle<()>| !thread.is_finished());
        if tick.elapsed() >= Duration::from_millis(33) {
            tick = Instant::now();
            if monitor
                .as_ref()
                .is_some_and(|(_, deadline)| deadline.is_some_and(|until| Instant::now() >= until))
            {
                monitor = None;
            }
            if let Some(capture) = recorder
                .as_ref()
                .or_else(|| monitor.as_ref().map(|(capture, _)| capture))
            {
                let _ = events.try_send(Event::Level(capture.current_level()));
                if let Some(error) = capture.take_error() {
                    recorder = None;
                    monitor = None;
                    stop_effects(&mut effects, &mut restoring);
                    report_error(&events, error);
                    if config.sounds {
                        crate::cues::play(crate::cues::Cue::Error);
                    }
                }
            }
        }
        let message =
            match rx.recv_timeout(Duration::from_millis(33).saturating_sub(tick.elapsed())) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
        let command = match message {
            Message::Shutdown => break,
            Message::Command(command) => {
                if matches!(command, Command::Start) {
                    start_mode = Mode::Dictate;
                }
                command
            }
            Message::Hotkey(source, event) if source == epoch => match event {
                HotkeyEvent::Cancel(true) => Command::Cancel,
                #[cfg(target_os = "macos")]
                HotkeyEvent::Interrupt if recorder.is_some() => Command::Cancel,
                HotkeyEvent::Main(pressed)
                | HotkeyEvent::Command(pressed)
                | HotkeyEvent::Translate(pressed) => {
                    let mode = match event {
                        HotkeyEvent::Command(_) => Mode::Command,
                        HotkeyEvent::Translate(_) => Mode::Translate,
                        _ => Mode::Dictate,
                    };
                    if !pressed && held == Some(mode) {
                        held = None;
                    }
                    if recorder.is_some() && context.0 != mode {
                        // A chord that extends the held shortcut, like Right Alt then Shift.
                        if pressed && held == Some(context.0) {
                            if mode != Mode::Dictate && context.2.is_empty() {
                                context.2 = crate::text_injector::primary_selection();
                            }
                            context.0 = mode;
                            held = Some(mode);
                            phase(&events, Phase::Listening(mode));
                        }
                        continue;
                    }
                    if pressed {
                        held = Some(mode);
                        start_mode = mode;
                        match config.mode {
                            RecordingMode::Hold => Command::Start,
                            RecordingMode::Toggle if recorder.is_some() => Command::Stop,
                            RecordingMode::Toggle => Command::Start,
                        }
                    } else if config.mode == RecordingMode::Hold {
                        Command::Stop
                    } else {
                        continue;
                    }
                }
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
                let audio = pending_audio.take();
                let result = result.and_then(|(dictation, delivery)| {
                    if !dictation.cleaned.is_empty() {
                        append_history(&paths, &dictation)?;
                    }
                    Ok((dictation, delivery))
                });
                match result {
                    Ok((dictation, _)) if dictation.cleaned.is_empty() => {
                        if retrying {
                            if let Err(error) = discard_failed(&paths) {
                                report_error(&events, error);
                            }
                            let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
                        }
                    }
                    Ok((dictation, delivery)) => {
                        let _ = events.try_send(Event::Done(dictation.clone()));
                        if dictation.mode == Mode::File {
                            match crate::text_injector::set_clipboard(&dictation.cleaned) {
                                Ok(()) => {
                                    let _ =
                                        events.try_send(Event::Copied(dictation.cleaned.clone()));
                                }
                                Err(error) => report_error(&events, error),
                            }
                        } else if retrying {
                            if let Err(error) = discard_failed(&paths) {
                                report_error(&events, error);
                            }
                            let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
                        } else if delivery == Delivery::Reply {
                            reply_visible = true;
                            let _ = events.try_send(Event::Reply {
                                prompt: dictation.raw,
                                text: dictation.cleaned.clone(),
                            });
                        } else if delivery == Delivery::Open {
                            let _ = events.try_send(Event::Open(dictation.cleaned.clone()));
                        } else if !deliver(
                            &mut injector,
                            &mut watch,
                            &dictation.cleaned,
                            dictation.app_id.as_deref(),
                            &config,
                            &events,
                        ) {
                            continue;
                        }
                    }
                    Err(error) => {
                        if !retrying
                            && context.0 == Mode::Dictate
                            && let Some(audio) = audio
                        {
                            let saved = crate::transcribe::encode_wav(&audio).and_then(|wav| {
                                let temporary = paths.failed_file.with_extension("wav.tmp");
                                std::fs::write(&temporary, wav)?;
                                std::fs::rename(temporary, &paths.failed_file)?;
                                Ok(())
                            });
                            if let Err(error) = saved {
                                eprintln!("typeaside: cannot save failed recording: {error:#}");
                            }
                        }
                        let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
                        report_error(&events, error);
                        if config.sounds {
                            crate::cues::play(crate::cues::Cue::Error);
                        }
                        continue;
                    }
                }
                phase(&events, Phase::Idle);
                continue;
            }
        };
        match command {
            Command::Start if recorder.is_none() && busy.is_none() => {
                reply_visible = false;
                let _ = events.try_send(Event::DismissReply);
                watch = None;
                if config.transcription == crate::config::Source::Local {
                    if start_mode == Mode::Command {
                        report_error(&events, crate::pipeline::LOCAL_COMMAND);
                        continue;
                    }
                    if start_mode == Mode::Translate {
                        report_error(&events, crate::pipeline::LOCAL_TRANSLATE);
                        continue;
                    }
                    crate::local::prewarm();
                }
                monitor = None;
                screen_names = None;
                if config.screen_context && start_mode == Mode::Dictate {
                    let (names_tx, names_rx) = std::sync::mpsc::sync_channel(1);
                    if thread::Builder::new()
                        .name("typeaside-screen-context".into())
                        .spawn(move || {
                            let _ = names_tx.send(crate::context::names());
                        })
                        .is_ok()
                    {
                        screen_names = Some(names_rx);
                    }
                }
                let app = focused_app();
                let selection = if start_mode != Mode::Dictate {
                    crate::text_injector::primary_selection()
                } else {
                    String::new()
                };
                crate::http::prewarm(&config);
                match AudioRecorder::start(&config) {
                    Ok(capture) => {
                        recorder = Some(capture);
                        context = (start_mode, app, selection);
                        effects =
                            crate::media::ListeningEffects::start(config.sounds, config.duck_media);
                        phase(&events, Phase::Listening(start_mode));
                    }
                    Err(error) => {
                        report_error(&events, error);
                        if config.sounds {
                            crate::cues::play(crate::cues::Cue::Error);
                        }
                    }
                }
            }
            Command::DiscardFailed if busy.is_none() => {
                if let Err(error) = discard_failed(&paths) {
                    report_error(&events, error);
                }
                let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
            }
            Command::TranscribeFile(path) if busy.is_none() && recorder.is_none() => {
                monitor = None;
                watch = None;
                retrying = false;
                generation = generation.wrapping_add(1);
                let id = generation;
                busy = Some(id);
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let _ = events.try_send(Event::FileStarted(name.clone()));
                phase(&events, Phase::Transcribing);
                let config = config.clone();
                let worker_tx = tx.clone();
                let spawned = thread::Builder::new()
                    .name("typeaside-file-transcription".into())
                    .spawn(move || {
                        let started = Instant::now();
                        let result =
                            crate::transcribe::transcribe_file(&config, &path).and_then(|raw| {
                                anyhow::ensure!(!raw.is_empty(), "No speech found");
                                Ok((
                                    Dictation {
                                        at: SystemTime::now(),
                                        cleaned: raw.clone(),
                                        raw,
                                        app_id: Some(name),
                                        audio_ms: 0,
                                        latency_ms: started
                                            .elapsed()
                                            .as_millis()
                                            .min(u128::from(u32::MAX))
                                            as u32,
                                        mode: Mode::File,
                                    },
                                    Delivery::Insert,
                                ))
                            });
                        let _ = worker_tx.send(Message::Finished(id, result));
                    });
                if let Err(error) = spawned {
                    let _ = tx.send(Message::Finished(id, Err(error.into())));
                }
            }
            Command::Stop | Command::RetryFailed if busy.is_none() => {
                retrying = matches!(command, Command::RetryFailed);
                if retrying && recorder.is_some() {
                    continue;
                }
                let audio = if retrying {
                    watch = None;
                    let loaded = std::fs::read(&paths.failed_file)
                        .context("cannot read failed recording")
                        .and_then(|wav| crate::transcribe::decode_wav(&wav));
                    match loaded {
                        Ok(audio) => {
                            context = (Mode::Dictate, None, String::new());
                            let _ = events.try_send(Event::Failed(failed_recording(&paths, true)));
                            audio
                        }
                        Err(error) => {
                            report_error(&events, error);
                            let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
                            if config.sounds {
                                crate::cues::play(crate::cues::Cue::Error);
                            }
                            continue;
                        }
                    }
                } else {
                    let Some(capture) = recorder.take() else {
                        continue;
                    };
                    stop_effects(&mut effects, &mut restoring);
                    let audio = match capture.stop() {
                        Ok(audio) => audio,
                        Err(error) => {
                            report_error(&events, error);
                            if config.sounds {
                                crate::cues::play(crate::cues::Cue::Error);
                            }
                            continue;
                        }
                    };
                    let tap = crate::transcribe::is_tap(&audio);
                    let silent = tap || !crate::transcribe::has_speech(&audio);
                    if silent && context.0 == Mode::Translate && !context.2.trim().is_empty() {
                        // Tapping Translate over a selection translates it in place.
                        Vec::new()
                    } else if tap {
                        phase(&events, Phase::Idle);
                        continue;
                    } else if silent {
                        report_error(&events, "No speech detected");
                        if config.sounds {
                            crate::cues::play(crate::cues::Cue::Error);
                        }
                        continue;
                    } else {
                        audio
                    }
                };
                let audio: Arc<[f32]> = audio.into();
                pending_audio = Some(audio.clone());
                generation = generation.wrapping_add(1);
                let id = generation;
                busy = Some(id);
                phase(&events, Phase::Transcribing);
                let pipeline_config = config.clone();
                let names = if retrying { None } else { screen_names.take() };
                let pipeline_tx = tx.clone();
                let (mode, app, selection) = context.clone();
                let spawned = thread::Builder::new()
                    .name("typeaside-transcription".into())
                    .spawn(move || {
                        let tx = pipeline_tx;
                        let result = (|| -> Result<(Dictation, Delivery)> {
                            let started = Instant::now();
                            let names = names
                                .and_then(|names| names.try_recv().ok())
                                .unwrap_or_default();
                            let output = crate::pipeline::run(
                                &pipeline_config,
                                &audio,
                                app.as_ref(),
                                mode,
                                &selection,
                                &names,
                                || {
                                    let _ = tx.send(Message::Cleaning(id));
                                },
                            )?;
                            Ok((
                                Dictation {
                                    at: SystemTime::now(),
                                    raw: output.raw,
                                    cleaned: output.text,
                                    app_id: app.and_then(|app| app.id),
                                    audio_ms: (audio.len() as u64 / 16).min(u64::from(u32::MAX))
                                        as u32,
                                    latency_ms: started
                                        .elapsed()
                                        .as_millis()
                                        .min(u128::from(u32::MAX))
                                        as u32,
                                    mode,
                                },
                                output.delivery,
                            ))
                        })();
                        let _ = tx.send(Message::Finished(id, result));
                    });
                if let Err(error) = spawned {
                    let _ = tx.send(Message::Finished(id, Err(error.into())));
                }
            }
            Command::Cancel => {
                if reply_visible {
                    reply_visible = false;
                    let _ = events.try_send(Event::DismissReply);
                }
                recorder = None;
                monitor = None;
                stop_effects(&mut effects, &mut restoring);
                busy = None;
                pending_audio = None;
                let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
                phase(&events, Phase::Idle);
            }
            Command::Learn(words) => {
                for word in words {
                    config.add_word(word);
                }
            }
            Command::Unlearn(words) => {
                for word in words {
                    config.remove_word(&word);
                }
            }
            Command::Reload(next) => {
                recorder = None;
                monitor = None;
                stop_effects(&mut effects, &mut restoring);
                busy = None;
                pending_audio = None;
                let _ = events.try_send(Event::Failed(failed_recording(&paths, false)));
                drop(hotkeys.take());
                epoch = epoch.wrapping_add(1);
                config = *next;
                if !config.auto_learn {
                    watch = None;
                }
                phase(&events, Phase::Idle);
                hotkeys = open_hotkeys(&config, &tx, epoch, &events);
            }
            Command::DismissReply => {
                reply_visible = false;
                let _ = events.try_send(Event::DismissReply);
            }
            Command::Insert(text) => {
                deliver(
                    &mut injector,
                    &mut watch,
                    &text,
                    focused_app().and_then(|app| app.id).as_deref(),
                    &config,
                    &events,
                );
            }
            Command::MicTest(false) => {
                monitor = None;
            }
            Command::MicTest(true) | Command::MicPreview if recorder.is_none() => {
                match AudioRecorder::monitor(&config) {
                    Ok(capture) => {
                        monitor = Some((
                            capture,
                            matches!(command, Command::MicTest(true))
                                .then(|| Instant::now() + Duration::from_secs(10)),
                        ))
                    }
                    Err(error) => {
                        report_error(&events, error);
                        if config.sounds {
                            crate::cues::play(crate::cues::Cue::Error);
                        }
                    }
                }
            }
            Command::Start
            | Command::Stop
            | Command::RetryFailed
            | Command::TranscribeFile(_)
            | Command::DiscardFailed
            | Command::MicTest(true)
            | Command::MicPreview => {}
        }
    }
    drop(recorder);
    drop(monitor);
    stop_effects(&mut effects, &mut restoring);
    for thread in restoring {
        let _ = thread.join();
    }
}

fn stop_effects(
    effects: &mut Option<crate::media::ListeningEffects>,
    restoring: &mut Vec<thread::JoinHandle<()>>,
) {
    if let Some(effects) = effects.take() {
        restoring.push(effects.finish());
    }
}

fn deliver(
    injector: &mut Option<TextInjector>,
    watch: &mut Option<crate::learn::Watch>,
    text: &str,
    app_id: Option<&str>,
    config: &AppConfig,
    events: &async_channel::Sender<Event>,
) -> bool {
    use crate::text_injector::{Delivery, deliver_text};
    *watch = None;
    // Auto-submit sends the text before anyone can edit it.
    let pending = (config.auto_learn && !config.auto_submit)
        .then(|| crate::learn::watch(text.to_owned(), events.clone()))
        .flatten();
    match deliver_text(injector, text, app_id, config.auto_submit) {
        Ok(Delivery::Pasted) => {
            *watch = pending;
            true
        }
        Ok(Delivery::Copied) => {
            let _ = events.try_send(Event::Copied(text.to_owned()));
            true
        }
        Err(error) => {
            eprintln!("typeaside: delivery failed: {error:#}");
            report_error(events, "Couldn't paste. Saved to Home.");
            if config.sounds {
                crate::cues::play(crate::cues::Cue::Error);
            }
            false
        }
    }
}
