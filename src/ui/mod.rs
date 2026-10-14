mod binding;
mod demo;
mod dictionary;
mod home;
mod motion;
mod overlay;
mod settings;
mod theme;
mod tray;
mod window;

use binding::{Binding, Preferences};
use gpui::{App, AppContext, QuitMode, WindowHandle};
use hermes::engine::{Dictation, Engine, Event, Phase};

pub struct AppState {
    preferences: Preferences,
    binding: Binding,
    history: Vec<Dictation>,
    phase: Phase,
    save_error: Option<String>,
    initial_page: window::Page,
}

impl AppState {
    fn persist(&mut self, key: &str) {
        self.save_error = self
            .binding
            .save(&self.preferences, key)
            .err()
            .map(|error| format!("Could not save: {error}"));
    }
}

pub fn run(is_demo: bool) -> anyhow::Result<()> {
    let (mut binding, mut preferences) = if is_demo {
        (Binding::demo(), Preferences::default())
    } else {
        Binding::load()?
    };
    let events = if is_demo {
        preferences.dictionary = vec![
            "Aditya".into(),
            "GPUI".into(),
            "niri".into(),
            "Zapsight".into(),
        ];
        preferences.key_hint = "gsk_••••••••3f9a".into();
        demo::spawn()
    } else {
        let (engine, events) = Engine::spawn(binding.config())?;
        binding.engine = Some(engine);
        events
    };
    preferences
        .dictionary
        .sort_by_key(|word| word.to_lowercase());
    preferences
        .dictionary
        .dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
    let initial_page = if is_demo {
        match std::env::var("HERMES_DEMO_PAGE").as_deref() {
            Ok("dictionary") => window::Page::Dictionary,
            Ok("settings") => window::Page::Settings,
            _ => window::Page::Home,
        }
    } else {
        window::Page::Home
    };
    let history = if is_demo { demo::history() } else { Vec::new() };
    gpui_platform::application()
        .with_assets(gpui_kit::assets::Assets)
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            theme::init(cx);
            let state = cx.new(|_| AppState {
                preferences,
                binding,
                history,
                phase: Phase::Idle,
                save_error: None,
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
                            if let Some(surface) = surface {
                                let _ =
                                    surface.update(cx, |view, window, cx| view.success(window, cx));
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
