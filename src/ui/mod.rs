mod assets;
mod corrections;
mod demo;
mod dictionary;
mod home;
mod instance;
mod motion;
mod onboarding;
mod overlay;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod permissions;
mod settings;
mod snippets;
mod style;
mod theme;
mod tray;
mod update;
mod window;

use gpui::{App, AppContext, QuitMode, WindowHandle};
use typeaside::engine::{Command, Dictation, Engine, Event, FailedRecording, Phase};
use typeaside::{config::AppConfig, credentials, paths::AppPaths};

pub struct AppState {
    account: Option<typeaside::account::Account>,
    account_revision: u64,
    /// Nudges settings sync; bounded so bursts collapse into one pending wake.
    sync: async_channel::Sender<()>,
    sync_focus_at: Option<std::time::Instant>,
    model: typeaside::local::Status,
    model_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    update: Option<typeaside::update::Release>,
    update_status: update::Toast,
    update_dismissed: bool,
    signing_in: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    onboarding_own_key: bool,
    config: AppConfig,
    paths: AppPaths,
    engine: Option<Engine>,
    is_demo: bool,
    key_hint: String,
    editing_key_hint: String,
    devices: Vec<String>,
    history: Vec<Dictation>,
    failed: Option<FailedRecording>,
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
                let mut config = if is_demo {
                    demo::config()
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
                history.sort_by_key(|item| std::cmp::Reverse(item.at));
                Ok::<_, anyhow::Error>((config, history))
            });
            cx.spawn(async move |cx| {
                let loaded = load.await;
                cx.update(move |cx| match loaded {
                    Ok((config, history)) => {
                        trace("config and history ready", started);
                        start_ui(config, paths, history, is_demo, actions, started, cx)
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

fn start_ui(
    config: AppConfig,
    paths: AppPaths,
    history: Vec<Dictation>,
    is_demo: bool,
    actions: Actions,
    started: std::time::Instant,
    cx: &mut App,
) {
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
        model: if is_demo {
            demo::model()
        } else {
            typeaside::local::Status::current()
        },
        model_cancel: None,
        update,
        update_status,
        update_dismissed: false,
        signing_in: None,
        onboarding_own_key: false,
        config,
        paths,
        engine: None,
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
        devices: vec![
            if is_demo && initial_page == window::Page::Onboarding {
                "MacBook Pro Microphone"
            } else {
                "System default"
            }
            .into(),
        ],
        history,
        failed: None,
        phase: Phase::Idle,
        save_error: None,
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
    let mut main_window: Option<gpui::AnyWindowHandle> = None;
    let (sender, actions) = actions;
    #[cfg(target_os = "linux")]
    let tray = cx
        .background_executor()
        .spawn(async move { tray::spawn(sender) });
    #[cfg(not(target_os = "linux"))]
    let tray = tray::spawn(sender);
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
        let config = state.read(cx).config.clone();
        let details = cx.background_executor().spawn(async move {
            let stt = key_hint(&config.stt.base_url);
            let editing = key_hint(&config.cleanup.endpoint.base_url);
            let mut devices = vec!["System default".to_owned()];
            if let Ok(inputs) = typeaside::audio::list_input_devices() {
                devices.extend(inputs.into_iter().map(|input| input.name));
            }
            devices.dedup();
            (stt, editing, devices)
        });
        let details_state = state.clone();
        cx.spawn(async move |cx| {
            let (stt, editing, devices) = details.await;
            details_state.update(cx, |s, cx| {
                s.key_hint = stt;
                s.editing_key_hint = editing;
                s.devices = devices;
                cx.notify();
            });
        })
        .detach();
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
                    s.command(Command::Reload(Box::new(s.config.clone())));
                    if s.mic_testing {
                        s.command(if !s.config.onboarded && s.onboarding_step == onboarding::Step::Microphone {Command::MicPreview} else {Command::MicTest(true)});
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
        while let Ok(event) = events.recv().await {
            cx.update(|cx| match event {
                Event::Level(level) => {
                    state.update(cx, |s, cx| {
                        if s.mic_testing || s.onboarding_step == onboarding::Step::Practice {
                            if s.onboarding_step == onboarding::Step::Microphone && level > 0.25 { s.onboarding_heard = true; }
                            if s.level != level { s.level = level; cx.notify(); }
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
                        if !s.config.auto_learn { return; }
                        let words = typeaside::corrections::learned(&inserted, &edited, s.config.known_words());
                        if words.is_empty() { return; }
                        for word in &words {
                            s.config.add_word(word.clone());
                        }
                        s.save_error = s.config.save(&s.paths).err().map(|error| format!("Could not save learned words: {error}"));
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
                        let updated = surface.is_some_and(|h| h.update(cx, |v, w, cx| v.learned(&learned, w, cx)).is_ok());
                        if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                            let _ = h.update(cx, |v, w, cx| v.learned(&learned, w, cx));
                            surface = Some(h);
                        }
                    }
                }
                Event::Copied(text) => {
                    let updated = surface.is_some_and(|h| h.update(cx, |v, w, cx| v.copied(text.clone(), w, cx)).is_ok());
                    if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                        let _ = h.update(cx, |v, w, cx| v.copied(text, w, cx));
                        surface = Some(h);
                    }
                }
                Event::Done(dictation) => {
                    if state.read(cx).config.transcription == typeaside::config::Source::Hosted {
                        refresh_account(&state, cx);
                    }
                    state.update(cx, |s, cx| {
                        if !s.config.onboarded && s.onboarding_step == onboarding::Step::Practice {
                            s.onboarding_transcript = Some(dictation.cleaned.clone());
                            s.onboarding_raw = Some(dictation.raw.clone());
                        }
                        s.history.insert(0, dictation);
                        cx.notify();
                    });
                    if !state.read(cx).config.onboarded { return; }
                    let updated =
                        surface.is_some_and(|h| h.update(cx, |v, w, cx| v.success(w, cx)).is_ok());
                    if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                        let _ = h.update(cx, |v, w, cx| v.success(w, cx));
                        surface = Some(h);
                    }
                }
                Event::Phase(phase) => {
                    if matches!(&phase, Phase::Error(error) if error.contains("Weekly free limit reached"))
                        && state.read(cx).config.transcription == typeaside::config::Source::Hosted
                    {
                        refresh_account(&state, cx);
                    }
                    state.update(cx, |s, cx| {
                        if !s.config.onboarded && s.onboarding_step == onboarding::Step::Practice && matches!(phase, Phase::Listening(_)) {
                            s.onboarding_transcript = None;
                            s.onboarding_raw = None;
                        }
                        if phase == Phase::Idle { s.file_transcribing = None; }
                        s.phase = phase.clone();
                        cx.notify();
                    });
                    if !state.read(cx).config.onboarded { return; }
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
        if let Ok(account) = task.await {
            state.update(cx, |s, cx| {
                if s.account_revision == revision {
                    s.account = account;
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
