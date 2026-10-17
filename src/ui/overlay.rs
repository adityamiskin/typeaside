use super::{AppState, motion::Spring, theme::*};
#[cfg(target_os = "linux")]
use gpui::layer_shell::*;
use gpui::{
    App, AppContext, Bounds, BoxShadow, Context, Entity, PathBuilder, Render, TextRun, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, canvas, div,
    fill, font, point, prelude::*, px, size,
};
use std::time::{Duration, Instant};
use typeaside::engine::Phase;

#[derive(Clone, PartialEq)]
enum Visual {
    Listening,
    Processing,
    Success,
    Cancel,
    Error(String),
    Copied(String),
    Exit,
}

pub struct Overlay {
    state: Entity<AppState>,
    command: bool,
    traced: Option<&'static str>,
    phase: Visual,
    phase_at: Instant,
    entered: Instant,
    last_frame: Instant,
    width: Spring,
    rise: Spring,
    scale: Spring,
    dot: Spring,
    bars: [Spring; BARS],
    positions: [Spring; BARS],
    levels: [f32; BARS],
    quiet: bool,
}

impl Overlay {
    fn new(state: Entity<AppState>) -> Self {
        let now = Instant::now();
        Self {
            state,
            command: false,
            traced: None,
            phase: Visual::Listening,
            phase_at: now,
            entered: now,
            last_frame: now,
            width: Spring::new(PILL_HEIGHT),
            rise: Spring::new(8.),
            scale: Spring::new(0.9),
            dot: Spring::new(0.),
            bars: [Spring::new(0.); BARS],
            positions: std::array::from_fn(|i| Spring::new((i as f32 - BAR_CENTER) * 5.)),
            levels: [0.15; BARS],
            quiet: false,
        }
    }

    pub fn level(&mut self, level: f32, cx: &mut Context<Self>) {
        self.levels.rotate_left(1);
        self.levels[BARS - 1] = if level.is_finite() {
            level.clamp(0., 1.)
        } else {
            0.
        };
        self.quiet = level < 0.05;
        cx.notify();
    }

    pub fn phase(&mut self, phase: Phase, window: &mut Window, cx: &mut Context<Self>) {
        let next = match phase {
            Phase::Listening(mode) => {
                self.command = mode == typeaside::engine::Mode::Command;
                cx.notify();
                Visual::Listening
            }
            Phase::Transcribing | Phase::Cleaning => Visual::Processing,
            Phase::Error(message) => Visual::Error(message),
            Phase::Idle if self.phase == Visual::Listening => Visual::Cancel,
            Phase::Idle if self.phase == Visual::Processing => Visual::Exit,
            Phase::Idle => return,
        };
        self.set(next, window, cx);
    }

    pub fn success(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set(Visual::Success, window, cx);
    }

    pub fn copied(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let first_line = text.lines().next().unwrap_or("");
        let preview = first_line.split_whitespace().collect::<Vec<_>>().join(" ");
        let measure = |text: String| {
            let len = text.len();
            f32::from(
                window
                    .text_system()
                    .shape_line(
                        text.into(),
                        px(13.),
                        &[TextRun {
                            len,
                            font: font("Inter"),
                            color: palette(cx).fg,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }],
                        None,
                    )
                    .width,
            )
        };
        let available = SURFACE_WIDTH - 48. - 46. - 6. - measure("Copied".into());
        let mut quoted = format!("“{preview}”");
        if measure(quoted.clone()) > available {
            let boundaries = preview
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(preview.len()))
                .collect::<Vec<_>>();
            let (mut low, mut high) = (0, boundaries.len() - 1);
            while low < high {
                let mid = (low + high).div_ceil(2);
                if measure(format!("“{}…”", &preview[..boundaries[mid]])) <= available {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            quoted = format!("“{}…”", &preview[..boundaries[low]]);
        }
        self.set(Visual::Copied(quoted), window, cx);
    }

    fn set(&mut self, phase: Visual, window: &mut Window, cx: &mut Context<Self>) {
        if self.phase == phase {
            return;
        }
        self.phase = phase;
        self.phase_at = Instant::now();
        if let Visual::Error(message) | Visual::Copied(message) = &self.phase {
            let line = window.text_system().shape_line(
                if matches!(self.phase, Visual::Copied(_)) {
                    format!("Copied{message}").into()
                } else {
                    message.clone().into()
                },
                px(13.),
                &[TextRun {
                    len: message.len()
                        + if matches!(self.phase, Visual::Copied(_)) {
                            6
                        } else {
                            0
                        },
                    font: font("Inter"),
                    color: palette(cx).fg,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            );
            self.width.target = (f32::from(line.width)
                + 46.
                + if matches!(self.phase, Visual::Copied(_)) {
                    6.
                } else {
                    0.
                })
            .clamp(96., SURFACE_WIDTH - 48.);
            self.dot = Spring::new(0.);
            self.dot.target = 1.;
        }
        let hold = match self.phase {
            Visual::Success => Some(CHECK_TIME + SUCCESS_HOLD),
            Visual::Cancel => Some(CANCEL_TIME),
            Visual::Error(_) => Some(ERROR_HOLD),
            Visual::Copied(_) => Some(COPIED_HOLD),
            _ => None,
        };
        if let Some(hold) = hold {
            let phase_at = self.phase_at;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_secs_f32(hold))
                    .await;
                let _ = this.update(cx, |this, cx| {
                    if this.phase_at == phase_at {
                        this.phase = Visual::Exit;
                        this.phase_at = Instant::now();
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    fn advance(&mut self, window: &mut Window) -> (f32, f32, f32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.032);
        self.last_frame = now;
        let entered = now.duration_since(self.entered).as_secs_f32();
        let elapsed = now.duration_since(self.phase_at).as_secs_f32();
        if self.phase == Visual::Exit && elapsed >= EXIT_TIME {
            window.remove_window();
        }
        let exit = if self.phase == Visual::Exit {
            (elapsed / EXIT_TIME).clamp(0., 1.).powi(2)
        } else {
            0.
        };
        let opacity = (entered / ENTER_FADE).min(1.) * (1. - exit);
        self.rise.target = 0.;
        self.scale.target = if self.phase == Visual::Exit { 0.92 } else { 1. };
        match self.phase {
            Visual::Listening => {
                self.width.target = if entered > OPEN_DELAY {
                    if self.command {
                        COMMAND_WIDTH
                    } else {
                        LISTEN_WIDTH
                    }
                } else {
                    PILL_HEIGHT
                }
            }
            Visual::Processing => self.width.target = WORK_WIDTH,
            Visual::Success => self.width.target = PILL_HEIGHT,
            Visual::Cancel | Visual::Exit => {}
            Visual::Error(_) | Visual::Copied(_) => {}
        }
        let mut moving = self.width.step(dt, PILL_STIFFNESS, PILL_DAMPING);
        moving |= self.rise.step(dt, PILL_STIFFNESS, PILL_DAMPING);
        moving |= self.scale.step(dt, PILL_STIFFNESS, PILL_DAMPING);
        moving |= self.dot.step(dt, PILL_STIFFNESS, PILL_DAMPING);
        for i in 0..BARS {
            let distance = (i as f32 - BAR_CENTER).abs();
            self.bars[i].target = if self.phase == Visual::Listening
                && entered >= OPEN_DELAY + distance * STAGGER
            {
                if self.quiet {
                    3. + (entered * std::f32::consts::TAU / QUIET_PERIOD - i as f32 * 0.45).sin()
                } else {
                    (22. * self.levels[i]
                        * (-((i as f32 - BAR_CENTER) / BAR_ENVELOPE).powi(2)).exp())
                    .max(3.)
                }
            } else if matches!(self.phase, Visual::Processing | Visual::Cancel) {
                3.
            } else {
                0.
            };
            self.positions[i].target = if self.phase == Visual::Listening {
                (i as f32 - BAR_CENTER) * 5.
            } else {
                (i.clamp(PROCESS_START, PROCESS_END) as f32 - BAR_CENTER) * 5.
            };
            moving |= self.bars[i].step(dt, BAR_STIFFNESS, BAR_DAMPING);
            moving |= self.positions[i].step(dt, PILL_STIFFNESS, PILL_DAMPING);
        }
        let active = matches!(self.phase, Visual::Listening | Visual::Processing)
            || self.phase == Visual::Success && elapsed < CHECK_TIME
            || self.phase == Visual::Cancel && elapsed < CANCEL_TIME
            || matches!(self.phase, Visual::Error(_)) && elapsed < SHAKE_TIME;
        if (moving || active || self.phase == Visual::Exit)
            && !(self.phase == Visual::Exit && elapsed >= EXIT_TIME)
        {
            window.request_animation_frame();
        }
        (opacity, elapsed, entered)
    }
}

impl Render for Overlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let command = self.command && self.phase == Visual::Listening;
        let (opacity, elapsed, _) = self.advance(window);
        let kind = match &self.phase {
            Visual::Listening if command => "command",
            Visual::Listening => "listening",
            Visual::Processing => "processing",
            Visual::Success => "success",
            Visual::Error(_) => "error",
            Visual::Copied(_) => "copied",
            Visual::Cancel => "cancel",
            Visual::Exit => "exit",
        };
        let trace = if self.traced != Some(kind)
            && std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1")
        {
            self.traced = Some(kind);
            Some(
                canvas(
                    |_, _, _| (),
                    move |_, _, _, _| eprintln!("typeaside overlay frame: {kind}"),
                )
                .absolute()
                .size(px(1.)),
            )
        } else {
            None
        };

        let scale = self.scale.value;
        let width = self.width.value * scale;
        let height = PILL_HEIGHT * scale;
        let shake = if matches!(self.phase, Visual::Error(_)) && elapsed < SHAKE_TIME {
            (elapsed / SHAKE_TIME * std::f32::consts::TAU * 3.).sin()
                * 5.
                * (1. - elapsed / SHAKE_TIME).powi(2)
        } else {
            0.
        };
        let mut pill = div()
            .absolute()
            .left(px((SURFACE_WIDTH - width) / 2. + shake))
            .top(px(SURFACE_HEIGHT - SHADOW_SPACE - height + self.rise.value))
            .w(px(width))
            .h(px(height))
            .rounded(px(height / 2.))
            .bg(p.pill)
            .border_1()
            .border_color(p.pill_border)
            .shadow(vec![BoxShadow {
                color: p.shadow,
                offset: point(px(0.), px(10.)),
                blur_radius: px(28.),
                spread_radius: px(0.),
                inset: false,
            }])
            .opacity(opacity)
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden();
        if let Visual::Error(message) | Visual::Copied(message) = &self.phase {
            let text_progress = (elapsed / ERROR_TEXT_TIME).min(1.);
            pill = pill.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(14.))
                    .child(
                        div()
                            .size(px(6. * self.dot.value.max(0.)))
                            .flex_shrink_0()
                            .rounded_full()
                            .bg(if matches!(self.phase, Visual::Copied(_)) {
                                p.blue
                            } else {
                                p.red
                            }),
                    )
                    .child(
                        div()
                            .relative()
                            .left(px(6. * (1. - text_progress)))
                            .opacity(text_progress)
                            .text_color(p.fg)
                            .text_size(px(13.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .whitespace_nowrap()
                            .when(matches!(self.phase, Visual::Copied(_)), |el| {
                                el.child("Copied")
                            })
                            .child(
                                div()
                                    .text_color(if matches!(self.phase, Visual::Copied(_)) {
                                        p.muted
                                    } else {
                                        p.fg
                                    })
                                    .child(message.clone()),
                            ),
                    ),
            );
        } else {
            if command {
                pill = pill.child(
                    div()
                        .absolute()
                        .left(px(width / 2. - 40.5))
                        .w(px(12.))
                        .text_size(px(12.))
                        .line_height(px(12.))
                        .text_color(p.lilac)
                        .child("✦"),
                );
            }
            let phase = self.phase.clone();
            let bars = self.bars;
            let positions = self.positions;
            pill = pill.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let center = bounds.center();
                        for i in 0..BARS {
                            let retained = (PROCESS_START..=PROCESS_END).contains(&i);
                            let processing = phase == Visual::Processing;
                            let fade = if processing && !retained {
                                (1. - elapsed / 0.15).max(0.)
                            } else if phase == Visual::Success {
                                (1. - elapsed / 0.1).max(0.)
                            } else {
                                1.
                            };
                            let wave = ((elapsed - (i as f32 - PROCESS_START as f32) * WAVE_STEP)
                                * std::f32::consts::TAU
                                / WAVE_PERIOD)
                                .sin()
                                * 0.5
                                + 0.5;
                            let alpha = if processing {
                                (0.35 + 0.65 * wave) * fade
                            } else {
                                fade
                            };
                            let y = if processing && retained {
                                -2. * wave
                            } else {
                                0.
                            };
                            let h = bars[i].value.max(0.) * scale;
                            let x = positions[i].value;
                            let rect = Bounds::new(
                                point(
                                    center.x
                                        + px((x + if command { 9. } else { 0. }) * scale - 1.5),
                                    center.y + px(y - h / 2.),
                                ),
                                size(px(3. * scale), px(h)),
                            );
                            window.paint_quad(
                                fill(
                                    rect,
                                    if processing {
                                        p.blue.opacity(alpha)
                                    } else {
                                        p.fg.opacity(alpha)
                                    },
                                )
                                .corner_radii(px(1.5)),
                            );
                        }
                        if phase == Visual::Success {
                            let progress = (elapsed / CHECK_TIME).min(1.);
                            let points = [
                                point(center.x - px(6.), center.y),
                                point(center.x - px(2.), center.y + px(4.)),
                                point(center.x + px(6.), center.y - px(5.)),
                            ];
                            let mut path = PathBuilder::stroke(px(2.));
                            path.move_to(points[0]);
                            let split = 0.32;
                            if progress < split {
                                path.line_to(
                                    points[0] + (points[1] - points[0]) * (progress / split),
                                );
                            } else {
                                path.line_to(points[1]);
                                path.line_to(
                                    points[1]
                                        + (points[2] - points[1])
                                            * ((progress - split) / (1. - split)),
                                );
                            }
                            if let Ok(path) = path.build() {
                                window.paint_path(path, p.fg);
                            }
                        }
                    },
                )
                .size_full(),
            );
        }
        div()
            .size_full()
            .font_family("Inter")
            .children(trace)
            .child(pill)
    }
}

pub fn open(state: Entity<AppState>, cx: &mut App) -> anyhow::Result<WindowHandle<Overlay>> {
    #[cfg(not(target_os = "linux"))]
    let display = cx.primary_display();
    #[cfg(not(target_os = "linux"))]
    let origin = display.as_ref().map_or(point(px(0.), px(0.)), |display| {
        let bounds = display.bounds();
        point(
            bounds.origin.x + (bounds.size.width - px(SURFACE_WIDTH)) / 2.,
            bounds.origin.y + bounds.size.height - px(SURFACE_HEIGHT),
        )
    });
    cx.open_window(
        WindowOptions {
            titlebar: None,
            focus: false,
            app_id: Some("typeaside-overlay".into()),
            window_background: WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(0.), px(0.)),
                size(px(SURFACE_WIDTH), px(SURFACE_HEIGHT)),
            ))),
            #[cfg(not(target_os = "linux"))]
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                origin,
                size(px(SURFACE_WIDTH), px(SURFACE_HEIGHT)),
            ))),
            #[cfg(not(target_os = "linux"))]
            display_id: display.map(|display| display.id()),
            #[cfg(not(target_os = "linux"))]
            kind: WindowKind::PopUp,
            #[cfg(not(target_os = "linux"))]
            is_movable: false,
            #[cfg(not(target_os = "linux"))]
            is_resizable: false,
            #[cfg(not(target_os = "linux"))]
            is_minimizable: false,
            #[cfg(target_os = "linux")]
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "typeaside-overlay".into(),
                layer: Layer::Overlay,
                anchor: Anchor::BOTTOM,
                keyboard_interactivity: KeyboardInteractivity::None,
                // The transparent shadow gutter places the visible pill 24px above the edge.
                margin: Some((px(0.), px(0.), px(0.), px(0.))),
                ..Default::default()
            }),
            ..Default::default()
        },
        |window, cx| {
            cx.new(|cx| {
                super::theme::apply(state.read(cx).config.theme, window.appearance(), cx);
                cx.observe_window_appearance(window, |this: &mut Overlay, window, cx| {
                    super::theme::apply(this.state.read(cx).config.theme, window.appearance(), cx);
                    cx.notify();
                })
                .detach();
                cx.observe(&state, |_, _, cx| cx.notify()).detach();
                Overlay::new(state)
            })
        },
    )
}
