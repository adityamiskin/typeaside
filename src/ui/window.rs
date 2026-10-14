use super::{AppState, settings::SettingsInputs, theme::*};
use gpui::{
    App, AppContext, Bounds, Context, Entity, FontWeight, IntoElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, rgba, size,
};
use gpui_kit::component::{
    input::{InputEvent, InputState},
    select::SelectEvent,
};
use hermes::engine::Phase;

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Home,
    Dictionary,
    Settings,
}

pub struct MainWindow {
    pub state: Entity<AppState>,
    pub settings: SettingsInputs,
    pub word_input: Entity<InputState>,
    pub copied: bool,
    pub copy_generation: u64,
    page: Page,
    dirty: bool,
}

impl MainWindow {
    fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let preferences = state.read(cx).preferences.clone();
        let settings = SettingsInputs::new(&preferences, window, cx);
        for input in settings.text_inputs() {
            cx.subscribe_in(input, window, |this, _, event, window, cx| match event {
                InputEvent::Change => this.dirty = true,
                InputEvent::Blur | InputEvent::PressEnter { .. } => this.save_settings(window, cx),
                InputEvent::Focus => {}
            })
            .detach();
        }
        cx.subscribe(&settings.device, |this, _, event, cx| {
            let SelectEvent::Confirm(value) = event;
            if let Some(value) = value {
                this.state.update(cx, |state, cx| {
                    state.preferences.device.clone_from(value);
                    state.persist("");
                    cx.notify();
                });
            }
        })
        .detach();
        let word_input = cx.new(|cx| InputState::new(window, cx).placeholder("Add a word or name"));
        cx.subscribe_in(&word_input, window, |this, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.add_word(window, cx);
            }
        })
        .detach();
        let entity = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let _ = entity.update(cx, |this, cx| this.save_settings(window, cx));
            true
        });
        let page = state.read(cx).initial_page;
        Self {
            state,
            settings,
            word_input,
            copied: false,
            copy_generation: 0,
            page,
            dirty: false,
        }
    }

    fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dirty {
            return;
        }
        let inputs = &self.settings;
        let key = inputs.key.read(cx).value().to_string();
        let hotkey = inputs.hotkey.read(cx).value().trim().to_owned();
        let endpoint = inputs.endpoint.read(cx).value().trim().to_owned();
        let model = inputs.model.read(cx).value().trim().to_owned();
        let language = inputs.language.read(cx).value().trim().to_owned();
        let cleanup_endpoint = inputs.cleanup_endpoint.read(cx).value().trim().to_owned();
        let cleanup_model = inputs.cleanup_model.read(cx).value().trim().to_owned();
        self.state.update(cx, |state, cx| {
            let preferences = &mut state.preferences;
            preferences.hotkey = hotkey;
            preferences.endpoint = endpoint;
            preferences.model = model;
            preferences.language = language;
            preferences.cleanup_endpoint = cleanup_endpoint;
            preferences.cleanup_model = cleanup_model;
            state.persist(&key);
            cx.notify();
        });
        if !key.is_empty() && self.state.read(cx).save_error.is_none() {
            let hint = super::binding::mask_key(&key);
            self.state
                .update(cx, |state, _| state.preferences.key_hint.clone_from(&hint));
            self.settings.key.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.set_placeholder(hint, window, cx);
            });
        }
        self.dirty = self.state.read(cx).save_error.is_some();
    }

    pub fn set_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.save_settings(window, cx);
        self.page = page;
        cx.notify();
    }
}

impl Render for MainWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let (color, status) = if let Some(error) = &state.save_error {
            (RED, error.clone())
        } else {
            match &state.phase {
                Phase::Idle => (GREEN, "Ready".into()),
                Phase::Listening => (WHITE, "Listening".into()),
                Phase::Transcribing => (WHITE, "Transcribing".into()),
                Phase::Cleaning => (WHITE, "Cleaning".into()),
                Phase::Error(error) => (RED, error.clone()),
            }
        };
        let side = div()
            .w(px(SIDEBAR))
            .flex_shrink_0()
            .h_full()
            .border_r_1()
            .border_color(rgb(DIVIDER))
            .px(px(10.))
            .py(px(18.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .px(px(10.))
                    .pb(px(18.))
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("hermes"),
            )
            .children(
                [
                    (Page::Home, "Home"),
                    (Page::Dictionary, "Dictionary"),
                    (Page::Settings, "Settings"),
                ]
                .into_iter()
                .map(|(page, title)| {
                    div()
                        .id(title)
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(RADIUS))
                        .cursor_pointer()
                        .bg(rgb(if self.page == page { ROW } else { BLACK }))
                        .text_color(if self.page == page {
                            rgb(WHITE)
                        } else {
                            rgba(MUTED)
                        })
                        .hover(|style| style.bg(rgb(ROW)).text_color(rgb(WHITE)))
                        .child(title)
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.set_page(page, window, cx)),
                        )
                }),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(10.))
                    .text_size(px(12.))
                    .text_color(rgba(MUTED))
                    .child(
                        div()
                            .size(px(6.))
                            .flex_shrink_0()
                            .rounded_full()
                            .bg(rgb(color)),
                    )
                    .child(div().min_w_0().text_ellipsis().child(status)),
            );
        let content = match self.page {
            Page::Home => self.home(cx).into_any_element(),
            Page::Dictionary => self.dictionary(cx).into_any_element(),
            Page::Settings => self.settings(cx).into_any_element(),
        };
        div()
            .size_full()
            .bg(rgb(BLACK))
            .text_color(rgb(WHITE))
            .font_family("Inter")
            .text_size(px(13.))
            .line_height(px(19.))
            .flex()
            .child(side)
            .child(div().flex_1().min_w_0().h_full().child(content))
    }
}

pub fn open(state: Entity<AppState>, cx: &mut App) -> anyhow::Result<gpui::AnyWindowHandle> {
    let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
    let (handle, _) = gpui_kit::open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(760.), px(480.))),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("hermes".into()),
                appears_transparent: true,
                ..Default::default()
            }),
            app_id: Some("hermes".into()),
            ..Default::default()
        },
        cx,
        |window, cx| cx.new(|cx| MainWindow::new(state, window, cx)),
    )?;
    Ok(handle)
}
