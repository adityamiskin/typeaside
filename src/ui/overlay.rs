use super::{AppState, motion::Spring, theme::*};
#[cfg(target_os = "linux")]
use gpui::layer_shell::*;
use gpui::{
    App, AppContext, Bounds, BoxShadow, Context, Entity, PathBuilder, Render, TextRun, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, canvas, div,
    fill, font, point, prelude::*, px, size,
};
use std::time::{Duration, Instant};
use typeaside::engine::{Command, Mode, Phase};

#[derive(Clone, PartialEq)]
enum Visual {
    Listening,
    Paused,
    Processing,
    Success,
    Cancel,
    Error(String),
    Notice(&'static str, String, Vec<String>),
    Exit,
}

pub struct Overlay {
    state: Entity<AppState>,
    mode: Mode,
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
    clickable: Option<bool>,
}

impl Overlay {
    fn new(state: Entity<AppState>) -> Self {
        let now = Instant::now();
        Self {
            state,
            mode: Mode::Dictate,
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
            clickable: None,
        }
    }

    pub fn level(&mut self, level: f32, cx: &mut Context<Self>) {
        if self.phase == Visual::Paused {
            return;
        }
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
                self.mode = mode;
                cx.notify();
                Visual::Listening
            }
            Phase::Paused(mode) => {
                self.mode = mode;
                Visual::Paused
            }
            Phase::Transcribing | Phase::Cleaning => Visual::Processing,
            Phase::Error(message) => Visual::Error(message),
            Phase::Idle if matches!(self.phase, Visual::Listening | Visual::Paused) => {
                Visual::Cancel
            }
            Phase::Idle if self.phase == Visual::Processing => Visual::Exit,
            Phase::Idle => return,
        };
        self.set(next, window, cx);
    }

    pub fn success(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set(Visual::Success, window, cx);
    }

    pub fn copied(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let preview = text
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        self.notice("Copied", &preview, &[], |s| format!("“{s}”"), window, cx);
    }

    pub fn exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set(Visual::Exit, window, cx);
    }

    pub fn opened(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Ok(url) = reqwest::Url::parse(url) {
            let label = if url.scheme() == "mailto" {
                "email"
            } else {
                url.host_str().unwrap_or_default()
            };
            self.notice("Opened", label, &[], str::to_owned, window, cx);
        }
    }

    pub fn learned(&mut self, words: &[String], window: &mut Window, cx: &mut Context<Self>) {
        self.notice(
            "Learned",
            &words.join(", "),
            words,
            str::to_owned,
            window,
            cx,
        );
    }

    fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Visual::Notice(_, _, words) = &self.phase else {
            return;
        };
        if words.is_empty() {
            return;
        }
        let words = words.clone();
        let saved = self.state.update(cx, |s, cx| {
            let previous = s.config.clone();
            for word in &words {
                s.config.remove_word(word);
            }
            s.save_error = s
                .config
                .save(&s.paths)
                .err()
                .map(|error| format!("Could not undo learned words: {error}"));
            if s.save_error.is_some() {
                s.config = previous;
                cx.notify();
                return false;
            }
            let _ = s.sync.try_send(());
            s.command(Command::Unlearn(words));
            cx.notify();
            true
        });
        if saved {
            self.set(Visual::Exit, window, cx);
        }
    }

    fn notice(
        &mut self,
        label: &'static str,
        text: &str,
        words: &[String],
        wrap: impl Fn(&str) -> String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let action_width = if words.is_empty() {
            0.
        } else {
            text_width("Undo", window, cx) + COPIED_GAP
        };
        let available =
            SURFACE_WIDTH - 48. - 46. - COPIED_GAP - text_width(label, window, cx) - action_width;
        let mut shown = wrap(text);
        if text_width(&shown, window, cx) > available {
            let ends = text.char_indices().map(|(i, _)| i).collect::<Vec<_>>();
            let fits = |end: usize| {
                text_width(&wrap(&format!("{}…", &text[..end])), window, cx) <= available
            };
            let end = ends[ends.partition_point(|&end| fits(end)).saturating_sub(1)];
            shown = wrap(&format!("{}…", &text[..end]));
        }
        self.set(Visual::Notice(label, shown, words.to_vec()), window, cx);
    }

    fn set(&mut self, phase: Visual, window: &mut Window, cx: &mut Context<Self>) {
        if self.phase == phase {
            return;
        }
        self.phase = phase;
        self.phase_at = Instant::now();
        if self.phase == Visual::Paused {
            self.width = Spring::new(96.);
            self.rise = Spring::new(0.);
            self.scale = Spring::new(1.);
            self.bars = [Spring::new(0.); BARS];
        }
        let text = match &self.phase {
            Visual::Error(message) => Some(text_width(message, window, cx)),
            Visual::Notice(label, text, words) => {
                let action_width = if words.is_empty() {
                    0.
                } else {
                    text_width("Undo", window, cx) + COPIED_GAP
                };
                Some(
                    text_width(label, window, cx)
                        + COPIED_GAP
                        + text_width(text, window, cx)
                        + action_width,
                )
            }
            _ => None,
        };
        if let Some(text) = text {
            self.width.target = (text + 46.).clamp(96., SURFACE_WIDTH - 48.);
            self.dot = Spring::new(0.);
            self.dot.target = 1.;
        }
        let hold = match self.phase {
            Visual::Success => Some(CHECK_TIME + SUCCESS_HOLD),
            Visual::Cancel => Some(CANCEL_TIME),
            Visual::Error(_) => Some(ERROR_HOLD),
            Visual::Notice(_, _, ref words) if !words.is_empty() => Some(LEARNED_HOLD),
            Visual::Notice(..) => Some(COPIED_HOLD),
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
        if self.phase == Visual::Paused {
            self.last_frame = Instant::now();
            return (1., 0., 0.);
        }
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
                    if matches!(self.mode, Mode::Command | Mode::Translate) {
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
            Visual::Paused | Visual::Cancel | Visual::Exit => {}
            Visual::Error(_) | Visual::Notice(..) => {}
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
        let mark = match self.mode {
            _ if self.phase != Visual::Listening => None,
            Mode::Command => Some(("✦", p.lilac)),
            Mode::Translate => Some(("⇄", p.blue)),
            Mode::Dictate | Mode::File => None,
        };
        let marked = mark.is_some();
        let (opacity, elapsed, _) = self.advance(window);
        let kind = match &self.phase {
            Visual::Listening if self.mode == Mode::Command => "command",
            Visual::Listening if self.mode == Mode::Translate => "translate",
            Visual::Listening => "listening",
            Visual::Paused => "paused",
            Visual::Processing => "processing",
            Visual::Success => "success",
            Visual::Error(_) => "error",
            Visual::Notice(..) => "notice",
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
        let clickable = matches!(&self.phase, Visual::Notice(_, _, words) if !words.is_empty());
        // Each region change commits the surface, so only send it when Undo appears or goes.
        #[cfg(target_os = "linux")]
        if self.clickable != Some(clickable) {
            self.clickable = Some(clickable);
            let target = self.width.target;
            let region = Bounds::new(
                point(
                    px((SURFACE_WIDTH - target) / 2.),
                    px(SURFACE_HEIGHT - SHADOW_SPACE - PILL_HEIGHT),
                ),
                size(px(target), px(PILL_HEIGHT)),
            );
            let regions = [region];
            window.set_input_region(Some(if clickable { &regions[..] } else { &[] }));
        }
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
        if self.phase == Visual::Paused {
            pill = pill.child(div().text_color(p.fg).text_size(px(13.)).child("Ⅱ Paused"));
        } else if let Visual::Error(message) | Visual::Notice(_, message, _) = &self.phase {
            let label = match self.phase {
                Visual::Notice(label, _, _) => Some(label),
                _ => None,
            };
            let notice = label.is_some();
            let text_progress = (elapsed / ERROR_TEXT_TIME).min(1.);
            pill =
                pill.child(
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
                                .bg(if notice { p.blue } else { p.red }),
                        )
                        .child(
                            div()
                                .relative()
                                .left(px(6. * (1. - text_progress)))
                                .opacity(text_progress)
                                .text_color(p.fg)
                                .text_size(px(13.))
                                .flex()
                                .gap(px(COPIED_GAP))
                                .whitespace_nowrap()
                                .children(label)
                                .child(
                                    div()
                                        .when(notice, |el| el.text_color(p.muted))
                                        .child(message.clone()),
                                )
                                .when(clickable, |el| {
                                    el.child(
                                        div()
                                            .id("undo-learned")
                                            .cursor_pointer()
                                            .child("Undo")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.undo(window, cx)
                                            })),
                                    )
                                }),
                        ),
                );
        } else {
            if let Some((glyph, color)) = mark {
                pill = pill.child(
                    div()
                        .absolute()
                        .left(px(width / 2. - 40.5))
                        .w(px(12.))
                        .text_size(px(12.))
                        .line_height(px(12.))
                        .text_color(color)
                        .child(glyph),
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
                                    center.x + px((x + if marked { 9. } else { 0. }) * scale - 1.5),
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
    let handle = cx.open_window(
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
    )?;
    #[cfg(target_os = "macos")]
    handle.update(cx, |_, window, _| borderless(window))?;
    Ok(handle)
}

/// GPUI gives untitled windows a titled frame, whose border and shadow outline the whole surface.
#[cfg(target_os = "macos")]
fn borderless(window: &Window) {
    use objc2_app_kit::{NSView, NSWindowStyleMask};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    if let Some(window) = view.window() {
        window.setStyleMask(NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel);
        window.setHasShadow(false);
    }
}

fn text_width(text: &str, window: &Window, cx: &App) -> f32 {
    let run = TextRun {
        len: text.len(),
        font: font("Inter"),
        color: palette(cx).fg,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    f32::from(
        window
            .text_system()
            .shape_line(text.to_owned().into(), px(13.), &[run], None)
            .width,
    )
}

const ANSWER_WIDTH: f32 = 440.;
const ANSWER_BOTTOM: f32 = SHADOW_SPACE + PILL_HEIGHT + 12.;

pub struct AnswerCard {
    state: Entity<AppState>,
    prompt: String,
    text: String,
    body_height: f32,
    entered: Instant,
    last_frame: Instant,
    rise: Spring,
    copied_at: Option<Instant>,
    #[cfg(target_os = "linux")]
    input_region: Option<Bounds<gpui::Pixels>>,
}

impl AnswerCard {
    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.read(cx).command(Command::DismissReply);
        #[cfg(target_os = "linux")]
        window.set_input_region(Some(&[]));
        window.remove_window();
    }
}

impl Render for AnswerCard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.032);
        self.last_frame = now;
        self.rise.target = 0.;
        let moving = self.rise.step(dt, PILL_STIFFNESS, PILL_DAMPING);
        let opacity = (now.duration_since(self.entered).as_secs_f32() / ENTER_FADE).min(1.);
        if moving || opacity < 1. {
            window.request_animation_frame();
        }
        #[cfg(target_os = "linux")]
        let (left, top) = (40., 24. + self.rise.value);
        #[cfg(not(target_os = "linux"))]
        let (left, top) = (0., self.rise.value);
        #[cfg(target_os = "linux")]
        {
            let region = Bounds::new(
                point(px(left), px(top)),
                size(px(ANSWER_WIDTH), px(self.body_height + 92.)),
            );
            if self.input_region != Some(region) {
                window.set_input_region(Some(&[region]));
                self.input_region = Some(region);
            }
        }
        div().size_full().font_family("Inter").child(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(ANSWER_WIDTH))
                .p(px(16.))
                .rounded(px(14.))
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
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .child(div().text_color(p.lilac).child("✦"))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(p.muted)
                                .child(self.prompt.clone()),
                        )
                        .child(
                            div()
                                .id("close-answer")
                                .cursor_pointer()
                                .text_size(px(14.))
                                .text_color(p.muted)
                                .hover(|el| el.text_color(p.fg))
                                .child("×")
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.close(window, cx)),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("answer-body")
                        .h(px(self.body_height))
                        .max_h(px(220.))
                        .overflow_y_scroll()
                        .text_size(px(13.))
                        .line_height(px(19.))
                        .text_color(p.fg)
                        .child(self.text.clone()),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(16.))
                        .text_size(px(13.))
                        .line_height(px(18.))
                        .text_color(p.muted)
                        .child(
                            div()
                                .id("copy-answer")
                                .cursor_pointer()
                                .hover(|el| el.text_color(p.fg))
                                .child(if self.copied_at.is_some() {
                                    "Copied"
                                } else {
                                    "Copy"
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    match typeaside::text_injector::set_clipboard(&this.text) {
                                        Ok(()) => {
                                            let copied_at = Instant::now();
                                            this.copied_at = Some(copied_at);
                                            cx.notify();
                                            cx.spawn(async move |this, cx| {
                                                cx.background_executor()
                                                    .timer(Duration::from_millis(1200))
                                                    .await;
                                                let _ = this.update(cx, |this, cx| {
                                                    if this.copied_at == Some(copied_at) {
                                                        this.copied_at = None;
                                                        cx.notify();
                                                    }
                                                });
                                            })
                                            .detach();
                                        }
                                        Err(error) => this.state.update(cx, |state, cx| {
                                            state.save_error =
                                                Some(format!("Could not copy answer: {error}"));
                                            cx.notify();
                                        }),
                                    }
                                })),
                        )
                        .child(
                            div()
                                .id("insert-answer")
                                .cursor_pointer()
                                .hover(|el| el.text_color(p.fg))
                                .child("Insert")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.state
                                        .read(cx)
                                        .command(Command::Insert(this.text.clone()));
                                    this.close(window, cx);
                                })),
                        ),
                ),
        )
    }
}

pub fn open_answer(
    state: Entity<AppState>,
    prompt: String,
    text: String,
    cx: &mut App,
) -> anyhow::Result<WindowHandle<AnswerCard>> {
    let run = TextRun {
        len: text.len(),
        font: font("Inter"),
        color: palette(cx).fg,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let text_system = gpui::WindowTextSystem::new(cx.text_system().clone());
    let lines = text_system.shape_text(
        text.clone().into(),
        px(13.),
        &[run],
        Some(px(ANSWER_WIDTH - 34.)),
        None,
    )?;
    let body_height = (lines
        .iter()
        .map(|line| line.wrap_boundaries.len() + 1)
        .sum::<usize>() as f32
        * 19.)
        .clamp(19., 220.);
    let height = body_height + 92.;
    #[cfg(not(target_os = "linux"))]
    let display = cx.primary_display();
    #[cfg(not(target_os = "linux"))]
    let origin = display.as_ref().map_or(point(px(0.), px(0.)), |display| {
        let bounds = display.bounds();
        point(
            bounds.origin.x + (bounds.size.width - px(ANSWER_WIDTH)) / 2.,
            bounds.origin.y + bounds.size.height - px(ANSWER_BOTTOM + height),
        )
    });
    let handle = cx.open_window(
        WindowOptions {
            titlebar: None,
            focus: false,
            app_id: Some("typeaside-answer".into()),
            window_background: WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(0.), px(0.)),
                size(px(ANSWER_WIDTH + 80.), px(height + 72.)),
            ))),
            #[cfg(target_os = "linux")]
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "typeaside-answer".into(),
                layer: Layer::Overlay,
                anchor: Anchor::BOTTOM,
                keyboard_interactivity: KeyboardInteractivity::None,
                margin: Some((px(0.), px(0.), px(ANSWER_BOTTOM - 48.), px(0.))),
                ..Default::default()
            }),
            #[cfg(not(target_os = "linux"))]
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                origin,
                size(px(ANSWER_WIDTH), px(height)),
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
            ..Default::default()
        },
        |window, cx| {
            #[cfg(target_os = "linux")]
            window.set_input_region(Some(&[]));
            cx.new(|cx| {
                super::theme::apply(state.read(cx).config.theme, window.appearance(), cx);
                cx.observe_window_appearance(window, |this: &mut AnswerCard, window, cx| {
                    super::theme::apply(this.state.read(cx).config.theme, window.appearance(), cx);
                    cx.notify();
                })
                .detach();
                let now = Instant::now();
                AnswerCard {
                    state,
                    prompt: prompt.split_whitespace().collect::<Vec<_>>().join(" "),
                    text,
                    body_height,
                    entered: now,
                    last_frame: now,
                    rise: Spring::new(8.),
                    copied_at: None,
                    #[cfg(target_os = "linux")]
                    input_region: None,
                }
            })
        },
    )?;
    #[cfg(target_os = "macos")]
    handle.update(cx, |_, window, _| borderless(window))?;
    Ok(handle)
}
