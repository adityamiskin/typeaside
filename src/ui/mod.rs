mod demo;
mod dictionary;
mod home;
mod motion;
mod overlay;
mod settings;
mod theme;
mod tray;
mod window;

use gpui::{App, AppContext, QuitMode, WindowHandle};
use hermes::engine::{Dictation, Engine, Event, Phase};
use hermes::{config::AppConfig, credentials, paths::AppPaths};

pub struct AppState {
    config: AppConfig,
    paths: AppPaths,
    engine: Option<Engine>,
    is_demo: bool,
    key_hint: String,
    history: Vec<Dictation>,
    phase: Phase,
    save_error: Option<String>,
    trace_started: Option<std::time::Instant>,
    initial_page: window::Page,
}

impl AppState {
    fn persist(&mut self, key: &str) {
        let result = (|| -> anyhow::Result<()> {
            if !self.is_demo {
                if !key.is_empty() {
                    let url = reqwest::Url::parse(&self.config.stt.base_url)?;
                    let host = url
                        .host_str()
                        .ok_or_else(|| anyhow::anyhow!("endpoint has no host"))?;
                    credentials::save_credential(&self.paths, host, key)?;
                }
                self.config.save(&self.paths)?;
                if let Some(engine) = &self.engine {
                    engine.send(hermes::engine::Command::Reload(self.config.clone()));
                }
            }
            if !key.is_empty() {
                self.key_hint = "••••••••".into();
            } else if !self.is_demo {
                self.key_hint = key_hint(&self.paths, &self.config.stt.base_url);
            }
            Ok(())
        })();
        self.save_error = result.err().map(|error| format!("Could not save: {error}"));
    }
}

pub fn run(is_demo: bool, started: std::time::Instant) -> anyhow::Result<()> {
    let paths = AppPaths::discover()?;
    let mut config = if is_demo {
        AppConfig::default()
    } else {
        AppConfig::load(&paths)?
    };
    let mut startup_error = None;
    let (engine, events) = if is_demo {
        config.dictionary = vec![
            "Aditya".into(),
            "GPUI".into(),
            "niri".into(),
            "Zapsight".into(),
        ];
        (None, demo::spawn())
    } else {
        match Engine::spawn(config.clone()) {
            Ok((engine, events)) => (Some(engine), events),
            Err(error) => {
                startup_error = Some(format!("Engine startup failed: {error}"));
                let (_, events) = async_channel::unbounded();
                (None, events)
            }
        }
    };
    let key_hint = if is_demo {
        "••••••••".into()
    } else {
        key_hint(&paths, &config.stt.base_url)
    };
    let initial_page = if is_demo {
        match std::env::var("HERMES_DEMO_PAGE").as_deref() {
            Ok("dictionary") => window::Page::Dictionary,
            Ok("settings") => window::Page::Settings,
            _ => window::Page::Home,
        }
    } else {
        window::Page::Home
    };
    let mut history = if is_demo {
        demo::history()
    } else {
        hermes::engine::load_history(&paths)
    };
    history.sort_by_key(|item| std::cmp::Reverse(item.at));
    gpui_platform::application()
        .with_assets(gpui_kit::assets::Assets)
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            theme::init(cx);
            let state = cx.new(|_| AppState {
                config,
                paths,
                engine,
                is_demo,
                key_hint,
                history,
                phase: Phase::Idle,
                save_error: startup_error,
                trace_started: (std::env::var("HERMES_TRACE").as_deref() == Ok("1"))
                    .then_some(started),
                initial_page,
            });
            let mut main_window = match window::open(state.clone(), cx) {
                Ok(window) => Some(window),
                Err(error) => {
                    eprintln!("hermes: cannot open main window: {error:#}");
                    cx.quit();
                    return;
                }
            };
            match tray::spawn() {
                Ok((handle, actions)) => {
                    let state = state.clone();
                    cx.spawn(async move |cx| {
                        let _handle = handle;
                        while let Ok(action) = actions.recv().await {
                            cx.update(|cx| match action {
                                tray::Action::Quit => cx.quit(),
                                tray::Action::Open => {
                                    let activated = main_window.is_some_and(|window| {
                                        window
                                            .update(cx, |_, window, _| window.activate_window())
                                            .is_ok()
                                    });
                                    if !activated {
                                        match window::open(state.clone(), cx) {
                                            Ok(window) => main_window = Some(window),
                                            Err(error) => eprintln!(
                                                "hermes: cannot open main window: {error:#}"
                                            ),
                                        }
                                    }
                                }
                            });
                        }
                    })
                    .detach();
                }
                Err(error) => eprintln!("hermes: tray unavailable: {error:#}"),
            }
            cx.spawn(async move |cx| {
                let mut surface: Option<WindowHandle<overlay::Overlay>> = None;
                while let Ok(event) = events.recv().await {
                    cx.update(|cx| match event {
                        Event::Level(level) => {
                            if let Some(surface) = surface {
                                let _ = surface.update(cx, |view, _, cx| view.level(level, cx));
                            }
                        }
                        Event::Done(dictation) => {
                            state.update(cx, |state, cx| {
                                state.history.insert(0, dictation);
                                cx.notify();
                            });
                            let updated = surface.is_some_and(|surface| {
                                surface
                                    .update(cx, |view, window, cx| view.success(window, cx))
                                    .is_ok()
                            });
                            if !updated {
                                match overlay::open(cx) {
                                    Ok(opened) => {
                                        let _ = opened.update(cx, |view, window, cx| {
                                            view.success(window, cx)
                                        });
                                        surface = Some(opened);
                                    }
                                    Err(error) => {
                                        eprintln!("hermes: cannot open overlay: {error:#}")
                                    }
                                }
                            }
                        }
                        Event::Phase(phase) => {
                            state.update(cx, |state, cx| {
                                state.phase = phase.clone();
                                cx.notify();
                            });
                            let updated = surface.is_some_and(|surface| {
                                surface
                                    .update(cx, |view, window, cx| {
                                        view.phase(phase.clone(), window, cx)
                                    })
                                    .is_ok()
                            });
                            if !updated && phase != Phase::Idle {
                                match overlay::open(cx) {
                                    Ok(opened) => {
                                        let _ = opened.update(cx, |view, window, cx| {
                                            view.phase(phase, window, cx)
                                        });
                                        surface = Some(opened);
                                    }
                                    Err(error) => {
                                        eprintln!("hermes: cannot open overlay: {error:#}")
                                    }
                                }
                            }
                        }
                    });
                }
            })
            .detach();
        });
    Ok(())
}

fn key_hint(paths: &AppPaths, endpoint: &str) -> String {
    reqwest::Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .and_then(|host| credentials::get_credential(paths, &host).ok().flatten())
        .map_or_else(|| "Enter API key".into(), |_| "••••••••".into())
}
