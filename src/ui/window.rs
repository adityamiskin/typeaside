use super::{
    AppState,
    settings::SettingsInputs,
    theme::{self, *},
};
use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, FontWeight, IntoElement, Render, Styled,
    Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
};
use gpui_kit::component::input::{InputEvent, InputState};
use typeaside::{config::AppCategory, engine::Command};

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Home,
    Dictionary,
    Snippets,
    Style,
    Settings,
    Account,
    Onboarding,
}
impl Page {
    pub fn from_demo() -> Self {
        match std::env::var("TYPEASIDE_DEMO_PAGE").as_deref() {
            Ok("dictionary") => Self::Dictionary,
            Ok("snippets") => Self::Snippets,
            Ok("style") => Self::Style,
            Ok("settings") => Self::Settings,
            Ok("account") => Self::Account,
            Ok("onboarding") => Self::Onboarding,
            _ => Self::Home,
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum Shortcut {
    Dictate,
    Command,
    Translate,
    Pause,
}

pub struct MainWindow {
    pub state: Entity<AppState>,
    pub settings: SettingsInputs,
    pub word_input: Entity<InputState>,
    pub search_input: Entity<InputState>,
    pub toast: Option<&'static str>,
    pub toast_task: Option<gpui::Task<()>>,
    pub history_tab: super::home::HistoryTab,
    pub menu_history: Option<std::time::SystemTime>,
    pub snippet_editor: Option<super::snippets::SnippetEditor>,
    pub style_category: AppCategory,
    pub app_instructions: super::style::AppInstructions,
    pub hovered_history: Option<usize>,
    pub hovered_word: Option<usize>,
    pub capture: Option<Shortcut>,
    pub capture_modifiers: gpui::Modifiers,
    pub settings_scroll: gpui::ScrollHandle,
    pub settings_jump: Option<(usize, gpui::Pixels)>,
    pub focus: FocusHandle,
    page: Page,
    pub onboarding: super::onboarding::Onboarding,
    trace_started: Option<std::time::Instant>,
}
impl MainWindow {
    fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        theme::apply(state.read(cx).config.theme, window.appearance(), cx);
        cx.observe_window_appearance(window, |this, window, cx| {
            theme::apply(this.state.read(cx).config.theme, window.appearance(), cx);
            cx.notify();
        })
        .detach();
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.state.update(cx, |s, _| {
                    if s.sync_focus_at
                        .is_none_or(|at| at.elapsed().as_secs() >= 60)
                    {
                        s.sync_focus_at = Some(std::time::Instant::now());
                        let _ = s.sync.try_send(());
                    }
                });
            }
        })
        .detach();
        let config = state.read(cx).config.clone();
        let devices = state.read(cx).devices.clone();
        let settings = SettingsInputs::new(&config, &devices, window, cx);
        let word_input = cx.new(|cx| InputState::new(window, cx).placeholder("Add a word or name"));
        let search = state.read(cx).search.clone();
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search history")
                .default_value(search)
        });
        cx.subscribe_in(&word_input, window, |this, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.add_word(window, cx);
            }
        })
        .detach();
        cx.subscribe(&search_input, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                let search = input.read(cx).value().to_string();
                this.state.update(cx, |s, cx| {
                    s.search = search;
                    cx.notify();
                });
            }
        })
        .detach();
        cx.observe_in(&state, window, |this, _, window, cx| {
            this.settings.refresh(&this.state, window, cx);
            this.sync_app_instructions(window, cx);
            if this.onboarding.pending_key {
                if this.state.read(cx).key_hint == "Saved in keyring" {
                    this.onboarding.pending_key = false;
                    this.onboarding_next(false, window, cx);
                } else if this.state.read(cx).save_error.is_some() {
                    this.onboarding.pending_key = false;
                }
            }
            theme::apply(this.state.read(cx).config.theme, window.appearance(), cx);
            cx.notify();
        })
        .detach();
        let weak = state.downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            #[cfg(target_os = "macos")]
            show_in_dock(false);
            let _ = weak.update(cx, |s, cx| {
                s.mic_testing = false;
                s.mic_generation += 1;
                s.command(Command::MicTest(false));
                cx.notify();
            });
            true
        });
        let page = state.read(cx).initial_page;
        let trace_started = state.read(cx).trace_started;
        let mut this = Self {
            state: state.clone(),
            settings,
            word_input,
            search_input,
            snippet_editor: None,
            toast: None,
            toast_task: None,
            menu_history: None,
            history_tab: super::home::HistoryTab::default(),
            style_category: AppCategory::Work,
            app_instructions: super::style::AppInstructions::new(window, cx),
            hovered_history: None,
            hovered_word: None,
            capture: None,
            capture_modifiers: gpui::Modifiers::default(),
            settings_scroll: gpui::ScrollHandle::new(),
            settings_jump: None,
            focus: cx.focus_handle(),
            page,
            onboarding: super::onboarding::Onboarding::new(state.read(cx).onboarding_steps.clone()),
            trace_started,
        };
        this.bind_settings(window, cx);
        this.bind_app_instructions(window, cx);
        if this.state.read(cx).is_demo {
            if page == Page::Home {
                let at = this.state.read(cx).history.get(1).map(|d| d.at);
                this.state.update(cx, |s, _| s.expanded = at);
                this.hovered_history = Some(0);
                this.history_tab = match std::env::var("TYPEASIDE_DEMO_TAB").as_deref() {
                    Ok("ask") => super::home::HistoryTab::AskAnything,
                    Ok("dictations") => super::home::HistoryTab::Dictations,
                    _ => super::home::HistoryTab::All,
                };
                if let Ok(kind) = std::env::var("TYPEASIDE_DEMO_DIALOG") {
                    let timestamp = (kind == "delete").then(|| this.state.read(cx).history[0].at);
                    let failed_at = this.state.read(cx).failed.first().map(|failed| failed.at);
                    cx.spawn_in(window, async move |this, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(150))
                            .await;
                        let _ = this.update_in(cx, |this, window, cx| {
                            this.confirm_removal(timestamp, failed_at, window, cx)
                        });
                    })
                    .detach();
                }
                if let Ok(message) = std::env::var("TYPEASIDE_DEMO_TOAST") {
                    if message == "deleted" {
                        this.delete_history(0, cx);
                    } else {
                        this.show_toast(
                            match message.as_str() {
                                "raw" => "Copied raw",
                                "discarded" => "Discarded",
                                _ => "Copied",
                            },
                            cx,
                        );
                    }
                }
            }
            if page == Page::Dictionary {
                this.hovered_word = Some(3);
            }
            if page == Page::Settings {
                if let Ok(scroll) = std::env::var("TYPEASIDE_DEMO_SETTINGS_SCROLL") {
                    cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(150))
                            .await;
                        let _ = this.update(cx, |this, cx| {
                            if let Some(group) = super::settings::GROUPS
                                .iter()
                                .position(|group| group.eq_ignore_ascii_case(&scroll))
                            {
                                this.settings_scroll.scroll_to_top_of_item(group);
                            } else if scroll == "bottom" {
                                this.settings_scroll.scroll_to_bottom();
                            }
                            cx.notify();
                        });
                    })
                    .detach();
                }
                this.capture = Some(Shortcut::Translate);
                this.start_mic(cx);
            }
            if page == Page::Snippets {
                this.edit_snippet(Some(1), window, cx);
            }
        }
        if matches!(page, Page::Settings | Page::Account) {
            super::refresh_account(&this.state, cx);
        }
        this.init_onboarding(window, cx);
        this
    }
    pub fn set_page(&mut self, page: Page, _: &mut Window, cx: &mut Context<Self>) {
        if page == Page::Settings {
            super::refresh_account(&self.state, cx);
            super::refresh_devices(&self.state, cx);
        }
        self.page = page;
        self.state.update(cx, |s, _| s.initial_page = page);
        self.capture = None;
        cx.notify();
    }
    pub fn start_mic(&mut self, cx: &mut Context<Self>) {
        let generation = self.state.update(cx, |s, cx| {
            s.mic_testing = true;
            s.mic_generation += 1;
            s.command(Command::MicTest(true));
            cx.notify();
            s.mic_generation
        });
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(5))
                .await;
            state.update(cx, |s, cx| {
                if s.mic_generation == generation {
                    s.mic_testing = false;
                    s.command(Command::MicTest(false));
                    cx.notify();
                }
            });
        })
        .detach();
    }
}
impl Drop for MainWindow {
    fn drop(&mut self) {
        if std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1") {
            eprintln!("typeaside main window released");
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let trace = self.trace_started.take().map(|started| {
            gpui::canvas(
                |_, _, _| (),
                move |_, _, _, _| {
                    eprintln!(
                        "typeaside first main frame: {:.3} ms",
                        started.elapsed().as_secs_f64() * 1000.
                    );
                },
            )
            .absolute()
            .size(px(1.))
        });
        let state = self.state.read(cx);
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let permission_notice = state.permission_notice.clone();
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let permission_help = super::permissions::help(state);
        let onboarding = !state.config.onboarded;
        let save_error = state.save_error.clone();
        let account_label = state
            .account
            .as_ref()
            .map_or("Sign in".to_owned(), |account| account.email.clone());
        let usage = state
            .account
            .as_ref()
            .filter(|account| {
                account.plan == typeaside::account::Plan::Free
                    && state.config.transcription == typeaside::config::Source::Hosted
            })
            .map(|account| {
                let cap = account.usage.cap.unwrap_or(2000);
                let limit = account.at_limit();
                let label = if limit {
                    let weekday = chrono::DateTime::from_timestamp_millis(account.usage.resets_at)
                        .map(|date| date.with_timezone(&chrono::Local).format("%a").to_string())
                        .unwrap_or_default();
                    format!("Limit reached · resets {weekday}")
                } else {
                    format!(
                        "{} / {} words",
                        super::settings::grouped(account.usage.words),
                        super::settings::grouped(cap)
                    )
                };
                let fraction = if cap == 0 {
                    1.
                } else {
                    (account.usage.words as f32 / cap as f32).min(1.)
                };
                div()
                    .px(px(10.))
                    .pb(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_ellipsis()
                                    .text_color(if limit { p.red } else { p.muted })
                                    .child(label),
                            )
                            .child(
                                div()
                                    .id("sidebar-upgrade")
                                    .flex_shrink_0()
                                    .text_color(p.blue)
                                    .cursor_pointer()
                                    .child("Upgrade")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if this.state.read(cx).is_demo {
                                            return;
                                        }
                                        cx.background_executor()
                                            .spawn(async {
                                                let _ = typeaside::account::open(format!(
                                                    "{}/account",
                                                    typeaside::account::web_url()
                                                ));
                                            })
                                            .detach();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(6.))
                            .h(px(2.))
                            .w_full()
                            .rounded_full()
                            .bg(p.divider)
                            .child(
                                div()
                                    .h_full()
                                    .w(gpui::relative(fraction))
                                    .rounded_full()
                                    .bg(if limit { p.red } else { p.blue }),
                            ),
                    )
            });
        let side = div()
            .w(px(SIDEBAR))
            .flex_shrink_0()
            .h_full()
            .bg(p.side)
            .border_r_1()
            .border_color(p.divider)
            .px(px(10.))
            .py(px(14.))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(10.))
                    // Clears the macOS traffic lights drawn over the transparent titlebar.
                    .pt(px(if cfg!(target_os = "macos") { 22. } else { 2. }))
                    .pb(px(16.))
                    .text_size(px(14.))
                    .line_height(px(20.3))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(
                        gpui::svg()
                            .path("icons/typeaside.svg")
                            .size(px(16.))
                            .text_color(p.fg),
                    )
                    .child("TypeAside"),
            )
            .children(
                [
                    (Page::Home, "Home", "icons/house.svg"),
                    (Page::Dictionary, "Dictionary", "icons/book-a.svg"),
                    (Page::Snippets, "Snippets", "icons/text-quote.svg"),
                    (Page::Style, "Style", "icons/palette.svg"),
                    (Page::Settings, "Settings", "icons/settings.svg"),
                ]
                .into_iter()
                .map(|(page, title, icon)| {
                    let selected = self.page == page;
                    div()
                        .id(title)
                        .min_h(px(30.))
                        .max_h(px(30.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .cursor_pointer()
                        .bg(if selected { p.tint } else { p.side })
                        .text_color(if selected { p.fg } else { p.muted })
                        .hover(|s| s.bg(p.hover))
                        .child(
                            gpui::svg()
                                .path(icon)
                                .size(px(15.))
                                .flex_shrink_0()
                                .text_color(if selected { p.blue } else { p.muted }),
                        )
                        .child(title)
                        .on_click(cx.listener(move |this, _, w, cx| this.set_page(page, w, cx)))
                }),
            )
            .child(div().flex_1())
            .when_some(usage, |side, usage| side.child(usage))
            .child(
                div()
                    .id("feedback")
                    .h(px(30.))
                    .flex_shrink_0()
                    .px(px(10.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .hover(|s| s.bg(p.hover))
                    .child(
                        gpui::svg()
                            .path("icons/message-square.svg")
                            .size(px(15.))
                            .text_color(p.muted),
                    )
                    .child("Feedback")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.state.read(cx).is_demo {
                            return;
                        }
                        cx.background_executor()
                            .spawn(async {
                                let _ = typeaside::account::open(format!(
                                    "{}/feedback?version={}&os={}",
                                    typeaside::account::web_url(),
                                    env!("CARGO_PKG_VERSION"),
                                    std::env::consts::OS
                                ));
                            })
                            .detach();
                    })),
            )
            .child({
                let selected = self.page == Page::Account;
                div()
                    .id("account")
                    .h(px(30.))
                    .flex_shrink_0()
                    .px(px(10.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .when(selected, |row| row.bg(p.tint))
                    .text_color(if selected { p.fg } else { p.muted })
                    .hover(|s| s.bg(p.hover))
                    .child(
                        gpui::svg()
                            .path("icons/circle-user.svg")
                            .size(px(15.))
                            .flex_shrink_0()
                            .text_color(if selected { p.blue } else { p.muted }),
                    )
                    .child(div().min_w_0().text_ellipsis().child(account_label))
                    .on_click(cx.listener(|this, _, w, cx| this.set_page(Page::Account, w, cx)))
            })
            .when_some(save_error, |side, error| {
                side.child(
                    div()
                        .px(px(10.))
                        .py(px(8.))
                        .text_size(px(12.))
                        .line_height(px(17.4))
                        .text_color(p.red)
                        .child(error),
                )
            });
        let content = if onboarding {
            self.onboarding(window, cx).into_any_element()
        } else {
            match self.page {
                Page::Home => self.home(cx).into_any_element(),
                Page::Dictionary => self.dictionary(cx).into_any_element(),
                Page::Snippets => self.snippets(cx).into_any_element(),
                Page::Style => self.style(cx).into_any_element(),
                Page::Account => self.account_page(cx).into_any_element(),
                Page::Settings | Page::Onboarding => self.settings(cx).into_any_element(),
            }
        };
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let content = div()
            .size_full()
            .flex()
            .flex_col()
            .children(
                permission_notice
                    .as_ref()
                    .filter(|_| !onboarding)
                    .map(|notice| {
                        div()
                            .px(px(16.))
                            .py(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.))
                                    .text_color(p.red)
                                    .child(notice.clone())
                                    .child(
                                        div()
                                            .id("permission-settings")
                                            .cursor_pointer()
                                            .text_color(p.fg)
                                            .underline()
                                            .child(if cfg!(target_os = "linux") {
                                                "Allow"
                                            } else {
                                                "Open Settings"
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let missing = super::permissions::statuses()
                                                    .into_iter()
                                                    .position(|granted| !granted);
                                                if let Some(index) = missing {
                                                    super::permissions::allow(
                                                        index,
                                                        &this.state,
                                                        cx,
                                                    );
                                                }
                                            })),
                                    ),
                            )
                            .children(
                                permission_help.map(|help| div().text_color(p.muted).child(help)),
                            )
                    }),
            )
            .child(div().flex_1().min_h_0().child(content));
        let root = div()
            .id("main")
            .on_drop::<gpui::ExternalPaths>(cx.listener(
                |this, paths: &gpui::ExternalPaths, _, cx| {
                    if this.page == Page::Home
                        && this.state.read(cx).config.onboarded
                        && let Some(path) = paths.0.first()
                    {
                        this.transcribe_file(path.clone(), cx);
                    }
                },
            ))
            .relative()
            .track_focus(&self.focus)
            .size_full()
            .bg(p.bg)
            .border_1()
            .border_color(p.frame)
            .rounded(px(10.))
            .overflow_hidden()
            .text_color(p.fg)
            .font_family("Inter")
            .text_size(px(13.))
            .line_height(px(18.85))
            .flex()
            .children(trace)
            .when(!onboarding, |root| root.child(side))
            .child(div().flex_1().min_w_0().h_full().child(content))
            .children(self.update_toast(cx))
            .children(self.history_toast(cx));
        let _ = window;
        root.capture_any_mouse_down(cx.listener(|this, _, _, cx| {
            if this.skip_intro(cx) {
                cx.stop_propagation();
            }
        }))
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, w, cx| {
            if this.skip_intro(cx) {
                cx.stop_propagation();
                return;
            }
            if let Some(target) = this.capture.take() {
                #[cfg(target_os = "linux")]
                let combo = typeaside::hotkeys::chord();
                #[cfg(not(target_os = "linux"))]
                let combo = super::settings::shortcut_combo(&event.keystroke);
                this.save_shortcut(target, combo, cx);
                cx.stop_propagation();
            } else if !this.state.read(cx).config.onboarded {
                match event.keystroke.key.as_str() {
                    "enter" => this.onboarding_next(false, w, cx),
                    "escape" => this.onboarding_back(cx),
                    _ => {}
                }
            }
        }))
        .on_modifiers_changed(cx.listener(
            |this, event: &gpui::ModifiersChangedEvent, _, cx| {
                if event.modifiers.modified() && this.skip_intro(cx) {
                    cx.stop_propagation();
                    return;
                }
                if this.capture.is_some() {
                    if event.modifiers.modified() {
                        this.capture_modifiers.control |= event.modifiers.control;
                        this.capture_modifiers.alt |= event.modifiers.alt;
                        this.capture_modifiers.shift |= event.modifiers.shift;
                        this.capture_modifiers.platform |= event.modifiers.platform;
                        this.capture_modifiers.function |= event.modifiers.function;
                    } else if let Some(target) = this.capture.take() {
                        #[cfg(target_os = "linux")]
                        this.save_shortcut(target, typeaside::hotkeys::chord(), cx);
                        #[cfg(not(target_os = "linux"))]
                        {
                            let modifiers = this.capture_modifiers;
                            let combo = [
                                (modifiers.control, "CTRL"),
                                (modifiers.alt, "ALT"),
                                (modifiers.shift, "SHIFT"),
                                (modifiers.platform, "META"),
                                (modifiers.function, "FN"),
                            ]
                            .into_iter()
                            .filter_map(|(held, name)| held.then_some(name))
                            .collect::<Vec<_>>()
                            .join("+");
                            this.save_shortcut(target, combo, cx);
                        }
                    }
                }
            },
        ))
    }
}
pub fn open(state: Entity<AppState>, cx: &mut App) -> anyhow::Result<gpui::AnyWindowHandle> {
    let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
    let (handle, _) = gpui_kit::open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(760.), px(480.))),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("TypeAside".into()),
                appears_transparent: true,
                ..Default::default()
            }),
            app_id: Some("typeaside".into()),
            ..Default::default()
        },
        cx,
        |window, cx| cx.new(|cx| MainWindow::new(state, window, cx)),
    )?;
    Ok(handle)
}

/// The Dock icon follows the main window; otherwise TypeAside lives in the menu bar.
#[cfg(target_os = "macos")]
pub fn show_in_dock(visible: bool) {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    let Some(main_thread) = objc2::MainThreadMarker::new() else {
        return;
    };
    NSApplication::sharedApplication(main_thread).setActivationPolicy(if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    });
}
