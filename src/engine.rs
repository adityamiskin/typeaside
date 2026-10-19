//! Contract between the dictation engine (background threads) and the GPUI app.
//! The UI only depends on what is declared here.

use crate::{
    audio::AudioRecorder,
    config::{HistoryRetention, RecordingMode},
    hotkeys::{HotkeyEvent, HotkeyService},
    paths::AppPaths,
    text_injector::{FocusedApp, TextInjector, focused_app},
};
use crate::{config::AppConfig, pipeline::Delivery};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command as ProcessCommand, Stdio},
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
    Paused(Mode),
    Transcribing,
    Cleaning,
    Error(String),
    LimitReached { resets_at: i64 },
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
    Usage(crate::account::Usage),
    Copied(String),
    Reply {
        prompt: String,
        text: String,
        open: bool,
    },
    Open(String),
    Notice(String),
    DismissReply,
    FileStarted(String),
    Edited {
        inserted: String,
        edited: String,
    },
    Failed(Vec<FailedRecording>),
}

#[derive(Debug, Clone)]
pub enum Command {
    Start,
    Stop,
    RetryFailed(SystemTime),
    TranscribeFile(PathBuf),
    DiscardFailed(SystemTime),
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

pub fn prune_history(
    history: &mut Vec<Dictation>,
    retention: HistoryRetention,
    now: SystemTime,
) -> bool {
    let days = match retention {
        HistoryRetention::Forever => return false,
        HistoryRetention::Month => 30,
        HistoryRetention::Week => 7,
        HistoryRetention::Off => {
            let changed = !history.is_empty();
            history.clear();
            return changed;
        }
    };
    let cutoff = now.checked_sub(Duration::from_secs(days * 24 * 60 * 60));
    let before = history.len();
    history.retain(|entry| cutoff.is_none_or(|cutoff| entry.at >= cutoff));
    history.len() != before
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

fn failed_path(paths: &AppPaths, at: SystemTime) -> Result<PathBuf> {
    let stamp = at.duration_since(SystemTime::UNIX_EPOCH)?.as_nanos();
    Ok(paths.data_dir.join("failed").join(format!("{stamp}.wav")))
}

fn failed_recordings(paths: &AppPaths, retrying: Option<SystemTime>) -> Vec<FailedRecording> {
    let mut recordings = Vec::new();
    if let Ok(entries) = std::fs::read_dir(paths.data_dir.join("failed")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "wav") {
                continue;
            }
            let Some(stamp) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.parse::<u64>().ok())
            else {
                continue;
            };
            let Some(at) = SystemTime::UNIX_EPOCH.checked_add(Duration::from_nanos(stamp)) else {
                continue;
            };
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            recordings.push(FailedRecording {
                at,
                audio_ms: (metadata.len().saturating_sub(44) / 32).min(u64::from(u32::MAX)) as u32,
                retrying: retrying == Some(at),
            });
        }
    }
    recordings.sort_by_key(|recording| std::cmp::Reverse(recording.at));
    recordings
}

fn save_failed(paths: &AppPaths, audio: &[f32]) -> Result<()> {
    let path = failed_path(paths, SystemTime::now())?;
    std::fs::create_dir_all(paths.data_dir.join("failed"))?;
    let temporary = path.with_extension("wav.tmp");
    std::fs::write(&temporary, crate::transcribe::encode_wav(audio)?)?;
    std::fs::rename(temporary, path)?;
    for recording in failed_recordings(paths, None).into_iter().skip(10) {
        discard_failed(paths, recording.at)?;
    }
    Ok(())
}

fn idle_reload<T>(pending: &mut Option<T>, recording: bool, transcribing: bool) -> Option<T> {
    if recording || transcribing {
        None
    } else {
        pending.take()
    }
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

fn deliver_and_record(
    paths: &AppPaths,
    dictation: &Dictation,
    retention: HistoryRetention,
    events: &async_channel::Sender<Event>,
    deliver: impl FnOnce(),
) {
    deliver();
    if retention != HistoryRetention::Off
        && let Err(error) = append_history(paths, dictation)
    {
        eprintln!("typeaside: cannot save history: {error:#}");
        let _ = events.try_send(Event::Notice("Couldn't save transcript history".into()));
    }
}

fn spawn_dictation_hook(
    command: &str,
    dictation: &Dictation,
) -> std::io::Result<thread::JoinHandle<()>> {
    #[cfg(unix)]
    let mut process = ProcessCommand::new("sh");
    #[cfg(unix)]
    process.arg("-c");
    #[cfg(windows)]
    let mut process = ProcessCommand::new("cmd");
    #[cfg(windows)]
    process.arg("/C");
    let mut child = process
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let input = serde_json::to_vec(dictation)?;
    thread::Builder::new()
        .name("typeaside-dictation-hook".into())
        .spawn(move || {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(&input);
            }
            match child.wait() {
                Ok(status) if !status.success() => {
                    eprintln!("typeaside: dictation hook exited with {status}");
                }
                Err(error) => eprintln!("typeaside: dictation hook failed: {error}"),
                _ => {}
            }
        })
}

fn discard_failed(paths: &AppPaths, at: SystemTime) -> Result<()> {
    match std::fs::remove_file(failed_path(paths, at)?) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result.context("cannot discard failed recording"),
    }
}

enum Message {
    Command(Command),
    Hotkey(u64, HotkeyEvent),
    Cleaning(u64),
    Finished(
        u64,
        Result<(Dictation, Delivery, Option<crate::account::Usage>)>,
    ),
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
    let _ = events.try_send(Event::Failed(failed_recordings(&paths, None)));
    let mut epoch = 0;
    let mut hotkeys = open_hotkeys(&config, &tx, epoch, &events);
    #[cfg(target_os = "linux")]
    if config.auto_learn {
        crate::learn::enable();
    }
    let mut recorder: Option<AudioRecorder> = None;
    let mut stream = None;
    let mut generation = 0u64;
    let mut busy = None;
    let mut pending_audio: Option<Arc<[f32]>> = None;
    let mut retrying = None;
    let mut pending_reload = None;
    let mut injector: Option<TextInjector> = None;
    let mut watch = None;
    let mut context = (Mode::Dictate, None, String::new());
    let mut screen_names = None;
    let mut cursor: Option<mpsc::Receiver<Option<(String, String)>>> = None;
    let mut around: Option<(String, String)> = None;
    let mut last_paste: Option<(Option<FocusedApp>, Instant, char)> = None;
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
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        crate::hotkeys::set_cancelable(
            recorder.is_some() || reply_visible || (cfg!(target_os = "windows") && busy.is_some()),
            recorder.is_some(),
        );
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
                    stream = None;
                    monitor = None;
                    stop_effects(&mut effects, &mut restoring);
                    report_error(&events, error);
                    if config.sounds {
                        crate::cues::play(crate::cues::Cue::Error);
                    }
                }
            }
        }
        let message = if let Some(next) =
            idle_reload(&mut pending_reload, recorder.is_some(), busy.is_some())
        {
            Message::Command(Command::Reload(next))
        } else if recorder.as_ref().is_some_and(AudioRecorder::limit_reached) {
            Message::Command(Command::Stop)
        } else {
            match rx.recv_timeout(Duration::from_millis(33).saturating_sub(tick.elapsed())) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
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
                HotkeyEvent::Pause(true) => {
                    if let Some(capture) = &recorder {
                        let paused = capture.toggle_paused();
                        let _ = events.try_send(Event::Level(0.));
                        phase(
                            &events,
                            if paused {
                                Phase::Paused(context.0)
                            } else {
                                Phase::Listening(context.0)
                            },
                        );
                    }
                    continue;
                }
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
                            phase(
                                &events,
                                if recorder.as_ref().is_some_and(AudioRecorder::is_paused) {
                                    Phase::Paused(mode)
                                } else {
                                    Phase::Listening(mode)
                                },
                            );
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
                let mut usage = None;
                match result {
                    Ok((dictation, _, _)) if dictation.cleaned.is_empty() => {
                        if let Some(at) = retrying {
                            if let Err(error) = discard_failed(&paths, at) {
                                report_error(&events, error);
                            }
                            let _ = events.try_send(Event::Failed(failed_recordings(&paths, None)));
                        }
                    }
                    Ok((dictation, delivery, metered)) => {
                        if dictation.mode == Mode::Dictate {
                            usage = metered;
                        }
                        deliver_and_record(
                            &paths,
                            &dictation,
                            config.history_retention,
                            &events,
                            || {
                                let _ = events.try_send(Event::Done(dictation.clone()));
                                if dictation.mode == Mode::File {
                                    match crate::text_injector::set_clipboard(&dictation.cleaned) {
                                        Ok(()) => {
                                            let _ = events
                                                .try_send(Event::Copied(dictation.cleaned.clone()));
                                        }
                                        Err(error) => report_error(&events, error),
                                    }
                                } else if let Some(at) = retrying {
                                    match crate::text_injector::set_clipboard(&dictation.cleaned) {
                                        Ok(()) => {
                                            let _ = events
                                                .try_send(Event::Copied(dictation.cleaned.clone()));
                                            if let Err(error) = discard_failed(&paths, at) {
                                                report_error(&events, error);
                                            }
                                        }
                                        Err(error) => report_error(&events, error),
                                    }
                                    let _ = events
                                        .try_send(Event::Failed(failed_recordings(&paths, None)));
                                } else if delivery == Delivery::Reply
                                    || (delivery == Delivery::Open && !context.2.is_empty())
                                {
                                    reply_visible = true;
                                    let _ = events.try_send(Event::Reply {
                                        prompt: dictation.raw.clone(),
                                        text: dictation.cleaned.clone(),
                                        open: delivery == Delivery::Open,
                                    });
                                } else if delivery == Delivery::Open {
                                    let _ = events.try_send(Event::Open(dictation.cleaned.clone()));
                                } else {
                                    // Without caret access, assume the caret still follows the last paste in this window.
                                    let (before, after) = match around.take() {
                                        Some((before, after)) => {
                                            (before.chars().last(), after.chars().next())
                                        }
                                        None => (
                                            last_paste
                                                .as_ref()
                                                .filter(|(app, at, _)| {
                                                    !config.auto_submit
                                                        && *app == context.1
                                                        && at.elapsed() < Duration::from_secs(300)
                                                })
                                                .map(|(.., last)| *last),
                                            None,
                                        ),
                                    };
                                    let text = spaced(&dictation.cleaned, before, after);
                                    let delivered = deliver(
                                        &mut injector,
                                        &mut watch,
                                        &text,
                                        dictation.app_id.as_deref(),
                                        &config,
                                        &events,
                                    );
                                    last_paste = if delivered {
                                        text.chars()
                                            .last()
                                            .map(|last| (context.1.clone(), Instant::now(), last))
                                    } else {
                                        None
                                    };
                                }
                            },
                        );
                        if let Some(command) = &config.on_dictation
                            && let Err(error) = spawn_dictation_hook(command, &dictation)
                        {
                            eprintln!("typeaside: cannot run dictation hook: {error}");
                        }
                    }
                    Err(error) => {
                        let limit =
                            error
                                .downcast_ref::<crate::http::HostedError>()
                                .and_then(|error| match error {
                                    crate::http::HostedError::Limit { resets_at } => {
                                        Some(*resets_at)
                                    }
                                    _ => None,
                                });
                        let mut recording_saved = retrying.is_some();
                        if retrying.is_none()
                            && (context.0 == Mode::Dictate || limit.is_some())
                            && let Some(audio) = audio
                        {
                            let saved = save_failed(&paths, &audio);
                            recording_saved = saved.is_ok();
                            if let Err(error) = saved {
                                eprintln!("typeaside: cannot save failed recording: {error:#}");
                            }
                        }
                        let _ = events.try_send(Event::Failed(failed_recordings(&paths, None)));
                        if let Some(resets_at) = limit.filter(|_| recording_saved) {
                            phase(&events, Phase::LimitReached { resets_at });
                            phase(&events, Phase::Idle);
                        } else {
                            report_error(&events, error);
                        }
                        if config.sounds {
                            crate::cues::play(crate::cues::Cue::Error);
                        }
                        continue;
                    }
                }
                phase(&events, Phase::Idle);
                if let Some(usage) = usage {
                    let _ = events.try_send(Event::Usage(usage));
                }
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
                }
                monitor = None;
                stream = (config.transcription == crate::config::Source::Hosted)
                    .then(crate::stream::Session::start)
                    .and_then(Result::ok);
                let capture = match &stream {
                    Some(stream) => AudioRecorder::start_streaming(&config, stream.audio()),
                    None => AudioRecorder::start(&config),
                };
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
                let (cursor_tx, cursor_rx) = mpsc::sync_channel(1);
                cursor = thread::Builder::new()
                    .name("typeaside-cursor".into())
                    .spawn(move || {
                        let _ = cursor_tx.send(crate::learn::cursor());
                    })
                    .is_ok()
                    .then_some(cursor_rx);
                match capture {
                    Ok(capture) => {
                        recorder = Some(capture);
                        let app = focused_app();
                        let selection = if start_mode != Mode::Dictate {
                            crate::text_injector::primary_selection()
                        } else {
                            String::new()
                        };
                        context = (start_mode, app, selection);
                        if config.transcription == crate::config::Source::Local {
                            crate::local::prewarm();
                        }
                        crate::http::prewarm(&config);
                        effects =
                            crate::media::ListeningEffects::start(config.sounds, config.duck_media);
                        phase(&events, Phase::Listening(start_mode));
                    }
                    Err(error) => {
                        stream = None;
                        report_error(&events, error);
                        if config.sounds {
                            crate::cues::play(crate::cues::Cue::Error);
                        }
                    }
                }
            }
            Command::DiscardFailed(at) if busy.is_none() => {
                if let Err(error) = discard_failed(&paths, at) {
                    report_error(&events, error);
                }
                let _ = events.try_send(Event::Failed(failed_recordings(&paths, None)));
            }
            Command::TranscribeFile(path) if busy.is_none() && recorder.is_none() => {
                monitor = None;
                watch = None;
                retrying = None;
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
                                    None,
                                ))
                            });
                        let _ = worker_tx.send(Message::Finished(id, result));
                    });
                if let Err(error) = spawned {
                    let _ = tx.send(Message::Finished(id, Err(error.into())));
                }
            }
            Command::Stop | Command::RetryFailed(_) if busy.is_none() => {
                if recorder.is_some() && matches!(command, Command::RetryFailed(_)) {
                    continue;
                }
                retrying = match command {
                    Command::RetryFailed(at) => Some(at),
                    _ => None,
                };
                let audio = if let Some(at) = retrying {
                    watch = None;
                    let loaded = failed_path(&paths, at)
                        .and_then(|path| Ok(std::fs::read(path)?))
                        .context("cannot read failed recording")
                        .and_then(|wav| crate::transcribe::decode_wav(&wav));
                    match loaded {
                        Ok(audio) => {
                            context = (Mode::Dictate, None, String::new());
                            let _ =
                                events.try_send(Event::Failed(failed_recordings(&paths, retrying)));
                            audio
                        }
                        Err(error) => {
                            report_error(&events, error);
                            let _ = events.try_send(Event::Failed(failed_recordings(&paths, None)));
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
                            stream = None;
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
                        stream = None;
                        Vec::new()
                    } else if tap {
                        stream = None;
                        phase(&events, Phase::Idle);
                        continue;
                    } else if silent {
                        stream = None;
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
                let names = if retrying.is_some() {
                    None
                } else {
                    screen_names.take()
                };
                let pipeline_tx = tx.clone();
                let (mode, app, selection) = context.clone();
                around = cursor.take().and_then(|rx| rx.try_recv().ok()).flatten();
                let caret = around.clone();
                let stream = stream.take();
                let spawned = thread::Builder::new()
                    .name("typeaside-transcription".into())
                    .spawn(move || {
                        let tx = pipeline_tx;
                        let result =
                            (|| -> Result<(Dictation, Delivery, Option<crate::account::Usage>)> {
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
                                    caret.as_ref(),
                                    stream,
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
                                    output.usage,
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
                stream = None;
                monitor = None;
                stop_effects(&mut effects, &mut restoring);
                busy = None;
                pending_audio = None;
                let _ = events.try_send(Event::Failed(failed_recordings(&paths, None)));
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
                if recorder.is_some() || busy.is_some() {
                    pending_reload = Some(next);
                    continue;
                }
                if next.history_retention != config.history_retention {
                    let mut history = load_history(&paths);
                    if prune_history(&mut history, next.history_retention, SystemTime::now())
                        && let Err(error) = save_history(&paths, &history)
                    {
                        report_error(&events, error);
                    }
                }
                monitor = None;
                drop(hotkeys.take());
                epoch = epoch.wrapping_add(1);
                config = *next;
                if !config.auto_learn {
                    watch = None;
                }
                #[cfg(target_os = "linux")]
                if config.auto_learn {
                    crate::learn::enable();
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
            | Command::RetryFailed(_)
            | Command::TranscribeFile(_)
            | Command::DiscardFailed(_)
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

/// Consecutive dictations would otherwise run together ("first.Second").
fn spaced(text: &str, before: Option<char>, after: Option<char>) -> String {
    let lead = before.is_some_and(|c| !c.is_whitespace() && !"([{“‘/@#".contains(c))
        && text.starts_with(char::is_alphanumeric);
    let trail = after.is_some_and(char::is_alphanumeric) && !text.ends_with(char::is_whitespace);
    format!(
        "{}{text}{}",
        if lead { " " } else { "" },
        if trail { " " } else { "" }
    )
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
    // Unknown focus on either side is not evidence the window changed.
    if let (Some(started), Some(now)) = (app_id, focused_app().and_then(|app| app.id))
        && started != now
    {
        return match crate::text_injector::set_clipboard(text) {
            Ok(()) => {
                let _ = events.try_send(Event::Copied(text.to_owned()));
                false
            }
            Err(error) => {
                report_error(events, format!("Couldn't copy transcript: {error}"));
                false
            }
        };
    }
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
            false
        }
        Err(error) => {
            eprintln!("typeaside: delivery failed: {error:#}");
            report_error(
                events,
                if config.history_retention == HistoryRetention::Off {
                    "Couldn't paste. History is off."
                } else {
                    "Couldn't paste. Check Home for the transcript."
                },
            );
            if config.sounds {
                crate::cues::play(crate::cues::Cue::Error);
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dictation, Mode, prune_history, spaced};
    use crate::config::HistoryRetention;
    use std::time::{Duration, SystemTime};

    fn test_paths(name: &str) -> crate::paths::AppPaths {
        let dir = std::env::temp_dir().join(format!("typeaside-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        crate::paths::AppPaths {
            config_file: dir.join("config.json"),
            history_file: dir.join("history.jsonl"),
            config_dir: dir.clone(),
            data_dir: dir,
        }
    }

    #[test]
    fn reload_waits_for_recording_and_transcription_and_uses_latest_config() {
        let mut pending = Some(1);
        assert_eq!(super::idle_reload(&mut pending, true, false), None);
        pending = Some(2);
        assert_eq!(super::idle_reload(&mut pending, false, true), None);
        assert_eq!(super::idle_reload(&mut pending, false, false), Some(2));
        assert_eq!(pending, None);
    }

    #[test]
    fn history_failure_happens_after_delivery_and_emits_a_notice() {
        let paths = test_paths("history-failure");
        std::fs::create_dir_all(&paths.history_file).unwrap();
        let dictation = Dictation {
            at: SystemTime::now(),
            raw: "test".into(),
            cleaned: "test".into(),
            app_id: None,
            audio_ms: 1000,
            latency_ms: 1,
            mode: Mode::Dictate,
        };
        let (tx, rx) = async_channel::unbounded();
        let mut delivered = false;
        super::deliver_and_record(&paths, &dictation, HistoryRetention::Forever, &tx, || {
            assert!(rx.is_empty());
            delivered = true;
        });
        assert!(delivered);
        assert!(matches!(rx.try_recv(), Ok(super::Event::Notice(_))));
        std::fs::remove_dir_all(paths.data_dir).unwrap();
    }

    #[test]
    fn failed_recordings_keep_ten_newest_and_discard_one_item() {
        let paths = test_paths("failed-queue");
        std::fs::create_dir_all(paths.data_dir.join("failed")).unwrap();
        for stamp in 1..=12 {
            let at = SystemTime::UNIX_EPOCH + Duration::from_nanos(stamp);
            std::fs::write(super::failed_path(&paths, at).unwrap(), [0; 44]).unwrap();
        }
        super::save_failed(&paths, &[0.0; 16]).unwrap();
        let queue = super::failed_recordings(&paths, None);
        assert_eq!(queue.len(), 10);
        assert_eq!(
            queue.last().unwrap().at,
            SystemTime::UNIX_EPOCH + Duration::from_nanos(4)
        );
        let retry = queue[3].at;
        let queue = super::failed_recordings(&paths, Some(retry));
        assert_eq!(queue.iter().filter(|item| item.retrying).count(), 1);
        super::discard_failed(&paths, retry).unwrap();
        assert_eq!(super::failed_recordings(&paths, None).len(), 9);
        assert!(
            !super::failed_recordings(&paths, None)
                .iter()
                .any(|item| item.at == retry)
        );
        std::fs::remove_dir_all(paths.data_dir).unwrap();
    }

    #[test]
    fn history_retention_keeps_the_cutoff_and_newer_entries() {
        let day = Duration::from_secs(24 * 60 * 60);
        let now = SystemTime::UNIX_EPOCH + day * 100;
        let entries = [
            now - day * 31,
            now - day * 30 - Duration::from_secs(1),
            now - day * 30,
            now - day * 7 - Duration::from_secs(1),
            now - day * 7,
            now,
            now + day,
        ]
        .map(|at| Dictation {
            at,
            raw: String::new(),
            cleaned: String::new(),
            app_id: None,
            audio_ms: 0,
            latency_ms: 0,
            mode: Mode::Dictate,
        });
        for (retention, expected) in [
            (HistoryRetention::Forever, &entries[..]),
            (HistoryRetention::Month, &entries[2..]),
            (HistoryRetention::Week, &entries[4..]),
            (HistoryRetention::Off, &entries[7..]),
        ] {
            let mut history = entries.to_vec();
            assert_eq!(
                prune_history(&mut history, retention, now),
                expected.len() != entries.len()
            );
            assert_eq!(
                history.iter().map(|entry| entry.at).collect::<Vec<_>>(),
                expected.iter().map(|entry| entry.at).collect::<Vec<_>>()
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn dictation_hook_receives_the_dictation_as_json() {
        use super::{Dictation, Mode, spawn_dictation_hook};
        let path = std::env::temp_dir().join(format!("typeaside-hook-{}.json", std::process::id()));
        let dictation = Dictation {
            at: std::time::SystemTime::UNIX_EPOCH,
            raw: "raw speech".into(),
            cleaned: "Quotes \" ' $(exit 1)".into(),
            app_id: Some("test.app".into()),
            audio_ms: 1234,
            latency_ms: 50,
            mode: Mode::Dictate,
        };
        spawn_dictation_hook(&format!("cat > '{}'", path.display()), &dictation)
            .unwrap()
            .join()
            .unwrap();
        let received: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(received, serde_json::to_value(&dictation).unwrap());
    }

    #[test]
    fn dictations_join_the_text_around_the_caret() {
        for (before, text, after, expected) in [
            (Some('.'), "Second one.", None, " Second one."),
            (Some('d'), "and more", Some('x'), " and more "),
            (Some(' '), "Next", None, "Next"),
            (Some('\n'), "Next", None, "Next"),
            (Some('('), "aside", Some(')'), "aside"),
            (Some('@'), "Sara", None, "Sara"),
            (Some('d'), ", then", None, ", then"),
            (None, "First.", None, "First."),
        ] {
            assert_eq!(spaced(text, before, after), expected);
        }
    }
}
