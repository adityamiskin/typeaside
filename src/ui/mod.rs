mod assets;
mod demo;
mod dictionary;
mod home;
mod instance;
mod motion;
mod overlay;
#[cfg(target_os = "macos")]
mod permissions;
mod settings;
mod snippets;
mod style;
mod theme;
mod tray;
mod window;

use gpui::{App, AppContext, QuitMode, WindowHandle};
use hermes::engine::{Command, Dictation, Engine, Event, FailedRecording, Phase};
use hermes::{config::AppConfig, credentials, paths::AppPaths};

pub struct AppState {
    account: Option<hermes::account::Account>,
    account_revision: u64,
    update: Option<hermes::update::Release>,
    update_status: hermes::update::Status,
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
    phase: Phase,
    save_error: Option<String>,
    #[cfg(target_os = "macos")]
    permission_notice: Option<String>,
    trace_started: Option<std::time::Instant>,
    initial_page: window::Page,
    expanded: Option<std::time::SystemTime>,
    search: String,
    mic_testing: bool,
    mic_generation: u64,
    level: f32,
    onboarding_step: usize,
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
        if self.save_error.is_none()
            && let Some(engine) = &self.engine
        {
            engine.send(Command::Reload(Box::new(self.config.clone())));
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
        "hermes-demo.sock"
    } else {
        "hermes.sock"
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
            if !is_demo {
                permissions::request();
            }
            let load_paths = paths.clone();
            let load = cx.background_executor().spawn(async move {
                let mut config = if is_demo {
                    demo::config()
                } else {
                    AppConfig::load(&load_paths)?
                };
                if is_demo {
                    match std::env::var("HERMES_THEME").as_deref() {
                        Ok("light") => config.theme = hermes::config::Theme::Light,
                        Ok("dark") => config.theme = hermes::config::Theme::Dark,
                        _ => {}
                    }
                }
                let mut history = if is_demo {
                    demo::history()
                } else {
                    hermes::engine::load_history(&load_paths)
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
                        eprintln!("hermes: cannot load configuration: {error:#}");
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
    let initial_page = if is_demo {
        window::Page::from_demo()
    } else {
        window::Page::Home
    };
    trace("theme init begin", started);
    theme::init(config.theme, cx);
    trace("theme init end", started);
    let state = cx.new(|_| AppState {
        account: if is_demo { demo::account() } else { None },
        account_revision: 0,
        update: None,
        update_status: hermes::update::Status::Available,
        signing_in: None,
        onboarding_own_key: false,
        config,
        paths,
        engine: None,
        is_demo,
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
        devices: vec!["System default".into()],
        history,
        failed: None,
        phase: Phase::Idle,
        save_error: None,
        #[cfg(target_os = "macos")]
        permission_notice: if is_demo { None } else { permissions::notice() },
        trace_started: (std::env::var("HERMES_TRACE").as_deref() == Ok("1")).then_some(started),
        initial_page,
        expanded: None,
        search: String::new(),
        mic_testing: false,
        mic_generation: 0,
        level: if is_demo { 0.7 } else { 0. },
        onboarding_step: if initial_page == window::Page::Onboarding {
            std::env::var("HERMES_DEMO_STEP")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|step| (1..=3).contains(step))
                .unwrap_or(2)
                - 1
        } else {
            0
        },
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
            .inspect_err(|e| eprintln!("hermes: tray unavailable: {e:#}"));
        #[cfg(not(target_os = "linux"))]
        let _tray = tray.inspect_err(|e| eprintln!("hermes: tray unavailable: {e:#}"));
        while let Ok(action) = actions.recv().await {
            cx.update(|cx| match action {
                tray::Action::Quit => cx.quit(),
                tray::Action::Open => {
                    let activated = main_window.is_some_and(|handle| {
                        handle.update(cx, |_, w, _| w.activate_window()).is_ok()
                    });
                    if !activated {
                        match window::open(tray_state.clone(), cx) {
                            Ok(window) => main_window = Some(window),
                            Err(e) => eprintln!("hermes: cannot open main window: {e:#}"),
                        }
                    }
                }
            });
        }
    })
    .detach();
    if !is_demo {
        schedule_updates(&state, cx);
        let config = state.read(cx).config.clone();
        let details = cx.background_executor().spawn(async move {
            let stt = key_hint(&config.stt.base_url);
            let editing = key_hint(&config.cleanup.endpoint.base_url);
            let mut devices = vec!["System default".to_owned()];
            if let Ok(inputs) = hermes::audio::list_input_devices() {
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
                        s.command(Command::MicTest(true));
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
                        if s.mic_testing {
                            s.level = level;
                            cx.notify();
                        }
                    });
                    if let Some(surface) = surface {
                        let _ = surface.update(cx, |v, _, cx| v.level(level, cx));
                    }
                }
                Event::Failed(failed) => {
                    state.update(cx, |s, cx| {
                        s.failed = failed;
                        cx.notify();
                    });
                }
                Event::Done(dictation) => {
                    if state.read(cx).config.transcription == hermes::config::Source::Hosted {
                        refresh_account(&state, cx);
                    }
                    state.update(cx, |s, cx| {
                        if !s.config.onboarded && s.onboarding_step == 2 {
                            s.onboarding_transcript = Some(dictation.cleaned.clone());
                        }
                        s.history.insert(0, dictation);
                        cx.notify();
                    });
                    let updated =
                        surface.is_some_and(|h| h.update(cx, |v, w, cx| v.success(w, cx)).is_ok());
                    if !updated && let Ok(h) = overlay::open(state.clone(), cx) {
                        let _ = h.update(cx, |v, w, cx| v.success(w, cx));
                        surface = Some(h);
                    }
                }
                Event::Phase(phase) => {
                    if matches!(&phase, Phase::Error(_))
                        && state.read(cx).config.transcription == hermes::config::Source::Hosted
                    {
                        refresh_account(&state, cx);
                    }
                    state.update(cx, |s, cx| {
                        s.phase = phase.clone();
                        cx.notify();
                    });
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
    if std::env::var("HERMES_TRACE").as_deref() == Ok("1") {
        eprintln!(
            "hermes startup {stage}: {:.3} ms",
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
        .spawn(async { hermes::account::me() });
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

fn schedule_updates(state: &gpui::Entity<AppState>, cx: &mut App) {
    let state = state.downgrade();
    cx.spawn(async move |cx| {
        let mut delay = std::time::Duration::from_secs(10);
        loop {
            cx.background_executor().timer(delay).await;
            let result = cx
                .background_executor()
                .spawn(async { hermes::update::check() })
                .await;
            let Some(state) = state.upgrade() else { break };
            match result {
                Ok(release) => state.update(cx, |s, cx| {
                    if s.update_status != hermes::update::Status::Installing {
                        s.update = release;
                        s.update_status = hermes::update::Status::Available;
                        cx.notify();
                    }
                }),
                Err(error) => eprintln!("hermes: update check failed: {error:#}"),
            }
            delay = std::time::Duration::from_secs(6 * 60 * 60);
        }
    })
    .detach();
}

fn install_update(state: &gpui::Entity<AppState>, cx: &mut App) {
    let current = state.read(cx);
    if current.is_demo || current.update_status == hermes::update::Status::Installing {
        return;
    }
    if current.update_status == hermes::update::Status::Failed {
        cx.background_executor()
            .spawn(async {
                let _ = hermes::account::open(format!("{}/download", hermes::account::web_url()));
            })
            .detach();
        return;
    }
    let Some(release) = current.update.clone() else {
        return;
    };
    state.update(cx, |s, cx| {
        s.update_status = hermes::update::Status::Installing;
        cx.notify();
    });
    let state = state.clone();
    let task = cx
        .background_executor()
        .spawn(async move { hermes::update::install(&release) });
    cx.spawn(async move |cx| {
        let result = task.await;
        state.update(cx, |s, cx| {
            s.update_status = match result {
                Ok(()) => hermes::update::Status::Available,
                Err(error) => {
                    eprintln!("hermes: update failed: {error:#}");
                    hermes::update::Status::Failed
                }
            };
            cx.notify();
        });
    })
    .detach();
}
