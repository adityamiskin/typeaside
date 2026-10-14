use super::{theme::*, window::MainWindow};
use gpui::{
    AppContext, Context, Entity, IntoElement, Styled, Window, div, prelude::*, px, rgb, rgba,
};
use gpui_kit::component::{
    IndexPath, Sizable,
    input::{Input, InputState},
    select::{Select, SelectState},
};
use hermes::config::{AppConfig, RecordingMode};

pub struct SettingsInputs {
    pub hotkey: Entity<InputState>,
    pub endpoint: Entity<InputState>,
    pub model: Entity<InputState>,
    pub key: Entity<InputState>,
    pub language: Entity<InputState>,
    pub cleanup_endpoint: Entity<InputState>,
    pub cleanup_model: Entity<InputState>,
    pub device: Entity<SelectState<Vec<String>>>,
}

impl SettingsInputs {
    pub fn new(
        config: &AppConfig,
        key_hint: &str,
        window: &mut Window,
        cx: &mut Context<MainWindow>,
    ) -> Self {
        let mut input = |value: &str, placeholder: &str, masked| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value.to_owned())
                    .placeholder(placeholder.to_owned())
                    .masked(masked)
            })
        };
        let hotkey = input(&config.hotkey, "RIGHTCTRL", false);
        let endpoint = input(
            &config.stt.base_url,
            "https://api.groq.com/openai/v1",
            false,
        );
        let model = input(&config.stt.model, "whisper-large-v3-turbo", false);
        let key = input("", key_hint, true);
        let language = input(
            config.language.as_deref().unwrap_or_default(),
            "Auto detect",
            false,
        );
        let cleanup_endpoint = input(
            &config.cleanup.endpoint.base_url,
            "https://api.groq.com/openai/v1",
            false,
        );
        let cleanup_model = input(&config.cleanup.endpoint.model, "openai/gpt-oss-20b", false);
        let mut devices = vec!["System default".to_owned()];
        match hermes::audio::list_input_devices() {
            Ok(inputs) => devices.extend(inputs.into_iter().map(|input| input.name)),
            Err(error) => eprintln!("hermes: cannot enumerate microphones: {error}"),
        }
        devices.dedup();
        let index = devices
            .iter()
            .position(|device| {
                device.as_str() == config.input_device.as_deref().unwrap_or("System default")
            })
            .unwrap_or(0);
        let device = cx
            .new(|cx| SelectState::new(devices, Some(IndexPath::default().row(index)), window, cx));
        Self {
            hotkey,
            endpoint,
            model,
            key,
            language,
            cleanup_endpoint,
            cleanup_model,
            device,
        }
    }

    pub fn text_inputs(&self) -> [&Entity<InputState>; 6] {
        [
            &self.hotkey,
            &self.endpoint,
            &self.model,
            &self.language,
            &self.cleanup_endpoint,
            &self.cleanup_model,
        ]
    }
}

pub fn input(state: &Entity<InputState>, width: f32) -> impl IntoElement {
    Input::new(state)
        .min_h(px(CONTROL))
        .max_h(px(CONTROL))
        .w(px(width))
        .max_w_full()
        .rounded(px(RADIUS))
        .text_size(px(12.))
        .font_family(MONO)
        .bg(rgb(BLACK))
}

impl MainWindow {
    pub fn settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let prefs = &self.state.read(cx).config;
        let mode_value = prefs.mode;
        let cleanup = prefs.cleanup.enabled;
        let auto_submit = prefs.auto_submit;
        let mode = div()
            .flex()
            .border_1()
            .border_color(rgb(BORDER))
            .rounded(px(RADIUS))
            .overflow_hidden()
            .children(
                [
                    (RecordingMode::Hold, "Hold"),
                    (RecordingMode::Toggle, "Toggle"),
                ]
                .into_iter()
                .map(|(value, label)| {
                    div()
                        .id(label)
                        .px(px(12.))
                        .py(px(5.))
                        .cursor_pointer()
                        .bg(rgb(if value == mode_value { WHITE } else { BLACK }))
                        .text_color(if value == mode_value {
                            rgb(BLACK)
                        } else {
                            rgba(MUTED)
                        })
                        .child(label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.state.update(cx, |state, cx| {
                                state.config.mode = value;
                                state.persist("");
                                cx.notify();
                            });
                        }))
                }),
            );
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(header("Settings"))
            .child(
                div()
                    .id("settings-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(INSET))
                    .pt(px(8.))
                    .pb(px(24.))
                    .child(section("Shortcut"))
                    .child(form_row(
                        "Dictation key",
                        input(&self.settings.hotkey, 180.),
                    ))
                    .child(form_row("Mode", div().flex().child(mode)))
                    .child(section("Microphone"))
                    .child(form_row(
                        "Input device",
                        Select::new(&self.settings.device)
                            .small()
                            .bg(rgb(BLACK))
                            .min_h(px(CONTROL))
                            .max_h(px(CONTROL))
                            .w_full()
                            .max_w(px(420.)),
                    ))
                    .child(section("Speech to text"))
                    .child(form_row("Endpoint", input(&self.settings.endpoint, 420.)))
                    .child(form_row("Model", input(&self.settings.model, 260.)))
                    .child(form_row(
                        "API key",
                        div()
                            .flex()
                            .items_center()
                            .flex_wrap()
                            .gap(px(8.))
                            .child(input(&self.settings.key, 260.))
                            .when(self.state.read(cx).key_hint != "Enter API key", |row| {
                                row.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgba(MUTED))
                                        .child("Stored in keyring"),
                                )
                            }),
                    ))
                    .child(form_row("Language", input(&self.settings.language, 180.)))
                    .child(section("Cleanup"))
                    .child(form_row(
                        "Remove fillers and fix punctuation",
                        switch("cleanup", cleanup).on_click(cx.listener(|this, _, _, cx| {
                            this.state.update(cx, |state, cx| {
                                state.config.cleanup.enabled = !state.config.cleanup.enabled;
                                state.persist("");
                                cx.notify();
                            });
                        })),
                    ))
                    .child(form_row(
                        "Endpoint",
                        input(&self.settings.cleanup_endpoint, 420.),
                    ))
                    .child(form_row("Model", input(&self.settings.cleanup_model, 260.)))
                    .child(section("Behavior"))
                    .child(form_row(
                        "Press Enter after pasting",
                        switch("auto-submit", auto_submit).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.state.update(cx, |state, cx| {
                                    state.config.auto_submit = !state.config.auto_submit;
                                    state.persist("");
                                    cx.notify();
                                });
                            },
                        )),
                    )),
            )
    }
}

fn switch(id: &'static str, enabled: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w(px(32.))
        .h(px(18.))
        .rounded(px(9.))
        .relative()
        .cursor_pointer()
        .bg(rgb(if enabled { WHITE } else { BORDER }))
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(px(if enabled { 16. } else { 2. }))
                .size(px(14.))
                .rounded_full()
                .bg(rgb(if enabled { BLACK } else { 0x777777 })),
        )
}
