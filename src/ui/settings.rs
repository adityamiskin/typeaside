use super::{
    AppState,
    theme::*,
    window::{MainWindow, Page, Shortcut},
};
use gpui::{AppContext, Context, Entity, IntoElement, Styled, Window, div, prelude::*, px};
use gpui_kit::component::{
    IndexPath, Sizable,
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
};
use hermes::config::{AppConfig, RecordingMode, Source, Theme};

const LANGUAGES: [(&str, &str); 12] = [
    ("English", "en"),
    ("Hindi", "hi"),
    ("Spanish", "es"),
    ("French", "fr"),
    ("German", "de"),
    ("Portuguese", "pt"),
    ("Italian", "it"),
    ("Japanese", "ja"),
    ("Korean", "ko"),
    ("Chinese", "zh"),
    ("Arabic", "ar"),
    ("Russian", "ru"),
];
#[derive(Clone, Copy)]
enum TextField {
    Endpoint,
    Model,
    EditingEndpoint,
    EditingModel,
}
#[derive(Clone, Copy)]
enum Selection {
    Device,
    Spoken,
    Translation,
}
#[derive(Clone, Copy)]
enum Toggle {
    Editing,
    Sounds,
    Duck,
    Submit,
    Login,
}

pub struct SettingsInputs {
    pub endpoint: Entity<InputState>,
    pub model: Entity<InputState>,
    pub key: Entity<InputState>,
    pub cleanup_endpoint: Entity<InputState>,
    pub cleanup_model: Entity<InputState>,
    pub cleanup_key: Entity<InputState>,
    pub device: Entity<SelectState<Vec<String>>>,
    pub spoken: Entity<SelectState<Vec<String>>>,
    pub translation: Entity<SelectState<Vec<String>>>,
    devices: Vec<String>,
    stt_hint: String,
    editing_hint: String,
}
impl SettingsInputs {
    pub fn new(
        c: &AppConfig,
        devices: &[String],
        w: &mut Window,
        cx: &mut Context<MainWindow>,
    ) -> Self {
        let mut text = |value: &str, masked| {
            cx.new(|cx| {
                InputState::new(w, cx)
                    .default_value(value.to_owned())
                    .masked(masked)
            })
        };
        let endpoint = text(&c.stt.base_url, false);
        let model = text(&c.stt.model, false);
        let key = text("", true);
        let cleanup_endpoint = text(&c.cleanup.endpoint.base_url, false);
        let cleanup_model = text(&c.cleanup.endpoint.model, false);
        let cleanup_key = text("", true);
        let mut select = |values: Vec<String>, selected: Option<&str>| {
            let index = selected
                .and_then(|s| values.iter().position(|v| v == s))
                .unwrap_or(0);
            cx.new(|cx| SelectState::new(values, Some(IndexPath::default().row(index)), w, cx))
        };
        let device = select(devices.to_vec(), c.input_device.as_deref());
        let spoken_name = c.language.as_deref().and_then(|code| {
            LANGUAGES
                .iter()
                .find(|(_, c)| *c == code)
                .map(|(name, _)| *name)
        });
        let spoken = select(
            std::iter::once("Detect automatically")
                .chain(LANGUAGES.iter().map(|(n, _)| *n))
                .map(str::to_owned)
                .collect(),
            spoken_name,
        );
        let translation = select(
            std::iter::once("Off")
                .chain(LANGUAGES.iter().map(|(n, _)| *n))
                .map(str::to_owned)
                .collect(),
            c.translate_to.as_deref(),
        );
        Self {
            endpoint,
            model,
            key,
            cleanup_endpoint,
            cleanup_model,
            cleanup_key,
            device,
            spoken,
            translation,
            devices: devices.to_vec(),
            stt_hint: String::new(),
            editing_hint: String::new(),
        }
    }
    pub fn refresh(
        &mut self,
        state: &Entity<AppState>,
        w: &mut Window,
        cx: &mut Context<MainWindow>,
    ) {
        let s = state.read(cx);
        let stt = s.key_hint.clone();
        let editing = s.editing_key_hint.clone();
        if self.devices != s.devices {
            self.devices = s.devices.clone();
            let devices = self.devices.clone();
            let selected = s.config.input_device.clone();
            self.device.update(cx, |v, cx| {
                v.set_items(devices, w, cx);
                v.set_selected_value(&selected.unwrap_or_else(|| "System default".into()), w, cx);
            });
        }
        if self.stt_hint != stt {
            self.stt_hint = stt.clone();
            self.key.update(cx, |v, cx| v.set_placeholder(stt, w, cx));
        }
        if self.editing_hint != editing {
            self.editing_hint = editing.clone();
            self.cleanup_key
                .update(cx, |v, cx| v.set_placeholder(editing, w, cx));
        }
    }
}
pub fn input(state: &Entity<InputState>, p: Palette) -> impl IntoElement {
    Input::new(state)
        .min_h(px(30.))
        .max_h(px(30.))
        .w_full()
        .rounded(px(6.))
        .text_size(px(13.))
        .bg(p.bg)
}
fn select(state: &Entity<SelectState<Vec<String>>>, p: Palette) -> impl IntoElement {
    Select::new(state)
        .small()
        .min_h(px(30.))
        .max_h(px(30.))
        .w(px(280.))
        .bg(p.bg)
        .text_size(px(13.))
}
impl MainWindow {
    pub fn bind_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (entity, field) in [
            (&self.settings.endpoint, TextField::Endpoint),
            (&self.settings.model, TextField::Model),
            (&self.settings.cleanup_endpoint, TextField::EditingEndpoint),
            (&self.settings.cleanup_model, TextField::EditingModel),
        ] {
            cx.subscribe(entity, move |this, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    this.state.update(cx, |s, cx| {
                        match field {
                            TextField::Endpoint => s.config.stt.base_url = value,
                            TextField::Model => s.config.stt.model = value,
                            TextField::EditingEndpoint => {
                                s.config.cleanup.endpoint.base_url = value
                            }
                            TextField::EditingModel => s.config.cleanup.endpoint.model = value,
                        }
                        s.persist();
                        cx.notify();
                    });
                }
            })
            .detach();
        }
        for (entity, editing) in [
            (&self.settings.key, false),
            (&self.settings.cleanup_key, true),
        ] {
            cx.subscribe_in(entity, window, move |this, _, event, w, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.save_key(editing, w, cx);
                }
            })
            .detach();
        }
        for (entity, field) in [
            (&self.settings.device, Selection::Device),
            (&self.settings.spoken, Selection::Spoken),
            (&self.settings.translation, Selection::Translation),
        ] {
            cx.subscribe(entity, move |this, _, event, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = value.clone();
                this.state.update(cx, |s, cx| {
                    match field {
                        Selection::Device => {
                            s.config.input_device = (value != "System default").then_some(value)
                        }
                        Selection::Spoken => {
                            s.config.language = LANGUAGES
                                .iter()
                                .find(|(n, _)| *n == value)
                                .map(|(_, code)| (*code).into())
                        }
                        Selection::Translation => {
                            s.config.translate_to = (value != "Off").then_some(value)
                        }
                    }
                    s.persist();
                    cx.notify();
                });
            })
            .detach();
        }
        self.settings.refresh(&self.state, window, cx);
    }
    fn save_key(&mut self, editing: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entity = if editing {
            self.settings.cleanup_key.clone()
        } else {
            self.settings.key.clone()
        };
        let key = entity.read(cx).value().to_string();
        if key.is_empty() {
            return;
        }
        let s = self.state.read(cx);
        let demo = s.is_demo;
        let endpoint = if editing {
            s.config.cleanup.endpoint.base_url.clone()
        } else {
            s.config.stt.base_url.clone()
        };
        let task = cx.background_executor().spawn(async move {
            let host = hermes::credentials::endpoint_host(&endpoint)?;
            if !demo {
                hermes::credentials::save_credential(&host, &key)?;
            }
            Ok::<_, anyhow::Error>(host)
        });
        entity.update(cx, |input, cx| input.set_value("", window, cx));
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            let result = task.await;
            state.update(cx, |s, cx| {
                if let Ok(host) = result {
                    if hermes::credentials::endpoint_host(&s.config.stt.base_url)
                        .ok()
                        .as_ref()
                        == Some(&host)
                    {
                        s.key_hint = "Saved in keyring".into();
                    }
                    if hermes::credentials::endpoint_host(&s.config.cleanup.endpoint.base_url)
                        .ok()
                        .as_ref()
                        == Some(&host)
                    {
                        s.editing_key_hint = "Saved in keyring".into();
                    }
                    s.persist();
                } else {
                    s.save_error = Some("Could not save API key in keyring".into());
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub fn save_shortcut(&mut self, target: Shortcut, key: String, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            match target {
                Shortcut::Dictate => s.config.hotkey = key,
                Shortcut::Command => s.config.command_hotkey = Some(key),
                Shortcut::Paste => s.config.paste_last_hotkey = Some(key),
                Shortcut::Cancel => s.config.cancel_hotkey = Some(key),
            }
            s.persist();
            cx.notify();
        });
        self.capture_modifiers = gpui::Modifiers::default();
        cx.notify();
    }
    fn shortcut(
        &self,
        target: Shortcut,
        value: Option<&str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette(cx);
        div()
            .flex()
            .items_center()
            .id(match target {
                Shortcut::Dictate => "dictate-key",
                Shortcut::Command => "command-key",
                Shortcut::Paste => "paste-key",
                Shortcut::Cancel => "cancel-key",
            })
            .cursor_pointer()
            .child(if self.capture == Some(target) {
                div()
                    .text_color(p.faint)
                    .child("Press keys…")
                    .into_any_element()
            } else {
                kbd(key_label(value.unwrap_or("Unbound")), p).into_any_element()
            })
            .on_click(cx.listener(move |this, _, w, cx| {
                this.capture = Some(target);
                this.capture_modifiers = gpui::Modifiers::default();
                w.focus(&this.focus, cx);
                cx.notify();
            }))
    }
    fn toggle(
        &self,
        id: &'static str,
        on: bool,
        field: Toggle,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette(cx);
        div().flex().justify_end().child(
            div()
                .id(id)
                .w(px(30.))
                .h(px(18.))
                .rounded_full()
                .relative()
                .bg(if on { p.accent } else { p.border })
                .cursor_pointer()
                .child(
                    div()
                        .absolute()
                        .top(px(2.))
                        .left(px(if on { 14. } else { 2. }))
                        .size(px(14.))
                        .rounded_full()
                        .bg(if on { p.on_accent } else { p.bg }),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.state.update(cx, |s, cx| {
                        match field {
                            Toggle::Editing => s.config.cleanup.enabled = !on,
                            Toggle::Sounds => s.config.sounds = !on,
                            Toggle::Duck => s.config.duck_media = !on,
                            Toggle::Submit => s.config.auto_submit = !on,
                            Toggle::Login => {
                                if !s.is_demo
                                    && let Err(e) = hermes::autostart::set_enabled(!on)
                                {
                                    s.save_error =
                                        Some(format!("Could not change login launch: {e}"));
                                    cx.notify();
                                    return;
                                }
                                s.config.launch_at_login = !on;
                            }
                        }
                        s.persist();
                        cx.notify();
                    });
                })),
        )
    }
    fn sign_in(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).signing_in.is_some() {
            return;
        }
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.state.update(cx, |s, cx| {
            s.account_revision += 1;
            s.signing_in = Some(cancel.clone());
            s.save_error = None;
            cx.notify();
        });
        let demo = self.state.read(cx).is_demo;
        let task_cancel = cancel.clone();
        let task = cx.background_executor().spawn(async move {
            if demo {
                return Err(anyhow::anyhow!("Sign in is unavailable in demo mode"));
            }
            hermes::account::sign_in(&task_cancel)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.state.update(cx, |s, cx| {
                    if !s
                        .signing_in
                        .as_ref()
                        .is_some_and(|active| std::sync::Arc::ptr_eq(active, &cancel))
                    {
                        return;
                    }
                    s.signing_in = None;
                    match result {
                        Ok(account) => {
                            s.account = Some(account);
                            s.config.transcription = Source::Hosted;
                            if !s.config.onboarded {
                                s.onboarding_step = 1;
                                s.mic_testing = true;
                                s.command(hermes::engine::Command::MicTest(true));
                            }
                            s.persist();
                        }
                        Err(error) => s.save_error = Some(error.to_string()),
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }
    fn account_row(&self, cx: &mut Context<Self>) -> gpui::Div {
        let p = palette(cx);
        let s = self.state.read(cx);
        let row = div().flex().items_center().gap(px(12.)).text_size(px(12.));
        if s.signing_in.is_some() {
            return row
                .child(div().text_color(p.muted).child("Waiting for browser..."))
                .child(
                    button("Cancel", false, p)
                        .id("cancel-sign-in")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.update(cx, |s, cx| {
                                if let Some(cancel) = s.signing_in.take() {
                                    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
                                }
                                cx.notify();
                            });
                        })),
                );
        }
        if let Some(account) = &s.account {
            let limit = account.at_limit();
            let plan = if limit {
                let resets = chrono::DateTime::from_timestamp_millis(account.usage.resets_at)
                    .map(|date| {
                        date.with_timezone(&chrono::Local)
                            .format("%a %-I:%M %p")
                            .to_string()
                    })
                    .unwrap_or_default();
                format!("Weekly limit reached, resets {resets}")
            } else if account.plan == hermes::account::Plan::Pro {
                "Pro".into()
            } else {
                format!(
                    "Free, {} of {} words this week",
                    grouped(account.usage.words),
                    grouped(account.usage.cap.unwrap_or(2000))
                )
            };
            row.child(div().text_color(p.fg).child(account.email.clone()))
                .child(
                    div()
                        .text_color(if limit { p.red } else { p.muted })
                        .child(plan),
                )
                .child(div().flex_1())
                .child(
                    button(if limit { "Upgrade" } else { "Manage" }, false, p)
                        .id("manage-account")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.state.read(cx).is_demo {
                                return;
                            }
                            cx.background_executor()
                                .spawn(async {
                                    let _ = hermes::account::open(format!(
                                        "{}/account",
                                        hermes::account::web_url()
                                    ));
                                })
                                .detach();
                        })),
                )
                .child(
                    button("Sign out", false, p)
                        .id("sign-out")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.update(cx, |s, _| s.account_revision += 1);
                            let mut config = this.state.read(cx).config.clone();
                            let demo = this.state.read(cx).is_demo;
                            let task = cx.background_executor().spawn(async move {
                                if !demo {
                                    hermes::account::sign_out(&mut config)?;
                                }
                                Ok::<_, anyhow::Error>(config.transcription)
                            });
                            cx.spawn(async move |this, cx| {
                                let result = task.await;
                                let _ = this.update(cx, |this, cx| {
                                    this.state.update(cx, |s, cx| {
                                        match result {
                                            Ok(source) => {
                                                s.account = None;
                                                s.config.transcription = source;
                                                s.persist();
                                            }
                                            Err(error) => s.save_error = Some(error.to_string()),
                                        }
                                        cx.notify();
                                    });
                                });
                            })
                            .detach();
                        })),
                )
        } else {
            row.child(div().text_color(p.muted).child("Not signed in"))
                .child(
                    button("Sign in", true, p)
                        .id("sign-in")
                        .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))),
                )
        }
    }
    pub fn settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let s = self.state.read(cx);
        let c = s.config.clone();
        let testing = s.mic_testing;
        let level = s.level;
        let theme = segmented(
            [
                (Theme::System, "System"),
                (Theme::Light, "Light"),
                (Theme::Dark, "Dark"),
            ],
            c.theme,
            p,
            cx,
            |this, value, w, cx| {
                this.state.update(cx, |s, _| s.config.theme = value);
                super::theme::apply(value, w.appearance(), cx);
            },
        );
        let mode = segmented(
            [
                (RecordingMode::Hold, "Hold"),
                (RecordingMode::Toggle, "Toggle"),
            ],
            c.mode,
            p,
            cx,
            |this, value, _, cx| this.state.update(cx, |s, _| s.config.mode = value),
        );
        let source = segmented(
            [
                (Source::Hosted, "Hermes account"),
                (Source::OwnKey, "Own key"),
            ],
            c.transcription,
            p,
            cx,
            |this, value, _, cx| this.state.update(cx, |s, _| s.config.transcription = value),
        );
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header("Settings", p))
            .child(
                div()
                    .id("settings-scroll")
                    .track_scroll(&self.settings_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(INSET))
                    .pb(px(24.))
                    .child(section("Account"))
                    .child(form_row("Account", self.account_row(cx), p))
                    .child(section("Appearance"))
                    .child(form_row("Theme", div().flex().child(theme), p))
                    .child(section("Shortcuts"))
                    .child(form_row(
                        "Dictate",
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(self.shortcut(Shortcut::Dictate, Some(&c.hotkey), cx))
                            .child(mode),
                        p,
                    ))
                    .child(form_row(
                        "Command",
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(self.shortcut(
                                Shortcut::Command,
                                c.command_hotkey.as_deref(),
                                cx,
                            ))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.4))
                                    .text_color(p.muted)
                                    .child("Edits selected text by voice"),
                            ),
                        p,
                    ))
                    .child(form_row(
                        "Paste last",
                        self.shortcut(Shortcut::Paste, c.paste_last_hotkey.as_deref(), cx),
                        p,
                    ))
                    .child(form_row(
                        "Cancel",
                        self.shortcut(Shortcut::Cancel, c.cancel_hotkey.as_deref(), cx),
                        p,
                    ))
                    .child(section("Microphone"))
                    .child(form_row(
                        "Input",
                        div()
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child(select(&self.settings.device, p))
                            .child(div().flex_1())
                            .when(testing, |row| row.child(meter(level, 7, 18., p)))
                            .child(
                                button("Test", false, p)
                                    .id("test-mic")
                                    .on_click(cx.listener(|this, _, _, cx| this.start_mic(cx))),
                            ),
                        p,
                    ))
                    .child(section("Language"))
                    .child(form_row("Spoken", select(&self.settings.spoken, p), p))
                    .child(form_row(
                        "Translate to",
                        select(&self.settings.translation, p),
                        p,
                    ))
                    .child(section("Transcription"))
                    .child(form_row("Source", div().flex().child(source), p))
                    .when(c.transcription == Source::OwnKey, |col| {
                        col.child(form_row("Endpoint", input(&self.settings.endpoint, p), p))
                            .child(form_row("Model", input(&self.settings.model, p), p))
                            .child(form_row("API key", input(&self.settings.key, p), p))
                    })
                    .child(section("Editing"))
                    .child(form_row(
                        "AI editing",
                        self.toggle("editing", c.cleanup.enabled, Toggle::Editing, cx),
                        p,
                    ))
                    .when(c.transcription == Source::OwnKey, |col| {
                        col.child(form_row(
                            "Endpoint",
                            input(&self.settings.cleanup_endpoint, p),
                            p,
                        ))
                        .child(form_row("Model", input(&self.settings.cleanup_model, p), p))
                        .child(form_row(
                            "API key",
                            input(&self.settings.cleanup_key, p),
                            p,
                        ))
                    })
                    .child(section("Behavior"))
                    .child(form_row(
                        "Sound cues",
                        self.toggle("sounds", c.sounds, Toggle::Sounds, cx),
                        p,
                    ))
                    .child(form_row(
                        "Lower media volume",
                        self.toggle("duck", c.duck_media, Toggle::Duck, cx),
                        p,
                    ))
                    .child(form_row(
                        "Press Enter after paste",
                        self.toggle("submit", c.auto_submit, Toggle::Submit, cx),
                        p,
                    ))
                    .child(form_row(
                        "Launch at login",
                        self.toggle("login", c.launch_at_login, Toggle::Login, cx),
                        p,
                    ))
                    .child(section("About"))
                    .child(form_row(
                        "Version",
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.4))
                                    .text_color(p.muted)
                                    .child(env!("CARGO_PKG_VERSION")),
                            )
                            .child(
                                button("Open config folder", false, p)
                                    .id("config-folder")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let folder = this.state.read(cx).paths.config_dir.clone();
                                        cx.background_executor()
                                            .spawn(async move {
                                                let _ = hermes::account::open(folder);
                                            })
                                            .detach();
                                    })),
                            ),
                        p,
                    )),
            )
    }
    pub fn onboarding(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let account_row = self.account_row(cx);
        let p = palette(cx);
        let s = self.state.read(cx);
        let step = s.onboarding_step;
        let title = if step == 2 {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child("Hold")
                .child(
                    kbd(key_label(&s.config.hotkey), p)
                        .text_size(px(16.))
                        .line_height(px(19.)),
                )
                .child("and say hello")
        } else {
            div().child(if step == 0 {
                "How should Hermes transcribe?"
            } else {
                "Check your microphone"
            })
        };
        let mut col = div()
            .w(px(420.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .font_family(MONO)
                    .text_size(px(12.))
                    .line_height(px(14.))
                    .text_color(p.muted)
                    .child(format!("{} / 3", step + 1)),
            )
            .child(
                div()
                    .text_size(px(20.))
                    .line_height(px(29.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(title),
            );
        col = match step {
            0 => col
                .when(s.signing_in.is_some(), |col| col.child(account_row))
                .when(s.signing_in.is_none(), |col| {
                    col.children(
                        [
                            (false, "Sign in, free", "2,000 words a week, no API key"),
                            (true, "Use my own key", "Groq or OpenAI-compatible key"),
                        ]
                        .into_iter()
                        .map(|(own, title, subtitle)| {
                            div()
                                .id(title)
                                .w_full()
                                .h(px(44.))
                                .border_1()
                                .border_color(p.border)
                                .rounded(px(6.))
                                .px(px(12.))
                                .flex()
                                .flex_col()
                                .justify_center()
                                .cursor_pointer()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(p.fg)
                                        .child(title),
                                )
                                .child(div().text_size(px(12.)).text_color(p.muted).child(subtitle))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if own {
                                        this.state.update(cx, |s, cx| {
                                            s.onboarding_own_key = true;
                                            s.config.transcription = Source::OwnKey;
                                            cx.notify();
                                        });
                                    } else {
                                        this.sign_in(cx);
                                    }
                                }))
                        }),
                    )
                })
                .when(s.onboarding_own_key && s.signing_in.is_none(), |col| {
                    col.child(input(&self.settings.key, p))
                }),
            1 => col
                .child(
                    Select::new(&self.settings.device)
                        .small()
                        .w_full()
                        .min_h(px(30.))
                        .max_h(px(30.))
                        .bg(p.bg),
                )
                .child(meter(s.level, 12, 28., p)),
            _ => col.child(
                div()
                    .text_color(if s.onboarding_transcript.is_some() {
                        p.fg
                    } else {
                        p.muted
                    })
                    .child(
                        s.onboarding_transcript
                            .clone()
                            .unwrap_or_else(|| "Waiting for your voice".into()),
                    ),
            ),
        };
        col = col.when_some(s.save_error.clone(), |col, error| {
            col.child(div().text_color(p.red).child(error))
        });
        col =
            col.child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(10.))
                    .mt(px(8.))
                    .when(step == 1, |row| {
                        row.child(button("Back", false, p).id("onboarding-back").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |s, cx| {
                                    s.onboarding_step = 0;
                                    s.mic_testing = false;
                                    s.mic_generation += 1;
                                    s.command(hermes::engine::Command::MicTest(false));
                                    cx.notify();
                                });
                            }),
                        ))
                    })
                    .when(step == 2, |row| {
                        row.child(
                            button("Skip", false, p).id("onboarding-skip").on_click(
                                cx.listener(|this, _, w, cx| this.finish_onboarding(w, cx)),
                            ),
                        )
                    })
                    .when(
                        step != 0 || (s.onboarding_own_key && s.signing_in.is_none()),
                        |row| {
                            row.child(
                                button(if step == 2 { "Done" } else { "Continue" }, true, p)
                                    .id("onboarding-next")
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        if step == 2 {
                                            if this.state.read(cx).onboarding_transcript.is_some() {
                                                this.finish_onboarding(w, cx);
                                            }
                                            return;
                                        }
                                        if step == 0 {
                                            this.save_key(false, w, cx);
                                        }
                                        this.state.update(cx, |s, cx| {
                                            s.onboarding_step += 1;
                                            if step == 1 {
                                                s.mic_testing = false;
                                                s.mic_generation += 1;
                                                s.command(hermes::engine::Command::MicTest(false));
                                            }
                                            cx.notify();
                                        });
                                        if step == 0 {
                                            this.start_mic(cx);
                                        }
                                    })),
                            )
                        },
                    ),
            );
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(col)
    }
    fn finish_onboarding(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.config.onboarded = true;
            s.mic_testing = false;
            s.mic_generation += 1;
            s.command(hermes::engine::Command::MicTest(false));
            s.persist();
            cx.notify();
        });
        self.set_page(Page::Home, w, cx);
    }
}
pub fn meter(level: f32, bars: usize, height: f32, p: Palette) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .h(px(height))
        .children((0..bars).map(|i| {
            let envelope =
                (1. - (i as f32 - (bars as f32 - 1.) * 0.45).abs() / (bars as f32 * 0.6)).max(0.1);
            div()
                .w(px(3.))
                .h(px((height * level.clamp(0., 1.) * envelope).max(3.)))
                .rounded(px(1.5))
                .bg(p.accent)
        }))
}
pub fn shortcut_combo(k: &gpui::Keystroke) -> String {
    let key = match k.key.as_str() {
        "escape" => "ESCAPE",
        "enter" => "ENTER",
        "backspace" => "BACKSPACE",
        "space" => "SPACE",
        "tab" => "TAB",
        "up" => "UP",
        "down" => "DOWN",
        "left" => "LEFT",
        "right" => "RIGHT",
        "pageup" => "PAGEUP",
        "pagedown" => "PAGEDOWN",
        "home" => "HOME",
        "end" => "END",
        "delete" => "DELETE",
        "insert" => "INSERT",
        "-" => "MINUS",
        "=" => "EQUAL",
        "," => "COMMA",
        "." => "DOT",
        "/" => "SLASH",
        ";" => "SEMICOLON",
        "[" => "LEFTBRACE",
        "]" => "RIGHTBRACE",
        "'" => "APOSTROPHE",
        "\\" => "BACKSLASH",
        "`" => "GRAVE",
        "control_r" => "RIGHTCTRL",
        "control_l" => "LEFTCTRL",
        "alt_r" => "RIGHTALT",
        "alt_l" => "LEFTALT",
        other => other,
    };
    // Linux binds evdev codes, which are side-specific; elsewhere either side matches.
    let [ctrl, alt, shift, meta] = if cfg!(target_os = "linux") {
        ["LEFTCTRL", "LEFTALT", "LEFTSHIFT", "LEFTMETA"]
    } else {
        ["CTRL", "ALT", "SHIFT", "META"]
    };
    let mut parts = Vec::new();
    if k.modifiers.control && !key.ends_with("CTRL") {
        parts.push(ctrl.into());
    }
    if k.modifiers.alt && !key.ends_with("ALT") {
        parts.push(alt.into());
    }
    if k.modifiers.shift && !key.ends_with("SHIFT") {
        parts.push(shift.into());
    }
    if k.modifiers.platform && !key.ends_with("META") {
        parts.push(meta.into());
    }
    parts.push(key.to_uppercase());
    parts.join("+")
}

fn grouped(value: u64) -> String {
    let digits = value.to_string();
    digits
        .chars()
        .enumerate()
        .fold(String::new(), |mut result, (index, ch)| {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                result.push(',');
            }
            result.push(ch);
            result
        })
}

fn segmented<T: Copy + PartialEq + 'static, const N: usize>(
    options: [(T, &'static str); N],
    current: T,
    p: Palette,
    cx: &mut Context<MainWindow>,
    select: impl Fn(&mut MainWindow, T, &mut Window, &mut Context<MainWindow>) + Copy + 'static,
) -> gpui::Div {
    div()
        .flex()
        .h(px(26.))
        .border_1()
        .border_color(p.border)
        .rounded(px(6.))
        .overflow_hidden()
        .children(options.map(|(value, label)| {
            let on = current == value;
            div()
                .id(label)
                .flex()
                .items_center()
                .px(px(10.))
                .text_size(px(12.))
                .line_height(px(17.4))
                .font_weight(if on {
                    gpui::FontWeight::MEDIUM
                } else {
                    gpui::FontWeight::NORMAL
                })
                .bg(if on { p.accent } else { p.bg })
                .text_color(if on { p.on_accent } else { p.fg })
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(move |this, _, w, cx| {
                    select(this, value, w, cx);
                    this.state.update(cx, |s, cx| {
                        s.persist();
                        cx.notify();
                    });
                }))
        }))
}
