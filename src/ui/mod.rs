mod assets;
mod capture;
mod clock;
mod demo;
mod dictionary;
mod home;
pub(crate) mod instance;
mod motion;
mod notes;
mod onboarding;
mod overlay;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod permissions;
mod settings;
mod snippets;
mod style;
mod theme;
mod toast;
mod tray;
mod update;
mod window;

use gpui::{App, AppContext, QuitMode, WindowHandle};
use typeaside::engine::{Command, Dictation, Engine, Event, FailedRecording, Phase};
use typeaside::{
    config::{AppConfig, Source},
    credentials,
    paths::AppPaths,
};

pub struct AppState {
    account: Option<typeaside::account::Account>,
    account_revision: u64,
    /// Nudges settings sync; bounded so bursts collapse into one pending wake.
    sync: async_channel::Sender<()>,
    sync_focus_at: Option<std::time::Instant>,
    models: [typeaside::local::Status; typeaside::local::ModelId::ALL.len()],
    model_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    update: Option<typeaside::update::Release>,
    update_status: update::Toast,
    update_dismissed: bool,
    signing_in: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    onboarding_own_key: bool,
    config: AppConfig,
    paths: AppPaths,
    engine: Option<Engine>,
    pending_commands: Vec<Command>,
    is_demo: bool,
    key_hint: String,
    editing_key_hint: String,
    devices: Vec<String>,
    history: Vec<Dictation>,
    notes: typeaside::notes::Store,
    notes_sync: async_channel::Sender<()>,
    note_draft: String,
    note_source: Option<String>,
    capture_window: Option<WindowHandle<capture::Capture>>,
    failed: Vec<FailedRecording>,
    file_transcribing: Option<String>,
    phase: Phase,
    save_error: Option<String>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    permission_notice: Option<String>,
    #[cfg(target_os = "macos")]
    permission_help: bool,
    trace_started: Option<std::time::Instant>,
    initial_page: window::Page,
    expanded: Option<std::time::SystemTime>,
    search: String,
    mic_testing: bool,
    mic_generation: u64,
    level: f32,
    onboarding_step: onboarding::Step,
    onboarding_steps: Vec<onboarding::Step>,
    onboarding_heard: bool,
    onboarding_raw: Option<String>,
    onboarding_transcript: Option<String>,
}

impl AppState {
    fn persist(&mut self) {
        if self.is_demo {
            return;
        }
        self.save_error = self
            .config
            .save(&self.paths)
            .err()
            .map(|e| format!("Could not save: {e}"));
        if self.save_error.is_none() {
            #[cfg(target_os = "macos")]
            typeaside::hotkeys::globe::follow(&self.config);
            let _ = self.sync.try_send(());
            if let Some(engine) = &self.engine {
                engine.send(Command::Reload(Box::new(self.config.clone())));
            }
        }
    }
    fn command(&self, command: Command) {
        if let Some(engine) = &self.engine {
            engine.send(command);
        }
    }
}

pub fn run(is_demo: bool, hidden: bool, started: std::time::Instant) -> anyhow::Result<()> {
    let paths = AppPaths::discover()?;
    let socket = if is_demo {
        "typeaside-demo.sock"
    } else {
        "typeaside.sock"
    };
    let listener = match instance::claim(socket, !hidden) {
        instance::Claim::Handed => return Ok(()),
        instance::Claim::Primary(listener) => listener,
    };
    let actions = async_channel::unbounded();
    if let Some(listener) = listener {
        instance::serve(listener, actions.0.clone());
    }
    if !hidden {
        let _ = actions.0.try_send(tray::Action::Open);
    }
    gpui_platform::application()
        .with_assets(assets::Assets)
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            trace("application ready", started);
            #[cfg(target_os = "macos")]
            {
                window::show_in_dock(false);
                cx.on_app_quit(|_| async { typeaside::hotkeys::globe::hold(false) })
                    .detach();
            }
            let load_paths = paths.clone();
            let load = cx.background_executor().spawn(async move {
                let (mut config, config_notice) = if is_demo {
                    (demo::config(), None)
                } else {
                    AppConfig::load(&load_paths)?
                };
                // Rewrites the entry so it follows a moved app and replaces older formats.
                if !is_demo
                    && !cfg!(debug_assertions)
                    && config.launch_at_login
                    && let Err(error) = typeaside::autostart::set_enabled(true)
                {
                    eprintln!("typeaside: cannot register login launch: {error:#}");
                }
                if is_demo {
                    match std::env::var("TYPEASIDE_THEME").as_deref() {
                        Ok("light") => config.theme = typeaside::config::Theme::Light,
                        Ok("dark") => config.theme = typeaside::config::Theme::Dark,
                        _ => {}
                    }
                }
                let mut history = if is_demo {
                    demo::history()
                } else {
                    typeaside::engine::load_history(&load_paths)
                };
                if !is_demo
                    && typeaside::engine::prune_history(
                        &mut history,
                        config.history_retention,
                        std::time::SystemTime::now(),
                    )
                {
                    load_paths.ensure()?;
                    typeaside::engine::save_history(&load_paths, &history)?;
                }
                history.sort_by_key(|item| std::cmp::Reverse(item.at));
                Ok::<_, anyhow::Error>(Startup {
                    config,
                    history,
                    notes: if is_demo {
                        demo::notes()
                    } else {
                        typeaside::notes::Store::load(&load_paths)?
                    },
                    notice: config_notice,
                })
            });
            cx.spawn(async move |cx| {
                let loaded = load.await;
                cx.update(move |cx| match loaded {
                    Ok(startup) => {
                        trace("config and history ready", started);
                        if startup.notice.is_some() {
                            let _ = actions.0.try_send(tray::Action::Open);
                        }
                        start_ui(startup, paths, is_demo, actions, started, cx)
                    }
                    Err(error) => {
                        eprintln!("typeaside: cannot load configuration: {error:#}");
                        cx.quit();
                    }
                });
            })
            .detach();
        });
    Ok(())
}
type Actions = (
    async_channel::Sender<tray::Action>,
    async_channel::Receiver<tray::Action>,
);

struct Startup {
    notes: typeaside::notes::Store,
    config: AppConfig,
    history: Vec<Dictation>,
    notice: Option<String>,
}

fn start_ui(
    startup: Startup,
    paths: AppPaths,
    is_demo: bool,
    actions: Actions,
    started: std::time::Instant,
    cx: &mut App,
) {
    let Startup {
        notes,
        config,
        history,
        notice,
    } = startup;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if !is_demo {
        #[cfg(target_os = "macos")]
        typeaside::hotkeys::globe::follow(&config);
        if config.onboarded {
            permissions::request();
        }
    }
    let initial_page = if is_demo {
        window::Page::from_demo()
    } else {
        window::Page::Home
    };
    trace("theme init begin", started);
    theme::init(config.theme, cx);
    trace("theme init end", started);
    let (sync_tx, sync_rx) = async_channel::bounded(1);
    let (notes_tx, notes_rx) = async_channel::bounded(1);
    let (update, update_status) = if is_demo {
        demo::update()
    } else {
        (None, update::Toast::Available)
    };
    let state = cx.new(|_| AppState {
        account: if is_demo { demo::account() } else { None },
        account_revision: 0,
        sync: sync_tx,
        sync_focus_at: None,
        models: typeaside::local::ModelId::ALL.map(|id| {
            if is_demo {
                if id == typeaside::local::ModelId::default() {
                    demo::model()
                } else {
                    typeaside::local::Status::NotInstalled
                }
            } else {
                typeaside::local::Status::current(id)
            }
        }),
        model_cancel: None,
        update,
        update_status,
        update_dismissed: false,
        signing_in: None,
        onboarding_own_key: false,
        config,
        paths,
        engine: None,
        pending_commands: Vec::new(),
        is_demo,
        file_transcribing: None,
        key_hint: if is_demo {
            "Saved in keyring"
        } else {
            "Checking keyring…"
        }
        .into(),
        editing_key_hint: if is_demo {
            "Saved in keyring"
        } else {
            "Checking keyring…"
        }
        .into(),
        devices: if is_demo && initial_page == window::Page::Onboarding {
            vec!["MacBook Pro Microphone".into()]
        } else {
            Vec::new()
        },
        history,
        notes,
        notes_sync: notes_tx,
        note_draft: String::new(),
        note_source: None,
        capture_window: None,
        failed: Vec::new(),
        phase: Phase::Idle,
        save_error: notice.clone(),
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        permission_notice: if is_demo { None } else { permissions::notice() },
        #[cfg(target_os = "macos")]
        permission_help: false,
        trace_started: (std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1")).then_some(started),
        initial_page,
        expanded: None,
        search: String::new(),
        mic_testing: false,
        mic_generation: 0,
        level: if is_demo { 0.7 } else { 0. },
        onboarding_step: onboarding::Step::initial(is_demo),
        onboarding_steps: onboarding::Onboarding::visible_steps(is_demo),
        onboarding_heard: is_demo,
        onboarding_raw: None,
        onboarding_transcript: None,
    });
    if let Some(notice) = notice
        && let Ok(handle) = overlay::open(state.clone(), cx)
    {
        let _ = handle.update(cx, |view, window, cx| view.warning(&notice, window, cx));
    }
    let mut main_window: Option<gpui::AnyWindowHandle> = None;
    let (sender, actions) = actions;
    let paste_available = !is_demo
        && state
            .read(cx)
            .history
            .iter()
            .any(|entry| !entry.cleaned.is_empty());
    let tray_updates = sender.clone();
    #[cfg(target_os = "linux")]
    let tray = cx
        .background_executor()
        .spawn(async move { tray::spawn(sender, paste_available) });
    #[cfg(not(target_os = "linux"))]
    let tray = tray::spawn(sender, paste_available);
    let tray_state = state.clone();
    cx.spawn(async move |cx| {
        #[cfg(target_os = "linux")]
        let _tray = tray
            .await
            .inspect_err(|e| eprintln!("typeaside: tray unavailable: {e:#}"));
        #[cfg(not(target_os = "linux"))]
        let _tray = tray.inspect_err(|e| eprintln!("typeaside: tray unavailable: {e:#}"));
        while let Ok(action) = actions.recv().await {
            cx.update(|cx| match action {
                tray::Action::Quit => cx.quit(),
                tray::Action::PasteAvailable => {
                    if let Ok(tray) = &_tray {
                        tray.enable_paste();
                    }
                }
                tray::Action::Command(command) => {
                    tray_state.update(cx, |s, _| {
                        if let Some(command) = command.engine() {
                            if s.engine.is_some() {
                                s.command(command);
                            } else if !s.is_demo {
                                s.pending_commands.push(command);
                            }
                        }
                    });
                }
                tray::Action::Open => {
                    #[cfg(target_os = "macos")]
                    window::show_in_dock(true);
                    let activated = main_window.is_some_and(|handle| {
                        handle.update(cx, |_, w, _| w.activate_window()).is_ok()
                    });
                    if !activated {
                        match window::open(tray_state.clone(), cx) {
                            Ok(window) => main_window = Some(window),
                            Err(e) => eprintln!("typeaside: cannot open main window: {e:#}"),
                        }
                    }
                }
            });
        }
    })
    .detach();
    if !is_demo {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        permissions::watch(&state, cx);
        update::schedule(&state, cx);
        schedule_settings_sync(&state, sync_rx, cx);
        notes::schedule_sync(&state, notes_rx, cx);
        let config = state.read(cx).config.clone();
        let details = cx.background_executor().spawn(async move {
            (
                key_hint(&config.stt.base_url),
                key_hint(&config.cleanup.endpoint.base_url),
            )
        });
        let details_state = state.clone();
        cx.spawn(async move |cx| {
            let (stt, editing) = details.await;
            details_state.update(cx, |s, cx| {
                s.key_hint = stt;
                s.editing_key_hint = editing;
                cx.notify();
            });
        })
        .detach();
        refresh_devices(&state, cx);
    }
    if !is_demo {
        let telemetry_state = state.clone();
        cx.spawn(async move |cx| {
            let mut delay = std::time::Duration::from_secs(30);
            loop {
                cx.background_executor().timer(delay).await;
                let (config, paths) = cx.update(|cx| {
                    let s = telemetry_state.read(cx);
                    (s.config.clone(), s.paths.clone())
                });
                if config.share_usage {
                    cx.background_executor()
                        .spawn(async move {
                            typeaside::telemetry::send(
                                &config,
                                &typeaside::engine::load_history(&paths),
                            );
                        })
                        .await;
                }
                delay = std::time::Duration::from_secs(6 * 60 * 60);
            }
        })
        .detach();
    }
    let startup_config = state.read(cx).config.clone();
    let startup = cx.background_executor().spawn(async move {
        if is_demo {
            return Ok((None, demo::spawn()));
        }
        let (engine, events) = Engine::spawn(startup_config)?;
        Ok::<_, anyhow::Error>((Some(engine), events))
    });
    cx.spawn(async move |cx| {
        let events = match startup.await {
            Ok((engine, events)) => {
                state.update(cx, |s, cx| {
                    s.engine = engine;
                    for command in std::mem::take(&mut s.pending_commands) {
                        s.command(command);
                    }
                    s.command(Command::Reload(Box::new(s.config.clone())));
                    if s.mic_testing {
                        s.command(
                            if !s.config.onboarded
                                && s.onboarding_step == onboarding::Step::Microphone
                            {
                                Command::MicPreview
                            } else {
                                Command::MicTest(true)
                            },
                        );
                    }
                    cx.notify();
                });
                events
            }
            Err(e) => {
                state.update(cx, |s, cx| {
                    s.save_error = Some(format!("Engine startup failed: {e}"));
                    cx.notify();
                });
                return;
            }
        };
        let mut surface: Option<WindowHandle<overlay::Overlay>> = None;
        let mut answer: Option<WindowHandle<overlay::AnswerCard>> = None;
        while let Ok(event) = events.recv().await {
            cx.update(|cx| match event {
                #[cfg(target_os = "linux")]
                Event::ShowCapture => capture::show(&state, cx),
                Event::Capture { selection, app } => capture::toggle(&state, selection, app, cx),
                Event::CloseCapture => {
                    if let Some(handle) = state.read(cx).capture_window {
                        let _ = handle.update(cx, |view, window, cx| view.close(window, cx));
                    }
                }
                Event::CaptureText(text) => {
                    if let Some(handle) = state.read(cx).capture_window {
                        let _ = handle.update(cx, |view, window, cx| view.insert(text, window, cx));
                    }
                }
                Event::Level(level) => {
                    state.update(cx, |s, cx| {
                        if s.mic_testing || s.onboarding_step == onboarding::Step::Practice {
                            if s.onboarding_step == onboarding::Step::Microphone && level > 0.25 {
                                s.onboarding_heard = true;
                            }
                            if s.level != level {
                                s.level = level;
                                cx.notify();
                            }
                        }
                    });
                    if let Some(surface) = surface {
                        let _ = surface.update(cx, |v, _, cx| v.level(level, cx));
                    }
                }
                Event::FileStarted(name) => {
                    state.update(cx, |s, cx| {
                        s.file_transcribing = Some(name);
                        cx.notify();
                    });
                }
                Event::Failed(failed) => {
                    state.update(cx, |s, cx| {
                        s.failed = failed;
                        cx.notify();
                    });
                }
                Event::Edited { inserted, edited } => {
                    let mut learned = Vec::new();
                    state.update(cx, |s, cx| {
                        if !s.config.auto_learn {
                            return;
                        }
                        let words = typeaside::corrections::learned(
                            &inserted,
                            &edited,
                            s.config.known_words(),
                        );
                        if words.is_empty() {
                            return;
                        }
                        for word in &words {
                            s.config.add_word(word.clone());
                        }
                        s.save_error = s
                            .config
                            .save(&s.paths)
                            .err()
                            .map(|error| format!("Could not save learned words: {error}"));
                        if s.save_error.is_some() {
                            cx.notify();
                            return;
                        }
                        let _ = s.sync.try_send(());
                        s.command(Command::Learn(words.clone()));
                        cx.notify();
                        learned = words;
                    });
                    if !learned.is_empty() {
                        let updated = surface.is_some_and(|h| {
                            h.update(cx, |v, w, cx| v.learned(&learned, w, cx)).is_ok()
                        });
                        if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                            let _ = h.update(cx, |v, w, cx| v.learned(&learned, w, cx));
                            surface = Some(h);
                        }
                    }
                }
                Event::DismissReply => {
                    if let Some(handle) = answer.take() {
                        let _ = handle.update(cx, |_, window, _| {
                            #[cfg(target_os = "linux")]
                            window.set_input_region(Some(&[]));
                            window.remove_window();
                        });
                    }
                }
                Event::Reply { prompt, text, open } => {
                    if let Some(handle) = answer.take() {
                        let _ = handle.update(cx, |_, window, _| window.remove_window());
                    }
                    if let Some(surface) = surface {
                        let _ = surface.update(cx, |view, window, cx| view.exit(window, cx));
                    }
                    match overlay::open_answer(state.clone(), prompt, text, open, cx) {
                        Ok(handle) => answer = Some(handle),
                        Err(error) => state.update(cx, |state, cx| {
                            state.command(Command::DismissReply);
                            state.save_error = Some(format!("Could not show answer: {error}"));
                            cx.notify();
                        }),
                    }
                }
                Event::Open(url) => {
                    cx.open_url(&url);
                    let updated = surface
                        .is_some_and(|h| h.update(cx, |v, w, cx| v.opened(&url, w, cx)).is_ok());
                    if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                        let _ = h.update(cx, |v, w, cx| v.opened(&url, w, cx));
                        surface = Some(h);
                    }
                }
                Event::Notice(message) => {
                    state.update(cx, |s, cx| {
                        s.save_error = Some(message.clone());
                        cx.notify();
                    });
                    let updated = surface.is_some_and(|h| {
                        h.update(cx, |v, w, cx| v.warning(&message, w, cx)).is_ok()
                    });
                    if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                        let _ = h.update(cx, |v, w, cx| v.warning(&message, w, cx));
                        surface = Some(h);
                    }
                }
                Event::Copied(text) => {
                    let updated = surface.is_some_and(|h| {
                        h.update(cx, |v, w, cx| v.copied(text.clone(), w, cx))
                            .is_ok()
                    });
                    if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                        let _ = h.update(cx, |v, w, cx| v.copied(text, w, cx));
                        surface = Some(h);
                    }
                }
                Event::Usage(usage) => {
                    let warning = state.update(cx, |s, cx| {
                        let Some(account) = &mut s.account else {
                            return None;
                        };
                        account.usage = usage;
                        cx.notify();
                        let usage = &account.usage;
                        let cap = usage.cap?;
                        (account.plan == typeaside::account::Plan::Free
                            && s.config.transcription == Source::Hosted
                            && usage.words > cap * 80 / 100
                            && s.config.usage_notice_week != Some(usage.resets_at))
                        .then(|| usage.clone())
                    });
                    if let Some(usage) = warning {
                        let updated = surface.is_some_and(|h| {
                            h.update(cx, |v, w, cx| v.usage_warning(&usage, w, cx))
                                .is_ok()
                        });
                        let shown = updated || {
                            if let Ok(h) = overlay::open(state.clone(), cx) {
                                let _ = h.update(cx, |v, w, cx| v.usage_warning(&usage, w, cx));
                                surface = Some(h);
                                true
                            } else {
                                false
                            }
                        };
                        if shown {
                            state.update(cx, |s, _| {
                                s.config.usage_notice_week = Some(usage.resets_at);
                                s.persist();
                            });
                        }
                    }
                }
                Event::Done(dictation) => {
                    if !is_demo && !dictation.cleaned.is_empty() {
                        let _ = tray_updates.try_send(tray::Action::PasteAvailable);
                    }
                    if state.read(cx).config.transcription == Source::Hosted {
                        refresh_account(&state, cx);
                    }
                    state.update(cx, |s, cx| {
                        if !s.config.onboarded && s.onboarding_step == onboarding::Step::Practice {
                            s.onboarding_transcript = Some(dictation.cleaned.clone());
                            s.onboarding_raw = Some(dictation.raw.clone());
                        }
                        if s.config.history_retention != typeaside::config::HistoryRetention::Off {
                            s.history.insert(0, dictation);
                            typeaside::engine::prune_history(
                                &mut s.history,
                                s.config.history_retention,
                                std::time::SystemTime::now(),
                            );
                        }
                        cx.notify();
                    });
                    if !state.read(cx).config.onboarded {
                        return;
                    }
                    let updated =
                        surface.is_some_and(|h| h.update(cx, |v, w, cx| v.success(w, cx)).is_ok());
                    if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                        let _ = h.update(cx, |v, w, cx| v.success(w, cx));
                        surface = Some(h);
                    }
                }
                Event::Phase(phase) => {
                    if matches!(&phase, Phase::LimitReached { .. })
                        && state.read(cx).config.transcription == Source::Hosted
                    {
                        refresh_account(&state, cx);
                    }
                    state.update(cx, |s, cx| {
                        if !s.config.onboarded
                            && s.onboarding_step == onboarding::Step::Practice
                            && matches!(phase, Phase::Listening(_))
                        {
                            s.onboarding_transcript = None;
                            s.onboarding_raw = None;
                        }
                        if phase == Phase::Idle {
                            s.file_transcribing = None;
                        }
                        s.phase = phase.clone();
                        cx.notify();
                    });
                    if !state.read(cx).config.onboarded {
                        return;
                    }
                    let updated = surface.is_some_and(|h| {
                        h.update(cx, |v, w, cx| v.phase(phase.clone(), w, cx))
                            .is_ok()
                    });
                    if !updated
                        && phase != Phase::Idle
                        && let Ok(h) = overlay::open(state.clone(), cx)
                    {
                        let _ = h.update(cx, |v, w, cx| v.phase(phase, w, cx));
                        surface = Some(h);
                    }
                }
            });
        }
    })
    .detach();
}

fn key_hint(endpoint: &str) -> String {
    credentials::endpoint_host(endpoint)
        .ok()
        .and_then(|host| credentials::get_credential(&host).ok().flatten())
        .map_or_else(|| "Enter API key".into(), |_| "Saved in keyring".into())
}

fn trace(stage: &str, started: std::time::Instant) {
    if std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1") {
        eprintln!(
            "typeaside startup {stage}: {:.3} ms",
            started.elapsed().as_secs_f64() * 1000.
        );
    }
}

/// Re-enumerated when Settings opens so docked or unplugged mics show up without polling.
fn refresh_devices(state: &gpui::Entity<AppState>, cx: &mut App) {
    if state.read(cx).is_demo {
        return;
    }
    let devices = cx.background_executor().spawn(async {
        let mut names: Vec<_> = typeaside::audio::list_input_devices()
            .unwrap_or_default()
            .into_iter()
            .map(|input| input.name)
            .collect();
        names.dedup();
        names
    });
    let state = state.clone();
    cx.spawn(async move |cx| {
        let devices = devices.await;
        state.update(cx, |s, cx| {
            if s.devices != devices {
                s.devices = devices;
                cx.notify();
            }
        });
    })
    .detach();
}

fn refresh_account(state: &gpui::Entity<AppState>, cx: &mut App) {
    if state.read(cx).is_demo {
        return;
    }
    let revision = state.update(cx, |s, _| {
        s.account_revision += 1;
        s.account_revision
    });
    let state = state.clone();
    let task = cx
        .background_executor()
        .spawn(async { typeaside::account::me() });
    cx.spawn(async move |cx| {
        if let Ok(mut account) = task.await {
            state.update(cx, |s, cx| {
                if s.account_revision == revision {
                    if account.is_some() && s.config.transcription != Source::Hosted {
                        s.config.transcription = Source::Hosted;
                        s.persist();
                    }
                    if let (Some(current), Some(next)) = (&s.account, &mut account)
                        && current.usage.resets_at == next.usage.resets_at
                    {
                        next.usage.words = next.usage.words.max(current.usage.words);
                    }
                    s.account = account;
                    let _ = s.notes_sync.try_send(());
                    cx.notify();
                }
            });
        }
    })
    .detach();
}

fn schedule_settings_sync(
    state: &gpui::Entity<AppState>,
    wake: async_channel::Receiver<()>,
    cx: &mut App,
) {
    use std::time::Duration;
    use typeaside::settings_sync::{Fields, SyncState};
    let ticker = state.read(cx).sync.clone();
    cx.background_executor()
        .spawn({
            let executor = cx.background_executor().clone();
            async move {
                while !ticker.is_closed() {
                    let _ = ticker.try_send(());
                    executor.timer(Duration::from_secs(15 * 60)).await;
                }
            }
        })
        .detach();
    let state = state.downgrade();
    cx.spawn(async move |cx| {
        while wake.recv().await.is_ok() {
            // Wait for edits to settle so typing a dictionary word sends one request.
            loop {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                if wake.try_recv().is_err() {
                    break;
                }
            }
            let Some(state) = state.upgrade() else { break };
            let (config, revision) = cx.update(|cx| {
                let s = state.read(cx);
                (s.config.clone(), s.account_revision)
            });
            let fields = Fields::from_config(&config);
            let result = cx
                .background_executor()
                .spawn(async move {
                    let url = typeaside::account::hosted_url();
                    let host = credentials::endpoint_host(&url)?;
                    let Some(token) = credentials::get_credential(&host)? else {
                        return Ok(None);
                    };
                    typeaside::settings_sync::synchronize(&config, &url, &token).map(Some)
                })
                .await;
            let Ok(Some(remote)) = result else { continue };
            state.update(cx, |s, cx| {
                if s.account_revision != revision || Fields::from_config(&s.config) != fields {
                    return;
                }
                let synced = Some(SyncState {
                    updated_at: remote.updated_at,
                    base: remote.fields.clone(),
                });
                if s.config.settings_sync == synced && remote.fields == fields {
                    return;
                }
                s.config.settings_sync = synced;
                remote.fields.apply(&mut s.config);
                s.persist();
                cx.notify();
            });
        }
    })
    .detach();
}
