use super::{
    AppState,
    theme::*,
    window::{MainWindow, Shortcut},
};
use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, Styled, Window, div, point, prelude::*,
    px,
};
use gpui_kit::component::{
    IndexPath, Sizable,
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use typeaside::{
    config::{AppConfig, RecordingMode, Source, Theme},
    local::Status,
};

const FIELD: f32 = 280.;
pub(super) const GROUPS: [&str; 5] = ["Shortcuts", "Language", "Audio", "Dictation", "General"];
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
pub(super) enum Toggle {
    Editing,
    Sounds,
    Duck,
    Submit,
    AutoLearn,
    ScreenContext,
    Login,
    ShareUsage,
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
    stt_models: ModelList,
    cleanup_models: ModelList,
    customize: bool,
    devices: Vec<String>,
    stt_hint: String,
    editing_hint: String,
}
/// Models offered by an own-key endpoint; the text input stays until a list loads.
struct ModelList {
    select: Entity<SelectState<Vec<String>>>,
    fetched_for: String,
    loaded: bool,
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
        let mut models = || ModelList {
            select: select(Vec::new(), None),
            fetched_for: String::new(),
            loaded: false,
        };
        let stt_models = models();
        let cleanup_models = models();
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
            LANGUAGES.iter().map(|(n, _)| (*n).to_owned()).collect(),
            Some(&c.translate_to),
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
            stt_models,
            cleanup_models,
            customize: false,
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
        let endpoints = (s.config.transcription == Source::OwnKey).then(|| {
            [
                (false, s.config.stt.base_url.clone(), stt.clone()),
                (
                    true,
                    s.config.cleanup.endpoint.base_url.clone(),
                    editing.clone(),
                ),
            ]
        });
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
        for (editing, base_url, hint) in endpoints.into_iter().flatten() {
            let list = self.models(editing);
            let fetch = format!("{base_url} {hint}");
            if list.fetched_for == fetch {
                continue;
            }
            list.fetched_for = fetch.clone();
            list.loaded = false;
            let task = cx
                .background_executor()
                .spawn(async move { typeaside::http::models(&base_url) });
            cx.spawn_in(w, async move |this, cx| {
                let Ok((speech, chat)) = task.await else {
                    return;
                };
                let _ = this.update_in(cx, |this, w, cx| {
                    let c = &this.state.read(cx).config;
                    let current = if editing {
                        c.cleanup.endpoint.model.clone()
                    } else {
                        c.stt.model.clone()
                    };
                    let mut ids = if editing { chat } else { speech };
                    let list = this.settings.models(editing);
                    if list.fetched_for != fetch || ids.is_empty() {
                        return;
                    }
                    if !current.is_empty() && !ids.contains(&current) {
                        ids.insert(0, current.clone());
                    }
                    list.select.update(cx, |v, cx| {
                        v.set_items(ids, w, cx);
                        v.set_selected_value(&current, w, cx);
                    });
                    list.loaded = true;
                    cx.notify();
                });
            })
            .detach();
        }
    }
    fn models(&mut self, editing: bool) -> &mut ModelList {
        if editing {
            &mut self.cleanup_models
        } else {
            &mut self.stt_models
        }
    }
}
pub fn input(state: &Entity<InputState>, p: Palette) -> impl IntoElement {
    Input::new(state)
        .min_h(px(30.))
        .max_h(px(30.))
        .w(px(FIELD))
        .rounded(px(6.))
        .text_size(px(13.))
        .bg(p.bg)
}
fn model_field(list: &ModelList, fallback: &Entity<InputState>, p: Palette) -> gpui::AnyElement {
    if list.loaded {
        select(&list.select, p).into_any_element()
    } else {
        input(fallback, p).into_any_element()
    }
}
fn select(state: &Entity<SelectState<Vec<String>>>, p: Palette) -> Select<Vec<String>> {
    Select::new(state)
        .small()
        .min_h(px(30.))
        .max_h(px(30.))
        .w(px(FIELD))
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
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
                if !editing
                    && !this.state.read(cx).config.onboarded
                    && this.state.read(cx).onboarding_step == super::onboarding::Step::Account
                {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.onboarding_next(false, w, cx);
                    }
                    return;
                }
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.save_key(editing, w, cx);
                }
            })
            .detach();
        }
        for (entity, editing) in [
            (&self.settings.stt_models.select, false),
            (&self.settings.cleanup_models.select, true),
        ] {
            cx.subscribe_in(entity, window, move |this, _, event, w, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let input = if editing {
                    &this.settings.cleanup_model
                } else {
                    &this.settings.model
                };
                input.update(cx, |v, cx| v.set_value(value.clone(), w, cx));
                this.state.update(cx, |s, cx| {
                    if editing {
                        s.config.cleanup.endpoint.model = value.clone();
                    } else {
                        s.config.stt.model = value.clone();
                    }
                    s.persist();
                    cx.notify();
                });
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
                        Selection::Translation => s.config.translate_to = value,
                    }
                    s.persist();
                    if matches!(field, Selection::Device) && s.mic_testing {
                        s.command(if !s.config.onboarded {
                            typeaside::engine::Command::MicPreview
                        } else {
                            typeaside::engine::Command::MicTest(true)
                        });
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        self.settings.refresh(&self.state, window, cx);
    }
    pub(super) fn save_key(&mut self, editing: bool, window: &mut Window, cx: &mut Context<Self>) {
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
            let host = typeaside::credentials::endpoint_host(&endpoint)?;
            if !demo {
                typeaside::credentials::save_credential(&host, &key)?;
            }
            Ok::<_, anyhow::Error>(host)
        });
        if !editing {
            self.state.update(cx, |s, _| {
                s.key_hint = "Saving…".into();
                s.save_error = None;
            });
        }
        entity.update(cx, |input, cx| input.set_value("", window, cx));
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            let result = task.await;
            state.update(cx, |s, cx| {
                if let Ok(host) = result {
                    if typeaside::credentials::endpoint_host(&s.config.stt.base_url)
                        .ok()
                        .as_ref()
                        == Some(&host)
                    {
                        s.key_hint = "Saved in keyring".into();
                    }
                    if typeaside::credentials::endpoint_host(&s.config.cleanup.endpoint.base_url)
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
                Shortcut::Translate => s.config.translate_hotkey = Some(key),
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
                Shortcut::Translate => "translate-key",
            })
            .cursor_pointer()
            .child(if self.capture == Some(target) {
                div()
                    .text_color(p.faint)
                    .child("Press keys…")
                    .into_any_element()
            } else {
                kbd(value.unwrap_or_default(), p).into_any_element()
            })
            .on_click(cx.listener(move |this, _, w, cx| {
                this.capture = Some(target);
                this.capture_modifiers = gpui::Modifiers::default();
                w.focus(&this.focus, cx);
                cx.notify();
            }))
    }
    pub(super) fn toggle(
        &self,
        id: &'static str,
        on: bool,
        field: Toggle,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette(cx);
        let onboarding = !self.state.read(cx).config.onboarded;
        div().flex().justify_end().child(
            div()
                .id(id)
                .w(px(30.))
                .h(px(18.))
                .rounded_full()
                .relative()
                .bg(if on {
                    p.accent
                } else if onboarding {
                    p.kbd
                } else {
                    p.border
                })
                .cursor_pointer()
                .child(
                    div()
                        .absolute()
                        .top(px(2.))
                        .left(px(if on { 14. } else { 2. }))
                        .size(px(14.))
                        .rounded_full()
                        .bg(if on {
                            p.on_accent
                        } else if onboarding {
                            p.fg
                        } else {
                            p.bg
                        }),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.state.update(cx, |s, cx| {
                        match field {
                            Toggle::Editing => s.config.cleanup.enabled = !on,
                            Toggle::Sounds => s.config.sounds = !on,
                            Toggle::Duck => s.config.duck_media = !on,
                            Toggle::Submit => s.config.auto_submit = !on,
                            Toggle::AutoLearn => s.config.auto_learn = !on,
                            Toggle::ScreenContext => s.config.screen_context = !on,
                            Toggle::ShareUsage => {
                                if let Err(error) = s.config.set_share_usage(!on) {
                                    s.save_error = Some(error.to_string());
                                    cx.notify();
                                    return;
                                }
                            }
                            Toggle::Login => {
                                if !s.is_demo
                                    && let Err(e) = typeaside::autostart::set_enabled(!on)
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
    pub(super) fn sign_in(&mut self, cx: &mut Context<Self>) {
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
            typeaside::account::sign_in(&task_cancel)
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
                            s.account_revision += 1;
                            s.account = Some(account);
                            s.config.transcription = Source::Hosted;
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
    pub fn account_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        page(
            FORM_WIDTH,
            header("Account"),
            div().id("account-scroll").child(
                column(FORM_WIDTH)
                    .pt(px(14.))
                    .child(panel(self.account(cx), p)),
            ),
            p,
        )
    }
    fn account(&self, cx: &mut Context<Self>) -> Vec<Setting> {
        let p = palette(cx);
        let s = self.state.read(cx);
        if s.signing_in.is_some() {
            return vec![
                Setting::new("Account")
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
                    ),
            ];
        }
        let Some(account) = &s.account else {
            return vec![
                Setting::new("Account")
                    .description("Sign in to use hosted transcription and sync.")
                    .child(
                        button("Sign in", true, p)
                            .id("sign-in")
                            .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))),
                    ),
            ];
        };
        let limit = account.at_limit();
        let plan = if limit {
            let resets = chrono::DateTime::from_timestamp_millis(account.usage.resets_at)
                .map(|date| {
                    date.with_timezone(&chrono::Local)
                        .format(&format!("%a {}", super::clock::time_format()))
                        .to_string()
                })
                .unwrap_or_default();
            format!("Weekly limit reached, resets {resets}")
        } else if account.plan == typeaside::account::Plan::Pro {
            "Pro".into()
        } else {
            format!(
                "Free, {} of {} words this week",
                grouped(account.usage.words),
                grouped(account.usage.cap.unwrap_or(2000))
            )
        };
        vec![
            Setting::new("Plan")
                .description(div().when(limit, |plan| plan.text_color(p.red)).child(plan))
                .child(
                    button(if limit { "Upgrade" } else { "Manage" }, false, p)
                        .id("manage-account")
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
            Setting::new("Sync")
                .description("Dictionary, snippets and styles sync across your devices."),
            Setting::new("Email")
                .description(account.email.clone())
                .child(
                    button("Sign out", false, p)
                        .id("sign-out")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.update(cx, |s, _| s.account_revision += 1);
                            let mut config = this.state.read(cx).config.clone();
                            let demo = this.state.read(cx).is_demo;
                            let task = cx.background_executor().spawn(async move {
                                if !demo {
                                    typeaside::account::sign_out(&mut config)?;
                                }
                                Ok::<_, anyhow::Error>(config.transcription)
                            });
                            cx.spawn(async move |this, cx| {
                                let result = task.await;
                                let _ = this.update(cx, |this, cx| {
                                    this.state.update(cx, |s, cx| {
                                        match result {
                                            Ok(source) => {
                                                s.account_revision += 1;
                                                s.account = None;
                                                s.config.settings_sync = None;
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
                ),
        ]
    }
    fn download_model(&mut self, cx: &mut Context<Self>) {
        let cancel = Arc::new(AtomicBool::new(false));
        let (progress_tx, progress_rx) = async_channel::unbounded();
        let task = cx.background_executor().spawn({
            let cancel = cancel.clone();
            async move {
                typeaside::local::download(&cancel, move |percent| {
                    let _ = progress_tx.try_send(percent);
                })
            }
        });
        self.state.update(cx, |s, cx| {
            s.model = Status::Downloading(0);
            s.model_cancel = Some(cancel);
            cx.notify();
        });
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            while let Ok(percent) = progress_rx.recv().await {
                state.update(cx, |s, cx| {
                    s.model = Status::Downloading(percent);
                    cx.notify();
                });
            }
            let result = task.await;
            state.update(cx, |s, cx| {
                let cancelled = s
                    .model_cancel
                    .take()
                    .is_some_and(|cancel| cancel.load(Ordering::Relaxed));
                s.model = match result {
                    Ok(()) => Status::Installed,
                    Err(_) if cancelled => Status::NotInstalled,
                    Err(error) => {
                        eprintln!("typeaside model download: {error:#}");
                        Status::Failed
                    }
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn cancel_model(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = &self.state.read(cx).model_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    fn remove_model(&mut self, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let task = cx
            .background_executor()
            .spawn(async { typeaside::local::remove() });
        cx.spawn(async move |_, cx| {
            let result = task.await;
            state.update(cx, |s, cx| {
                match result {
                    Ok(()) => {
                        s.model = Status::NotInstalled;
                        if s.config.transcription == Source::Local {
                            s.config.transcription = if s.account.is_some() {
                                Source::Hosted
                            } else {
                                Source::OwnKey
                            };
                            s.persist();
                        }
                    }
                    Err(error) => s.save_error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Sets the offset directly rather than through `scroll_to_top_of_item`, which applies after
    /// this frame renders and would leave the old link highlighted. The clicked group stays
    /// highlighted until the offset changes, since a group near the end may not reach the top.
    fn jump(&mut self, group: usize, cx: &mut Context<Self>) {
        let scroll = &self.settings_scroll;
        if let Some(bounds) = scroll.bounds_for_item(group) {
            let y = (scroll.bounds().top() - bounds.top()).clamp(-scroll.max_offset().y, px(0.));
            scroll.set_offset(point(px(0.), y));
            self.settings_jump = Some((group, y));
            cx.notify();
        }
    }
    pub fn settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let s = self.state.read(cx);
        let c = s.config.clone();
        let testing = s.mic_testing;
        let level = s.level;
        let model = s.model;
        // Account users get transcription and editing handled for them; the knobs sit behind Customize.
        let managed = s.account.is_some() && c.transcription == Source::Hosted;
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
        let (model_text, model_label, model_action): (_, _, fn(&mut Self, &mut Context<Self>)) =
            match model {
                Status::NotInstalled => (
                    format!(
                        "Parakeet v3, 25 languages, {} MB",
                        typeaside::local::download_size_mb()
                    ),
                    "Download",
                    Self::download_model,
                ),
                Status::Downloading(percent) => (
                    format!("Downloading {percent}%"),
                    "Cancel",
                    Self::cancel_model,
                ),
                Status::Installed => (
                    "Installed, works offline".into(),
                    "Remove",
                    Self::remove_model,
                ),
                Status::Failed => (
                    "Download failed, check your connection".into(),
                    "Retry",
                    Self::download_model,
                ),
            };
        let source = segmented(
            [
                (Source::Hosted, "TypeAside account"),
                (Source::OwnKey, "Own key"),
            ]
            .into_iter()
            .chain((model == Status::Installed).then_some((Source::Local, "On device"))),
            c.transcription,
            p,
            cx,
            |this, value, _, cx| this.state.update(cx, |s, _| s.config.transcription = value),
        );
        let model_row = Setting::new("On-device model")
            .description(
                div()
                    .when(model == Status::Failed, |text| text.text_color(p.red))
                    .child(model_text),
            )
            .child(
                button(model_label, false, p)
                    .id("model-action")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.state.read(cx).is_demo {
                            model_action(this, cx);
                        }
                    })),
            );
        let own_key = c.transcription == Source::OwnKey;
        let mut transcription = Vec::new();
        if managed {
            transcription.push(
                Setting::new("Transcription")
                    .description("Handled by your TypeAside account")
                    .child(
                        button(
                            if self.settings.customize {
                                "Done"
                            } else {
                                "Customize"
                            },
                            false,
                            p,
                        )
                        .id("customize")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.settings.customize = !this.settings.customize;
                            cx.notify();
                        })),
                    ),
            );
        }
        if !managed || self.settings.customize {
            transcription.push(
                Setting::new("Source")
                    .when(c.transcription == Source::Local, |row| {
                        row.description(
                            "Audio never leaves this computer. Cleanup and translation are off.",
                        )
                    })
                    .child(source),
            );
            if typeaside::local::AVAILABLE {
                transcription.push(model_row);
            }
            if own_key {
                transcription.extend([
                    Setting::new("Endpoint").child(input(&self.settings.endpoint, p)),
                    Setting::new("Model").child(model_field(
                        &self.settings.stt_models,
                        &self.settings.model,
                        p,
                    )),
                    Setting::new("API key").child(input(&self.settings.key, p)),
                ]);
            }
        }
        let mut models = vec![panel(transcription, p)];
        if c.transcription != Source::Hosted {
            let mut editing = vec![
                Setting::new("AI editing")
                    .description("Clean up fillers, punctuation and lists.")
                    .child(self.toggle("editing", c.cleanup.enabled, Toggle::Editing, cx)),
            ];
            if own_key {
                editing.extend([
                    Setting::new("Endpoint").child(input(&self.settings.cleanup_endpoint, p)),
                    Setting::new("Model").child(model_field(
                        &self.settings.cleanup_models,
                        &self.settings.cleanup_model,
                        p,
                    )),
                    Setting::new("API key").child(input(&self.settings.cleanup_key, p)),
                ]);
            }
            models.push(panel(editing, p));
        }
        let shortcuts = vec![panel(
            [
                Setting::new("Dictate")
                    .description("Hold to talk, or tap to start and stop.")
                    .child(self.shortcut(Shortcut::Dictate, Some(&c.hotkey), cx))
                    .child(mode),
                Setting::new("Translate")
                    .description("Translate speech, or tap over selected text.")
                    .child(self.shortcut(Shortcut::Translate, c.translate_hotkey.as_deref(), cx)),
                Setting::new("Ask anything")
                    .description("Ask a question, or edit selected text.")
                    .child(self.shortcut(Shortcut::Command, c.command_hotkey.as_deref(), cx)),
            ],
            p,
        )];
        let language = vec![panel(
            [
                Setting::new("Spoken")
                    .description("The language you dictate in.")
                    .child(select(&self.settings.spoken, p)),
                Setting::new("Translate to")
                    .description("Language for the Translate shortcut.")
                    .child(select(&self.settings.translation, p)),
            ],
            p,
        )];
        let audio = vec![panel(
            [
                Setting::new("Microphone")
                    .when(testing, |row| row.child(meter(level, 7, 18., p)))
                    .child(select(&self.settings.device, p).w(px(FIELD - 60.)))
                    .child(
                        button("Test", false, p)
                            .id("test-mic")
                            .on_click(cx.listener(|this, _, _, cx| this.start_mic(cx))),
                    ),
                Setting::new("Sound cues")
                    .description("Short tones when recording starts and stops.")
                    .child(self.toggle("sounds", c.sounds, Toggle::Sounds, cx)),
                Setting::new("Lower media volume")
                    .description("Duck music and video while you dictate.")
                    .child(self.toggle("duck", c.duck_media, Toggle::Duck, cx)),
            ],
            p,
        )];
        let behavior = panel(
            [
                Setting::new("Press Enter after paste")
                    .description("Send chat messages right away.")
                    .child(self.toggle("submit", c.auto_submit, Toggle::Submit, cx)),
                Setting::new("Use names on screen")
                    .description("Pass visible names to transcription for this dictation.")
                    .child(self.toggle(
                        "screen-context",
                        c.screen_context,
                        Toggle::ScreenContext,
                        cx,
                    )),
                Setting::new("Learn words from your edits")
                    .description("Add fixed words to your dictionary.")
                    .child(self.toggle("auto-learn", c.auto_learn, Toggle::AutoLearn, cx)),
            ],
            p,
        );
        let dictation = std::iter::once(behavior).chain(models).collect::<Vec<_>>();
        let general = vec![panel(
            [
                Setting::new("Appearance").child(theme),
                Setting::new("Launch at login")
                    .description("Start hidden when you log in.")
                    .child(self.toggle("login", c.launch_at_login, Toggle::Login, cx)),
                Setting::new("Share anonymous usage stats")
                    .description("Counts and timings only, never text or audio.")
                    .child(self.toggle("share-usage", c.share_usage, Toggle::ShareUsage, cx)),
                Setting::new("Version")
                    .description(env!("CARGO_PKG_VERSION"))
                    .child(
                        button("Open config folder", false, p)
                            .id("config-folder")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let folder = this.state.read(cx).paths.config_dir.clone();
                                cx.background_executor()
                                    .spawn(async move {
                                        let _ = typeaside::account::open(folder);
                                    })
                                    .detach();
                            })),
                    ),
            ],
            p,
        )];
        let scroll = &self.settings_scroll;
        let top = scroll.bounds().top() - scroll.offset().y;
        let bottom = -scroll.max_offset().y;
        // A short last group can never reach the top, so the end of the page selects it.
        let active = if let Some((group, _)) =
            (self.settings_jump).filter(|&(_, y)| y == scroll.offset().y)
        {
            group
        } else if bottom < px(0.) && scroll.offset().y <= bottom {
            GROUPS.len() - 1
        } else {
            (0..GROUPS.len())
                .rev()
                .find(|&i| {
                    scroll
                        .bounds_for_item(i)
                        .is_some_and(|group| group.top() <= top + px(1.))
                })
                .unwrap_or(0)
        };
        page(
            FORM_WIDTH,
            header("Settings").child(div().flex().gap(px(4.)).children(
                GROUPS.into_iter().enumerate().map(|(index, title)| {
                    div()
                        .id(title)
                        .px(px(10.))
                        .py(px(3.))
                        .rounded(px(6.))
                        .text_size(px(12.))
                        .line_height(px(17.4))
                        .cursor_pointer()
                        .when(index == active, |link| link.bg(p.tint))
                        .text_color(if index == active { p.fg } else { p.muted })
                        .hover(|link| link.text_color(p.fg))
                        .child(title)
                        .on_click(cx.listener(move |this, _, _, cx| this.jump(index, cx)))
                }),
            )),
            div()
                .id("settings-scroll")
                .track_scroll(&self.settings_scroll)
                .pb(px(24.))
                .children(
                    GROUPS
                        .into_iter()
                        .zip([shortcuts, language, audio, dictation, general])
                        .map(|(title, panels)| {
                            column(FORM_WIDTH)
                                .child(
                                    div()
                                        .pt(px(24.))
                                        .text_size(px(15.))
                                        .line_height(px(21.75))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(title),
                                )
                                .children(panels)
                        }),
                ),
            p,
        )
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
#[cfg(not(target_os = "linux"))]
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
    let [ctrl, alt, shift, meta] = ["CTRL", "ALT", "SHIFT", "META"];
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

pub(super) fn grouped(value: u64) -> String {
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

fn segmented<T: Copy + PartialEq + 'static>(
    options: impl IntoIterator<Item = (T, &'static str)>,
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
        .children(options.into_iter().map(|(value, label)| {
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
