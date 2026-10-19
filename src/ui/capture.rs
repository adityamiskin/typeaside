use super::{AppState, theme::*};
#[cfg(target_os = "linux")]
use gpui::layer_shell::*;
use gpui::{
    App, AppContext, Bounds, BoxShadow, Context, Entity, IntoElement, Render, Styled, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, point, prelude::*,
    px, size,
};
use gpui_kit::component::{
    Icon,
    input::{Enter, Escape, TextareaState},
};
use std::time::{Duration, Instant};
use typeaside::{engine::Command, notes::Op};

pub struct Capture {
    state: Entity<AppState>,
    input: Entity<TextareaState>,
    saved: bool,
    entered: Instant,
    visible: bool,
    ready: bool,
}
impl Capture {
    pub fn insert(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.visible && !self.saved {
            self.input
                .update(cx, |input, cx| input.insert(text, window, cx));
            cx.notify();
        }
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.visible {
            return;
        }
        self.visible = false;
        self.ready = false;
        let draft = if self.saved {
            String::new()
        } else {
            self.input.read(cx).value().to_string()
        };
        self.state.update(cx, |s, cx| {
            s.note_draft = draft;
            if self.saved {
                s.note_source = None;
            }
            #[cfg(not(target_os = "linux"))]
            {
                s.capture_window = None;
            }
            s.command(Command::CloseCapture);
            cx.notify();
        });
        #[cfg(target_os = "linux")]
        {
            window.set_input_region(Some(&[]));
            window.set_layer_shell_keyboard_interactivity(KeyboardInteractivity::None);
            window.blur(cx);
            cx.notify();
        }
        #[cfg(not(target_os = "linux"))]
        window.remove_window();
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saved {
            return;
        }
        let text = self.input.read(cx).value().trim().to_owned();
        if text.is_empty() {
            self.close(window, cx);
            return;
        }
        if text.chars().count() > 20_000 {
            return;
        }
        let note = match typeaside::notes::new_note(text) {
            Ok(note) => note,
            Err(error) => {
                self.state.update(cx, |s, cx| {
                    s.save_error = Some(error.to_string());
                    cx.notify();
                });
                return;
            }
        };
        if !self.state.update(cx, |s, cx| {
            let saved = s.change_note(Op::Put(note));
            cx.notify();
            saved
        }) {
            return;
        }
        self.saved = true;
        cx.notify();
        let entered = self.entered;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(350))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.entered == entered {
                    this.close(window, cx);
                }
            });
        })
        .detach();
    }
}
impl Render for Capture {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        if !self.visible {
            return div().size_full().into_any_element();
        }
        let value = self.input.read(cx).value();
        let run = gpui::TextRun {
            len: value.len(),
            font: gpui::font("Inter"),
            color: p.fg,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let lines = gpui::WindowTextSystem::new(cx.text_system().clone())
            .shape_text(value.clone(), px(15.), &[run], Some(px(566.)), None)
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| line.wrap_boundaries.len() + 1)
                    .sum::<usize>()
            })
            .unwrap_or(1)
            .clamp(4, 14);
        let height = 80. + lines as f32 * 22.;
        #[cfg(target_os = "linux")]
        if self.ready {
            window.set_input_region(Some(&[Bounds::new(
                point(px(40.), px(24.)),
                size(px(600.), px(height)),
            )]));
        }
        let s = self.state.read(cx);
        let footer = if self.saved {
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .text_color(p.green)
                .child(Icon::default().path("icons/check.svg").size(px(13.)))
                .child("Saved to Notes")
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(if s.config.mode == typeaside::config::RecordingMode::Hold {
                            "Hold"
                        } else {
                            "Press"
                        })
                        .child(kbd(&s.config.hotkey, p))
                        .child("to dictate"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(kbd("ENTER", p))
                        .child("Save"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(kbd("ESC", p))
                        .child("Close"),
                )
        };
        div()
            .size_full()
            .font_family("Inter")
            .text_color(p.fg)
            .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                if action.shift {
                    cx.propagate();
                } else {
                    this.save(window, cx);
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                this.close(window, cx);
                cx.stop_propagation();
            }))
            .child(
                div()
                    .absolute()
                    .left(px(40.))
                    .top(px(24.))
                    .w(px(600.))
                    .px(px(16.))
                    .py(px(12.))
                    .rounded(px(14.))
                    .border_1()
                    .border_color(p.border)
                    .bg(p.panel)
                    .shadow(vec![
                        BoxShadow {
                            color: p.shadow,
                            offset: point(px(0.), px(24.)),
                            blur_radius: px(64.),
                            spread_radius: px(0.),
                            inset: false,
                        },
                        BoxShadow {
                            color: p.shadow,
                            offset: point(px(0.), px(2.)),
                            blur_radius: px(6.),
                            spread_radius: px(0.),
                            inset: false,
                        },
                    ])
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .h(px(18.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(Icon::default().path("icons/notebook-pen.svg").size(px(13.)))
                            .child("New note")
                            .children(s.note_source.as_ref().map(|source| {
                                div()
                                    .ml_auto()
                                    .text_color(p.faint)
                                    .child(format!("From {source}"))
                            })),
                    )
                    .child(
                        div()
                            .pt(px(8.))
                            .pb(px(10.))
                            .opacity(if self.saved { 0.35 } else { 1. })
                            .child(
                                div()
                                    .h(px(lines as f32 * 22.))
                                    .text_size(px(15.))
                                    .line_height(px(22.))
                                    .child(if self.saved {
                                        div().child(value.to_string()).into_any_element()
                                    } else {
                                        self.input.clone().into_any_element()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .h(px(20.))
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(footer),
                    ),
            )
            .into_any_element()
    }
}

pub fn toggle(state: &Entity<AppState>, selection: String, app: Option<String>, cx: &mut App) {
    #[cfg(target_os = "linux")]
    {
        if state.read(cx).is_demo {
            show(state, cx);
        }
        if let Some(handle) = state.read(cx).capture_window {
            let _ = handle.update(cx, |view, window, cx| {
                if !view.visible {
                    return;
                }
                prefill(state, selection, app, cx);
                view.input.update(cx, |input, cx| {
                    input.set_value(state.read(cx).note_draft.clone(), window, cx);
                    input.focus(window, cx);
                    input.set_cursor_position(
                        gpui_kit::component::input::Position::new(u32::MAX, u32::MAX),
                        window,
                        cx,
                    );
                });
                view.ready = true;
                window.set_layer_shell_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                cx.notify();
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        if let Some(handle) = state.read(cx).capture_window {
            if handle
                .update(cx, |view, window, cx| view.close(window, cx))
                .is_ok()
            {
                return;
            }
            state.update(cx, |s, _| s.capture_window = None);
        }
        prefill(state, selection, app, cx);
        open(state, true, cx);
    }
}

fn prefill(state: &Entity<AppState>, selection: String, app: Option<String>, cx: &mut App) {
    state.update(cx, |s, _| {
        if s.note_draft.is_empty() {
            s.note_source = None;
        }
        if s.note_draft.is_empty() && !selection.trim().is_empty() {
            s.note_draft = format!("\"{}\"\n", selection.trim());
            s.note_source = app;
        }
        if s.is_demo
            && let Ok(text) = std::env::var("TYPEASIDE_DEMO_CAPTURE_TEXT")
        {
            s.note_draft.push_str(&text);
        }
    });
}

#[cfg(target_os = "linux")]
pub fn prepare(state: &Entity<AppState>, cx: &mut App) {
    if state.read(cx).capture_window.is_none() {
        open(state, false, cx);
    }
}

#[cfg(target_os = "linux")]
pub fn show(state: &Entity<AppState>, cx: &mut App) {
    prepare(state, cx);
    if let Some(handle) = state.read(cx).capture_window {
        let _ = handle.update(cx, |view, window, cx| {
            view.visible = true;
            view.ready = false;
            view.saved = false;
            view.entered = Instant::now();
            view.input.update(cx, |input, cx| {
                input.set_value(state.read(cx).note_draft.clone(), window, cx);
            });
            cx.notify();
        });
    }
}

fn open(state: &Entity<AppState>, visible: bool, cx: &mut App) {
    let display = cx.primary_display();
    let bounds = display
        .as_ref()
        .map(|display| display.bounds())
        .unwrap_or(Bounds::new(
            point(px(0.), px(0.)),
            size(px(1280.), px(720.)),
        ));
    let top = bounds.size.height * 0.22 - px(24.);
    let state = state.clone();
    let result = cx.open_window(
        WindowOptions {
            titlebar: None,
            focus: visible,
            inactive_frame_interval: None,
            app_id: Some("typeaside-capture".into()),
            window_background: WindowBackgroundAppearance::Transparent,
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(
                    bounds.origin.x + (bounds.size.width - px(680.)) / 2.,
                    bounds.origin.y + top,
                ),
                size(px(680.), px(460.)),
            ))),
            #[cfg(target_os = "linux")]
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "typeaside-capture".into(),
                layer: Layer::Overlay,
                anchor: Anchor::TOP,
                keyboard_interactivity: KeyboardInteractivity::None,
                margin: Some((top, px(0.), px(0.), px(0.))),
                ..Default::default()
            }),
            #[cfg(not(target_os = "linux"))]
            kind: WindowKind::PopUp,
            #[cfg(not(target_os = "linux"))]
            is_movable: false,
            #[cfg(not(target_os = "linux"))]
            is_resizable: false,
            ..Default::default()
        },
        |window, cx| {
            cx.new(|cx| {
                let input = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .default_value(state.read(cx).note_draft.clone())
                        .placeholder("Write a note…")
                        .auto_grow(4, 14)
                });
                if visible {
                    input.update(cx, |input, cx| {
                        input.focus(window, cx);
                        input.set_cursor_position(
                            gpui_kit::component::input::Position::new(u32::MAX, u32::MAX),
                            window,
                            cx,
                        );
                    });
                }
                #[cfg(target_os = "linux")]
                window.set_input_region(Some(&[]));
                cx.subscribe(&input, |_: &mut Capture, _, event, cx| {
                    if matches!(event, gpui_kit::component::input::InputEvent::Change) {
                        cx.notify();
                    }
                })
                .detach();
                super::theme::apply(state.read(cx).config.theme, window.appearance(), cx);
                cx.observe_window_appearance(window, |this: &mut Capture, window, cx| {
                    super::theme::apply(this.state.read(cx).config.theme, window.appearance(), cx);
                    cx.notify();
                })
                .detach();
                let now = Instant::now();
                Capture {
                    state: state.clone(),
                    input,
                    saved: false,
                    entered: now,
                    visible,
                    ready: visible,
                }
            })
        },
    );
    match result {
        Ok(handle) => {
            #[cfg(target_os = "macos")]
            let _ = handle.update(cx, |_, window, _| super::overlay::borderless(window));
            state.update(cx, |s, _| s.capture_window = Some(handle));
            if state.read(cx).is_demo
                && std::env::var("TYPEASIDE_DEMO_CAPTURE_SAVED").as_deref() == Ok("1")
            {
                let _ = handle.update(cx, |view, _, cx| {
                    view.saved = true;
                    cx.notify();
                });
            }
        }
        Err(error) => state.update(cx, |s, cx| {
            s.command(Command::CloseCapture);
            s.save_error = Some(format!("Could not open note: {error}"));
            cx.notify();
        }),
    }
}
