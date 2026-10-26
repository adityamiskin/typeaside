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
    config::{
        AppConfig, HistoryRetention, LocalUnload, MediaControl, RecordingMode, Source,
        SpeechSource, Theme,
    },
    local::{CleanupModel, ModelId, Status},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DownloadTarget {
    Speech(ModelId),
    Cleanup,
}
impl DownloadTarget {
    fn set_status(self, state: &mut AppState, status: Status) {
        match self {
            Self::Speech(model) => state.models[model.index()] = status,
            Self::Cleanup => state.cleanup_model = status,
        }
    }
    fn download(self, cancel: &AtomicBool, progress: impl Fn(u8)) -> anyhow::Result<()> {
        match self {
            Self::Speech(model) => typeaside::local::download(model, cancel, progress),
            Self::Cleanup => CleanupModel::download(cancel, progress),
        }
    }
}

const HISTORY_RETENTION: [(HistoryRetention, &str); 4] = [
    (HistoryRetention::Forever, "Until full"),
    (HistoryRetention::Month, "30 days"),
    (HistoryRetention::Week, "7 days"),
    (HistoryRetention::Off, "Off"),
];
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
    OnDictation,
}
#[derive(Clone, Copy)]
enum Selection {
    Spoken,
    Translation,
    HistoryRetention,
}
#[derive(Clone, Copy)]
pub(super) enum Toggle {
    MicrophoneReady,
    Editing,
    Sounds,
    Submit,
    SendCommand,
    AutoLearn,
    ScreenContext,
    Login,
}

pub struct SettingsInputs {
    pub endpoint: Entity<InputState>,
    pub model: Entity<InputState>,
    pub key: Entity<InputState>,
    pub cleanup_endpoint: Entity<InputState>,
    pub cleanup_model: Entity<InputState>,
    pub cleanup_key: Entity<InputState>,
    pub on_dictation: Entity<InputState>,
    pub device: Entity<SelectState<Vec<String>>>,
    pub spoken: Entity<SelectState<Vec<String>>>,
    pub translation: Entity<SelectState<Vec<String>>>,
    pub history_retention: Entity<SelectState<Vec<String>>>,
    stt_models: ModelList,
    cleanup_models: ModelList,
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
        let on_dictation = text(c.on_dictation.as_deref().unwrap_or_default(), false);
        on_dictation.update(cx, |input, cx| {
            input.set_placeholder("notify-send \"$(jq -r .cleaned)\"", w, cx);
        });
        let available: Vec<_> = devices
            .iter()
            .filter(|name| !c.input_devices.contains(name))
            .cloned()
            .collect();
        let device = cx.new(|cx| SelectState::new(available.clone(), None, w, cx));
        let mut select = |values: Vec<String>, selected: Option<&str>| {
            let index = selected
                .and_then(|s| values.iter().position(|v| v == s))
                .unwrap_or(0);
            cx.new(|cx| SelectState::new(values, Some(IndexPath::default().row(index)), w, cx))
        };
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
        let history_retention = select(
            HISTORY_RETENTION
                .iter()
                .map(|(_, label)| (*label).to_owned())
                .collect(),
            HISTORY_RETENTION
                .iter()
                .find(|(value, _)| *value == c.history_retention)
                .map(|(_, label)| *label),
        );
        Self {
            endpoint,
            model,
            key,
            cleanup_endpoint,
            cleanup_model,
            cleanup_key,
            on_dictation,
            device,
            spoken,
            translation,
            history_retention,
            stt_models,
            cleanup_models,
            devices: available,
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
        let endpoints = [
            (s.config.transcription == SpeechSource::OwnKey)
                .then(|| (false, s.config.stt.base_url.clone(), stt.clone())),
            Some((
                true,
                s.config.cleanup.endpoint.base_url.clone(),
                editing.clone(),
            )),
        ];
        let available: Vec<_> = s
            .devices
            .iter()
            .filter(|name| !s.config.input_devices.contains(name))
            .cloned()
            .collect();
        if self.devices != available {
            self.devices = available.clone();
            self.device.update(cx, |v, cx| {
                v.set_items(available, w, cx);
                v.set_selected_index(None, w, cx);
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
    pub(super) fn connect_wispr(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).is_demo {
            return;
        }
        if let Some(cancel) = self.state.update(cx, |s, _| s.wispr_login.take()) {
            cancel.store(true, Ordering::Relaxed);
            cx.notify();
            return;
        }
        let login = match typeaside::wispr::Login::new() {
            Ok(login) => login,
            Err(error) => {
                self.state.update(cx, |s, cx| {
                    s.save_error = Some(error.to_string());
                    cx.notify();
                });
                return;
            }
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.state.update(cx, |s, cx| {
            s.wispr_login = Some(cancel.clone());
            s.save_error = None;
            cx.notify();
        });
        cx.open_url(&login.url());
        let worker_cancel = cancel.clone();
        let task = cx
            .background_executor()
            .spawn(async move { login.wait(&worker_cancel) });
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            let result = task.await;
            state.update(cx, |s, cx| {
                if !s
                    .wispr_login
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &cancel))
                {
                    return;
                }
                s.wispr_login = None;
                match result {
                    Ok(account) => {
                        s.wispr_account = Some(account);
                        s.persist();
                    }
                    Err(error) => s.save_error = Some(format!("Wispr sign-in: {error}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn disconnect_wispr(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).is_demo {
            return;
        }
        if let Some(cancel) = self.state.update(cx, |s, _| s.wispr_login.take()) {
            cancel.store(true, Ordering::Relaxed);
        }
        let task = cx
            .background_executor()
            .spawn(async { typeaside::wispr::disconnect() });
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            let result = task.await;
            state.update(cx, |s, cx| {
                match result {
                    Ok(()) => s.wispr_account = None,
                    Err(error) => s.save_error = Some(format!("Cannot disconnect Wispr: {error}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn bind_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (entity, field) in [
            (&self.settings.endpoint, TextField::Endpoint),
            (&self.settings.model, TextField::Model),
            (&self.settings.cleanup_endpoint, TextField::EditingEndpoint),
            (&self.settings.cleanup_model, TextField::EditingModel),
            (&self.settings.on_dictation, TextField::OnDictation),
        ] {
            cx.subscribe(entity, move |this, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    if let TextField::Endpoint | TextField::EditingEndpoint = field {
                        this.refresh_key_hint(
                            matches!(field, TextField::EditingEndpoint),
                            value.clone(),
                            cx,
                        );
                    }
                    this.state.update(cx, |s, cx| {
                        match field {
                            TextField::Endpoint => s.config.stt.base_url = value,
                            TextField::Model => s.config.stt.model = value,
                            TextField::EditingEndpoint => {
                                s.config.cleanup.endpoint.base_url = value
                            }
                            TextField::EditingModel => s.config.cleanup.endpoint.model = value,
                            TextField::OnDictation => {
                                s.config.on_dictation = (!value.trim().is_empty()).then_some(value)
                            }
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
                if !this.state.read(cx).config.onboarded {
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
        cx.subscribe(&self.settings.device, |this, _, event, cx| {
            let SelectEvent::Confirm(Some(value)) = event else {
                return;
            };
            let value = value.clone();
            this.update_microphones(cx, |devices| {
                if !devices.contains(&value) {
                    devices.push(value);
                }
            });
        })
        .detach();
        for (entity, field) in [
            (&self.settings.spoken, Selection::Spoken),
            (&self.settings.translation, Selection::Translation),
            (
                &self.settings.history_retention,
                Selection::HistoryRetention,
            ),
        ] {
            cx.subscribe(entity, move |this, _, event, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = value.clone();
                this.state.update(cx, |s, cx| {
                    match field {
                        Selection::Spoken => {
                            s.config.language = LANGUAGES
                                .iter()
                                .find(|(n, _)| *n == value)
                                .map(|(_, code)| (*code).into())
                        }
                        Selection::Translation => s.config.translate_to = value,
                        Selection::HistoryRetention => {
                            let Some((retention, _)) =
                                HISTORY_RETENTION.iter().find(|(_, label)| *label == value)
                            else {
                                return;
                            };
                            s.config.history_retention = *retention;
                            typeaside::engine::prune_history(
                                &mut s.history,
                                *retention,
                                std::time::SystemTime::now(),
                            );
                            s.expanded = None;
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
    /// The keyring hint follows the endpoint, since keys are stored per host.
    fn refresh_key_hint(&self, editing: bool, endpoint: String, cx: &mut Context<Self>) {
        if self.state.read(cx).is_demo {
            return;
        }
        let task = cx.background_executor().spawn({
            let endpoint = endpoint.clone();
            async move { super::key_hint(&endpoint) }
        });
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            let hint = task.await;
            state.update(cx, |s, cx| {
                let (current, slot) = if editing {
                    (&s.config.cleanup.endpoint.base_url, &mut s.editing_key_hint)
                } else {
                    (&s.config.stt.base_url, &mut s.key_hint)
                };
                if *current == endpoint {
                    *slot = hint;
                    cx.notify();
                }
            });
        })
        .detach();
    }
    fn update_microphones(
        &mut self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut Vec<String>),
    ) {
        self.state.update(cx, |s, cx| {
            update(&mut s.config.input_devices);
            s.persist();
            if s.mic_testing {
                s.command(if !s.config.onboarded {
                    typeaside::engine::Command::MicPreview
                } else {
                    typeaside::engine::Command::MicTest(true)
                });
            }
            cx.notify();
        });
    }

    pub(super) fn microphones(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let s = self.state.read(cx);
        let preferences = &s.config.input_devices;
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(6.))
            .when(preferences.is_empty(), |rows| {
                rows.child(div().text_color(p.muted).child("System default"))
            })
            .children(preferences.iter().enumerate().map(|(index, name)| {
                let connected = s.devices.contains(name);
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_h(px(30.))
                    .child(
                        div()
                            .w(px(18.))
                            .text_color(p.muted)
                            .child((index + 1).to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(if connected { p.fg } else { p.muted })
                            .child(if connected {
                                name.clone()
                            } else {
                                format!("{name} (not connected)")
                            }),
                    )
                    .children(
                        [
                            ("up", "icons/chevron-up.svg", index > 0),
                            (
                                "down",
                                "icons/chevron-down.svg",
                                index + 1 < preferences.len(),
                            ),
                            ("remove", "icons/trash.svg", true),
                        ]
                        .into_iter()
                        .map(|(action, icon, enabled)| {
                            div()
                                .id((action, index))
                                .size(px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    gpui::svg().path(icon).size(px(14.)).text_color(if enabled {
                                        p.muted
                                    } else {
                                        p.faint
                                    }),
                                )
                                .when(enabled, |button| {
                                    button.cursor_pointer().hover(|b| b.bg(p.hover)).on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.update_microphones(cx, |devices| match action {
                                                "up" => devices.swap(index, index - 1),
                                                "down" => devices.swap(index, index + 1),
                                                _ => {
                                                    devices.remove(index);
                                                }
                                            });
                                        }),
                                    )
                                })
                        }),
                    )
            }))
            .child(select(&self.settings.device, p).placeholder("Add microphone"))
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
        self.state.update(cx, |s, _| {
            if editing {
                s.editing_key_hint = "Saving…".into();
            } else {
                s.key_hint = "Saving…".into();
            }
            s.save_error = None;
        });
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
                    let hint = if editing {
                        &mut s.editing_key_hint
                    } else {
                        &mut s.key_hint
                    };
                    *hint = "Enter API key".into();
                    s.save_error = Some("Could not save API key in keyring".into());
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub fn record_shortcut(&mut self, key: String, cx: &mut Context<Self>) {
        let Some(target) = self.capture else { return };
        let now = std::time::Instant::now();
        let down = self.shortcut_down.take().unwrap_or(now);
        let quick = now.duration_since(down).as_millis() < 300 && !key.contains('+');
        if quick
            && self.shortcut_tap.as_ref().is_some_and(|(first, at)| {
                first == &key && now.duration_since(*at).as_millis() <= 350
            })
        {
            self.shortcut_task = None;
            self.shortcut_tap = None;
            self.capture = None;
            self.save_shortcut(target, format!("{key}*2"), cx);
            return;
        }
        if !quick {
            self.capture = None;
            self.save_shortcut(target, key, cx);
            return;
        }
        self.shortcut_tap = Some((key.clone(), down));
        self.shortcut_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(
                    std::time::Duration::from_millis(350).saturating_sub(now.duration_since(down)),
                )
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.capture == Some(target) {
                    this.capture = None;
                    this.shortcut_tap = None;
                    this.save_shortcut(target, key, cx);
                }
            });
        }));
    }
    pub fn save_shortcut(&mut self, target: Shortcut, key: String, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            match target {
                Shortcut::Dictate => s.config.hotkey = key,
                Shortcut::Command => s.config.command_hotkey = Some(key),
                Shortcut::Translate => s.config.translate_hotkey = Some(key),
                Shortcut::Pause => s.config.pause_hotkey = Some(key),
                Shortcut::Note => s.config.note_hotkey = Some(key),
                Shortcut::PasteLast => s.config.paste_last_hotkey = Some(key),
            }
            s.persist();
            cx.notify();
        });
        typeaside::hotkeys::set_shortcut_recording(false);
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
                Shortcut::Pause => "pause-key",
                Shortcut::Note => "note-key",
                Shortcut::PasteLast => "paste-last-key",
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
            .when(
                matches!(target, Shortcut::Pause | Shortcut::PasteLast) && value.is_some(),
                |el| {
                    el.child(
                        div()
                            .id(match target {
                                Shortcut::PasteLast => "clear-paste-last-key",
                                _ => "clear-pause-key",
                            })
                            .ml(px(8.))
                            .text_color(p.muted)
                            .child("×")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.capture = None;
                                this.state.update(cx, |s, cx| {
                                    match target {
                                        Shortcut::PasteLast => s.config.paste_last_hotkey = None,
                                        _ => s.config.pause_hotkey = None,
                                    }
                                    s.persist();
                                    cx.notify();
                                });
                                cx.notify();
                            })),
                    )
                },
            )
            .on_click(cx.listener(move |this, _, w, cx| {
                typeaside::hotkeys::set_shortcut_recording(true);
                this.shortcut_down = None;
                this.shortcut_tap = None;
                this.shortcut_task = None;
                this.shortcut_key = None;
                this.capture = Some(target);
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
                            Toggle::MicrophoneReady => s.config.keep_microphone_ready = !on,
                            Toggle::Sounds => s.config.sounds = !on,
                            Toggle::Submit => s.config.auto_submit = !on,
                            Toggle::SendCommand => s.config.send_command = !on,
                            Toggle::AutoLearn => s.config.auto_learn = !on,
                            Toggle::ScreenContext => s.config.screen_context = !on,
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
    /// Downloads run one at a time; a request during another download waits its turn.
    pub(super) fn download_model(&mut self, model: DownloadTarget, cx: &mut Context<Self>) {
        if self.state.read(cx).model_cancel.is_some() {
            self.state.update(cx, |s, _| {
                if !s.download_queue.contains(&model) {
                    s.download_queue.push(model);
                }
            });
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let (progress_tx, progress_rx) = async_channel::unbounded();
        let task = cx.background_executor().spawn({
            let cancel = cancel.clone();
            async move {
                model.download(&cancel, move |percent| {
                    let _ = progress_tx.try_send(percent);
                })
            }
        });
        self.state.update(cx, |s, cx| {
            model.set_status(s, Status::Downloading(0));
            s.model_cancel = Some(cancel);
            cx.notify();
        });
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            while let Ok(percent) = progress_rx.recv().await {
                state.update(cx, |s, cx| {
                    model.set_status(s, Status::Downloading(percent));
                    cx.notify();
                });
            }
            let result = task.await;
            state.update(cx, |s, cx| {
                let cancelled = s
                    .model_cancel
                    .take()
                    .is_some_and(|cancel| cancel.load(Ordering::Relaxed));
                let status = match result {
                    Ok(()) => {
                        if let DownloadTarget::Speech(model) = model
                            && !s.config.local_model.installed()
                        {
                            s.config.local_model = model;
                            s.persist();
                        }
                        Status::Installed
                    }
                    Err(_) if cancelled => Status::NotInstalled,
                    Err(error) => {
                        eprintln!("typeaside model download: {error:#}");
                        Status::Failed
                    }
                };
                model.set_status(s, status);
                cx.notify();
            });
            let next = state.update(cx, |s, _| {
                (!s.download_queue.is_empty()).then(|| s.download_queue.remove(0))
            });
            if let Some(next) = next {
                let _ = this.update(cx, |this, cx| this.download_model(next, cx));
            }
        })
        .detach();
    }

    fn cancel_model(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = &self.state.read(cx).model_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    fn remove_model(&mut self, model: ModelId, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let task = cx
            .background_executor()
            .spawn(async move { typeaside::local::remove(model) });
        cx.spawn(async move |_, cx| {
            let result = task.await;
            state.update(cx, |s, cx| {
                match result {
                    Ok(()) => {
                        s.models[model.index()] = Status::NotInstalled;
                        if let Some(remaining) = ModelId::ALL
                            .into_iter()
                            .find(|m| s.models[m.index()] == Status::Installed)
                        {
                            s.config.local_model = remaining;
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

    fn remove_cleanup_model(&mut self, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let task = cx
            .background_executor()
            .spawn(async { CleanupModel::remove() });
        cx.spawn(async move |_, cx| {
            let result = task.await;
            state.update(cx, |s, cx| {
                match result {
                    Ok(()) => s.cleanup_model = Status::NotInstalled,
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
    pub(super) fn jump(&mut self, group: usize, cx: &mut Context<Self>) {
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
        let statuses = s.models;
        let downloading = s.model_cancel.is_some();
        let wispr_connecting = s.wispr_login.is_some();
        let wispr_account = s.wispr_account.clone();
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
        let active = if statuses[c.local_model.index()] == Status::Installed {
            Some(c.local_model)
        } else {
            ModelId::ALL
                .into_iter()
                .find(|id| statuses[id.index()] == Status::Installed)
        };
        let model_rows = ModelId::ALL
            .into_iter()
            .map(|model| {
                let status = statuses[model.index()];
                let is_active = active == Some(model);
                let label = match status {
                    Status::NotInstalled => "Download",
                    Status::Downloading(_) => "Cancel",
                    Status::Installed if is_active => "Remove",
                    Status::Installed => "Use",
                    Status::Failed => "Retry",
                };
                let description = match status {
                    Status::Downloading(percent) => format!(
                        "{}, {} MB, {percent}%",
                        model.description(),
                        model.size_mb()
                    ),
                    Status::Failed => format!(
                        "{}, {} MB. Download failed",
                        model.description(),
                        model.size_mb()
                    ),
                    _ => format!("{}, {} MB", model.description(), model.size_mb()),
                };
                Setting::new(if is_active {
                    format!("● {}", model.label())
                } else {
                    model.label().to_owned()
                })
                .description(
                    div()
                        .when(status == Status::Failed, |text| text.text_color(p.red))
                        .child(description),
                )
                .child(
                    button(label, false, p)
                        .id(model.key())
                        .when(
                            downloading
                                && !matches!(status, Status::Downloading(_) | Status::Installed),
                            |button| button.opacity(0.4),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.state.read(cx).is_demo {
                                return;
                            }
                            match status {
                                Status::Downloading(_) => this.cancel_model(cx),
                                Status::Installed if is_active => this.remove_model(model, cx),
                                Status::Installed => {
                                    this.state.update(cx, |s, cx| {
                                        s.config.local_model = model;
                                        s.persist();
                                        cx.notify();
                                    });
                                }
                                _ if !downloading => {
                                    this.download_model(DownloadTarget::Speech(model), cx)
                                }
                                _ => {}
                            }
                        })),
                )
            })
            .collect::<Vec<_>>();
        // Both stages share these labels, so each control needs its own id scope.
        let source = |id: &'static str,
                      current: Source,
                      set: fn(&mut AppConfig, Source),
                      cx: &mut Context<Self>| {
            div().id(id).child(segmented(
                [(Source::OwnKey, "API key"), (Source::Local, "On device")],
                current,
                p,
                cx,
                move |this, value, _, cx| this.state.update(cx, |s, _| set(&mut s.config, value)),
            ))
        };
        let unload = segmented(
            [
                (LocalUnload::FiveMinutes, "5 min"),
                (LocalUnload::ThirtyMinutes, "30 min"),
                (LocalUnload::Never, "Never"),
            ],
            c.local_unload,
            p,
            cx,
            |this, value, _, cx| this.state.update(cx, |s, _| s.config.local_unload = value),
        );
        let available = typeaside::local::AVAILABLE;
        let local_speech = available && c.transcription == SpeechSource::Local;
        let wispr = c.transcription == SpeechSource::Wispr;
        let local_editing = available && c.local_editing();
        let mut models = Vec::new();
        {
            let mut speech = vec![
                Setting::new("Speech")
                    .when(local_speech && active.is_none(), |row| {
                        row.description(
                            div()
                                .text_color(p.red)
                                .child("Download a model to dictate."),
                        )
                    })
                    .child(
                        div().id("speech-source").child(segmented(
                            [
                                (SpeechSource::OwnKey, "API key"),
                                (SpeechSource::Local, "On device"),
                                (SpeechSource::Wispr, "Wispr Flow"),
                            ]
                            .into_iter()
                            .filter(|(value, _)| available || *value != SpeechSource::Local),
                            c.transcription,
                            p,
                            cx,
                            |this, value, _, cx| {
                                this.state.update(cx, |s, _| s.config.transcription = value)
                            },
                        )),
                    ),
            ];
            if local_speech {
                speech.extend(model_rows);
            } else if wispr {
                let connected = wispr_account.is_some();
                speech.push(
                    Setting::new("Wispr account")
                        .description(wispr_account.unwrap_or_else(|| {
                            if wispr_connecting {
                                "Finish signing in through your browser.".into()
                            } else {
                                "Uses your Wispr Flow account.".into()
                            }
                        }))
                        .child(
                            button(
                                if wispr_connecting {
                                    "Cancel"
                                } else if connected {
                                    "Switch account"
                                } else {
                                    "Sign in"
                                },
                                false,
                                p,
                            )
                            .id("wispr-connect")
                            .on_click(cx.listener(|this, _, _, cx| this.connect_wispr(cx))),
                        )
                        .when(connected, |row| {
                            row.child(
                                button("Disconnect", false, p)
                                    .id("wispr-disconnect")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.disconnect_wispr(cx)),
                                    ),
                            )
                        }),
                );
            } else {
                speech.extend([
                    Setting::new("Endpoint").child(input(&self.settings.endpoint, p)),
                    Setting::new("Model").child(model_field(
                        &self.settings.stt_models,
                        &self.settings.model,
                        p,
                    )),
                    Setting::new("API key").child(input(&self.settings.key, p)),
                ]);
            }
            models.push(panel(speech, p));
        }
        {
            let mut editing = vec![
                Setting::new("AI editing")
                    .description(if wispr {
                        "Wispr formats your dictation."
                    } else if local_editing {
                        "English only. Skips code apps."
                    } else {
                        "Fixes fillers, punctuation and lists."
                    })
                    .when(available && c.cleanup.enabled && !wispr, |row| {
                        row.child(source(
                            "editing-source",
                            c.cleanup.source,
                            |c, v| c.cleanup.source = v,
                            cx,
                        ))
                    })
                    .child(self.toggle("editing", c.cleanup.enabled, Toggle::Editing, cx)),
            ];
            if local_editing {
                let status = self.state.read(cx).cleanup_model;
                let action = match status {
                    Status::Downloading(_) => "Cancel",
                    Status::Installed => "Remove",
                    Status::Failed => "Retry",
                    Status::NotInstalled => "Download",
                };
                let description = match status {
                    Status::Downloading(percent) => {
                        format!("English, {} MB, {percent}%", CleanupModel::size_mb())
                    }
                    Status::Failed => {
                        format!("English, {} MB. Download failed", CleanupModel::size_mb())
                    }
                    Status::NotInstalled => format!(
                        "English, {} MB. Until installed, only fillers are removed",
                        CleanupModel::size_mb()
                    ),
                    Status::Installed => format!("English, {} MB", CleanupModel::size_mb()),
                };
                editing.push(
                    Setting::new(CleanupModel::LABEL)
                        .description(
                            div()
                                .when(status == Status::Failed, |d| d.text_color(p.red))
                                .child(description),
                        )
                        .child(
                            button(action, false, p)
                                .id("s1-mini")
                                .when(
                                    downloading
                                        && !matches!(
                                            status,
                                            Status::Downloading(_) | Status::Installed
                                        ),
                                    |b| b.opacity(0.4),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if this.state.read(cx).is_demo {
                                        return;
                                    }
                                    match status {
                                        Status::Downloading(_) => this.cancel_model(cx),
                                        Status::Installed => this.remove_cleanup_model(cx),
                                        _ if !downloading => {
                                            this.download_model(DownloadTarget::Cleanup, cx)
                                        }
                                        _ => {}
                                    }
                                })),
                        ),
                );
            }
            // Ask anything and Translate always use this endpoint, so it stays reachable
            // when AI editing runs on device or is off.
            let chat = [
                Setting::new("Endpoint").child(input(&self.settings.cleanup_endpoint, p)),
                Setting::new("Model").child(model_field(
                    &self.settings.cleanup_models,
                    &self.settings.cleanup_model,
                    p,
                )),
                Setting::new("API key").child(input(&self.settings.cleanup_key, p)),
            ];
            if c.cleanup.enabled && !local_editing && !wispr {
                editing.extend(chat);
                models.push(panel(editing, p));
            } else {
                models.push(panel(editing, p));
                models.push(panel(
                    std::iter::once(
                        Setting::new("Ask anything and Translate")
                            .description("Run through this endpoint and key."),
                    )
                    .chain(chat),
                    p,
                ));
            }
        }
        if local_speech || local_editing {
            models.push(panel(
                [Setting::new("Unload when idle")
                    .description("Frees on-device model memory after this long unused.")
                    .child(unload)],
                p,
            ));
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
                Setting::new("Pause / resume")
                    .description("Tap while recording.")
                    .child(self.shortcut(Shortcut::Pause, c.pause_hotkey.as_deref(), cx)),
                Setting::new("Paste last dictation").child(self.shortcut(
                    Shortcut::PasteLast,
                    c.paste_last_hotkey.as_deref(),
                    cx,
                )),
                Setting::new("New note").child(self.shortcut(
                    Shortcut::Note,
                    c.note_hotkey.as_deref(),
                    cx,
                )),
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
                Setting::new("Microphones")
                    .description("First available is used.")
                    .child(div().w(px(FIELD)).child(self.microphones(cx))),
                Setting::new("Keep microphone ready")
                    .description("Catches the first word. The OS microphone indicator stays on.")
                    .child(self.toggle(
                        "microphone-ready",
                        c.keep_microphone_ready,
                        Toggle::MicrophoneReady,
                        cx,
                    )),
                Setting::new("Test microphone")
                    .when(testing, |row| row.child(meter(level, 7, 18., p)))
                    .child(
                        button("Test", false, p)
                            .id("test-mic")
                            .on_click(cx.listener(|this, _, _, cx| this.start_mic(cx))),
                    ),
                Setting::new("Sound cues")
                    .description("Short tones when recording starts and stops.")
                    .child(self.toggle("sounds", c.sounds, Toggle::Sounds, cx)),
                Setting::new("Media while dictating")
                    .description("Pause or quiet music and video while you dictate.")
                    .child(segmented(
                        [
                            (MediaControl::Off, "Off"),
                            (MediaControl::LowerVolume, "Lower volume"),
                            (MediaControl::PauseMedia, "Pause media"),
                        ],
                        c.media_control,
                        p,
                        cx,
                        |this, value, _, cx| {
                            this.state.update(cx, |s, _| s.config.media_control = value)
                        },
                    )),
            ],
            p,
        )];
        let behavior = panel(
            [
                Setting::new("Run after dictation").child(input(&self.settings.on_dictation, p)),
                Setting::new("Press Enter after paste")
                    .description("Send chat messages right away.")
                    .child(self.toggle("submit", c.auto_submit, Toggle::Submit, cx)),
                Setting::new("Say send to press Enter")
                    .description("End dictation with send. Say send alone for Enter only.")
                    .child(self.toggle("send-command", c.send_command, Toggle::SendCommand, cx)),
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
                Setting::new("Keep history").child(select(&self.settings.history_retention, p)),
                Setting::new("Version")
                    .description(env!("CARGO_PKG_VERSION"))
                    .child(
                        button("Open config folder", false, p)
                            .id("config-folder")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let folder = this.state.read(cx).paths.config_dir.clone();
                                cx.background_executor()
                                    .spawn(async move {
                                        let _ = typeaside::web::open(folder);
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
