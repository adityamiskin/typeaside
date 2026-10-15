use super::{
    motion::Spring,
    settings::Toggle,
    theme::*,
    window::{MainWindow, Page, Shortcut},
};
use gpui::{Context, FontWeight, IntoElement, Styled, Window, div, prelude::*, px};
use gpui_kit::component::{Sizable, input::Input, select::Select};
use hermes::{
    config::{RecordingMode, Source},
    engine::{Command, Phase},
};
#[cfg(target_os = "macos")]
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Welcome,
    Account,
    Permissions,
    Microphone,
    Practice,
    Done,
}
impl Step {
    pub fn initial(demo: bool) -> Self {
        if !demo {
            return Self::Welcome;
        }
        match std::env::var("HERMES_DEMO_STEP").as_deref() {
            Ok("1") => Self::Account,
            Ok("2") => Self::Permissions,
            Ok("3") => Self::Microphone,
            Ok("4") => Self::Practice,
            Ok("5") => Self::Done,
            _ => Self::Welcome,
        }
    }
}
pub struct Onboarding {
    steps: Vec<Step>,
    pub pending_key: bool,
    slide: Spring,
    direction: f32,
    entered: Instant,
    welcome_shown: bool,
    welcome_reveal: bool,
    frame: Instant,
    result: Spring,
    had_result: bool,
    #[cfg(target_os = "macos")]
    polling: bool,
    #[cfg(target_os = "macos")]
    reloaded: bool,
    #[cfg(target_os = "macos")]
    grants: [bool; 3],
}
impl Onboarding {
    pub fn visible_steps(demo: bool) -> Vec<Step> {
        let mut steps = vec![Step::Welcome, Step::Account];
        let permissions = demo && std::env::var("HERMES_DEMO_STEP").as_deref() == Ok("2");
        #[cfg(target_os = "macos")]
        let permissions = permissions || !super::permissions::statuses().into_iter().all(|s| s);
        if permissions {
            steps.push(Step::Permissions);
        }
        steps.extend([Step::Microphone, Step::Practice, Step::Done]);
        steps
    }
    pub fn new(steps: Vec<Step>) -> Self {
        let mut slide = Spring::new(0.);
        slide.target = 1.;
        Self {
            pending_key: false,
            steps,
            slide,
            direction: 1.,
            entered: Instant::now(),
            welcome_shown: false,
            welcome_reveal: false,
            frame: Instant::now(),
            result: Spring::new(0.),
            had_result: false,
            #[cfg(target_os = "macos")]
            polling: false,
            #[cfg(target_os = "macos")]
            reloaded: false,
            #[cfg(target_os = "macos")]
            grants: super::permissions::statuses(),
        }
    }
}
fn icon(path: &'static str, size: f32, color: gpui::Hsla) -> impl IntoElement {
    gpui::svg()
        .path(path)
        .size(px(size))
        .flex_shrink_0()
        .text_color(color)
}
fn text(value: impl Into<gpui::SharedString>, size: f32, color: gpui::Hsla) -> gpui::Div {
    div()
        .text_size(px(size))
        .text_color(color)
        .child(value.into())
}
fn title(value: &'static str, p: Palette) -> gpui::Div {
    text(value, 28., p.fg)
        .line_height(px(34.))
        .font_weight(FontWeight::SEMIBOLD)
}
fn sub(value: &'static str, p: Palette) -> gpui::Div {
    text(value, 14., p.muted)
        .line_height(px(21.))
        .max_w(px(380.))
        .mt(px(10.))
}
fn pill(label: &'static str, primary: bool, big: bool, p: Palette) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(if big { 40. } else { 34. }))
        .px(px(if big { 24. } else { 18. }))
        .rounded(px(if big { 20. } else { 17. }))
        .border_1()
        .border_color(if primary { p.accent } else { p.border })
        .when(primary, |d| d.bg(p.accent))
        .text_color(if primary { p.on_accent } else { p.fg })
        .text_size(px(if big { 14. } else { 13. }))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(label)
}
fn bars(level: f32, height: f32, count: usize, p: Palette) -> gpui::Div {
    const SHAPE: [f32; 28] = [
        0.15, 0.25, 0.4, 0.55, 0.7, 0.85, 0.95, 1., 0.9, 0.8, 0.95, 0.75, 0.6, 0.8, 0.7, 0.55,
        0.65, 0.5, 0.4, 0.45, 0.3, 0.35, 0.25, 0.2, 0.18, 0.12, 0.1, 0.08,
    ];
    div()
        .flex()
        .items_center()
        .h(px(height))
        .gap(px(3.))
        .children((0..count).map(|i| {
            div()
                .w(px(4.))
                .h(px((height * SHAPE[i] * level.clamp(0., 1.)).max(4.)))
                .rounded(px(2.))
                .bg(p.accent)
        }))
}
impl MainWindow {
    pub fn init_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.read(cx).config.onboarded {
            return;
        }
        if self.state.read(cx).is_demo {
            self.state.update(cx, |s, _| {
                s.onboarding_own_key =
                    std::env::var("HERMES_DEMO_ACCOUNT").as_deref() != Ok("signed-in");
                s.config.transcription = if s.onboarding_own_key {
                    Source::OwnKey
                } else {
                    Source::Hosted
                };
                if s.onboarding_step == Step::Practice {
                    if std::env::var("HERMES_DEMO_PRACTICE").as_deref() == Ok("listening") {
                        s.phase = Phase::Listening(hermes::engine::Mode::Dictate);
                    } else {
                        s.onboarding_transcript =
                            Some("Let's meet at 3 and bring the deck.".into());
                        s.onboarding_raw = Some(
                            "Let's meet at two, actually three, and um bring the deck.".into(),
                        );
                    }
                }
            });
        }
        window.focus(&self.focus, cx);
        self.onboarding_enter(cx);
    }
    fn onboarding_enter(&mut self, cx: &mut Context<Self>) {
        self.onboarding.entered = Instant::now();
        self.onboarding.frame = Instant::now();
        self.onboarding.slide = Spring::new(0.);
        self.onboarding.slide.target = 1.;
        self.capture = None;
        self.onboarding.pending_key = false;
        let step = self.state.read(cx).onboarding_step;
        self.onboarding.welcome_reveal = step == Step::Welcome && !self.onboarding.welcome_shown;
        if step == Step::Welcome {
            self.onboarding.welcome_shown = true;
        }
        self.state.update(cx, |s, cx| {
            s.mic_testing = step == Step::Microphone;
            s.mic_generation += 1;
            s.command(if s.mic_testing {
                Command::MicPreview
            } else {
                Command::MicTest(false)
            });
            cx.notify();
        });
        #[cfg(target_os = "macos")]
        if step == Step::Permissions && !self.onboarding.polling && !self.state.read(cx).is_demo {
            self.onboarding.polling = true;
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_secs(1)).await;
                    let keep = this
                        .update(cx, |this, cx| {
                            if this.state.read(cx).onboarding_step != Step::Permissions {
                                this.onboarding.polling = false;
                                return false;
                            }
                            let grants = super::permissions::statuses();
                            if grants != this.onboarding.grants {
                                this.onboarding.grants = grants;
                                this.state.update(cx, |s, _| {
                                    s.permission_notice = super::permissions::notice()
                                });
                                cx.notify();
                            }
                            let granted = grants.into_iter().all(|s| s);
                            if granted && !this.onboarding.reloaded {
                                this.onboarding.reloaded = true;
                                this.state.update(cx, |s, _| {
                                    s.command(Command::Reload(Box::new(s.config.clone())))
                                });
                                this.onboarding.polling = false;
                            }
                            !granted
                        })
                        .unwrap_or(false);
                    if !keep {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.notify();
    }
    fn onboarding_ready(&self, cx: &Context<Self>) -> bool {
        let s = self.state.read(cx);
        match s.onboarding_step {
            Step::Account => {
                if s.onboarding_own_key {
                    !self.settings.key.read(cx).value().is_empty()
                        || s.key_hint == "Saved in keyring"
                } else {
                    s.account.is_some()
                }
            }
            Step::Permissions => self.permission_status(cx).into_iter().all(|s| s),
            Step::Microphone => s.onboarding_heard,
            Step::Practice => s.onboarding_transcript.is_some(),
            _ => true,
        }
    }
    fn permission_status(&self, cx: &Context<Self>) -> [bool; 3] {
        if self.state.read(cx).is_demo {
            return [true, false, false];
        }
        #[cfg(target_os = "macos")]
        {
            super::permissions::statuses()
        }
        #[cfg(not(target_os = "macos"))]
        {
            [true; 3]
        }
    }
    pub fn onboarding_next(&mut self, skip: bool, w: &mut Window, cx: &mut Context<Self>) {
        if !skip && !self.onboarding_ready(cx) {
            return;
        }
        let step = self.state.read(cx).onboarding_step;
        if step == Step::Done {
            self.finish_onboarding(w, cx);
            return;
        }
        if step == Step::Account && self.state.read(cx).onboarding_own_key {
            if !self.settings.key.read(cx).value().is_empty() {
                self.onboarding.pending_key = true;
                self.save_key(false, w, cx);
                return;
            }
            if self.state.read(cx).key_hint != "Saved in keyring" {
                return;
            }
        }
        if step == Step::Account {
            self.state.update(cx, |s, _| s.persist());
        }
        if let Some(index) = self.onboarding.steps.iter().position(|s| *s == step)
            && let Some(next) = self.onboarding.steps.get(index + 1).copied()
        {
            self.state.update(cx, |s, _| s.onboarding_step = next);
            self.onboarding.direction = 1.;
            self.onboarding_enter(cx);
        }
    }
    pub fn onboarding_back(&mut self, cx: &mut Context<Self>) {
        let step = self.state.read(cx).onboarding_step;
        if let Some(index) = self.onboarding.steps.iter().position(|s| *s == step)
            && index > 0
        {
            let previous = self.onboarding.steps[index - 1];
            self.state.update(cx, |s, _| s.onboarding_step = previous);
            self.onboarding.direction = -1.;
            self.onboarding_enter(cx);
        }
    }
    fn finish_onboarding(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.config.onboarded = true;
            s.mic_testing = false;
            s.mic_generation += 1;
            s.command(Command::MicTest(false));
            s.persist();
            cx.notify();
        });
        self.set_page(Page::Home, w, cx);
    }
    pub fn onboarding(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let now = Instant::now();
        let dt = now
            .duration_since(self.onboarding.frame)
            .as_secs_f32()
            .min(0.032);
        self.onboarding.frame = now;
        let moving = self.onboarding.slide.step(dt, 260., 28.);
        let s = self.state.read(cx);
        let step = s.onboarding_step;
        let elapsed = self.onboarding.entered.elapsed().as_secs_f32();
        let reveal = |delay: f32| {
            if self.onboarding.welcome_reveal {
                ((elapsed - delay) / 0.3).clamp(0., 1.)
            } else {
                1.
            }
        };
        let has_result = s.onboarding_transcript.is_some();
        if has_result && !self.onboarding.had_result {
            self.onboarding.result = Spring::new(0.);
            self.onboarding.result.target = 1.;
        }
        self.onboarding.had_result = has_result;
        let result_moving = self.onboarding.result.step(dt, 260., 28.);
        if moving || result_moving || (self.onboarding.welcome_reveal && elapsed < 0.7) {
            w.request_animation_frame();
        }
        let mut col = div()
            .w(px(440.))
            .flex()
            .flex_col()
            .items_center()
            .text_center();
        col = match step {
            Step::Welcome => col
                .child(
                    div()
                        .size(px(64.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(px(64. * (0.92 + 0.08 * reveal(0.))))
                                .rounded(px(16.))
                                .bg(p.accent)
                                .opacity(reveal(0.)),
                        ),
                )
                .child(
                    text("Your voice, written.", 34., p.fg)
                        .line_height(px(40.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .mt(px(32.))
                        .relative()
                        .top(px(10. * (1. - reveal(0.12))))
                        .opacity(reveal(0.12)),
                )
                .child(
                    sub(
                        "Hold a key, speak naturally, and Hermes types clean text into any app.",
                        p,
                    )
                    .mt(px(12.))
                    .relative()
                    .top(px(10. * (1. - reveal(0.22))))
                    .opacity(reveal(0.22)),
                )
                .child(
                    div()
                        .mt(px(32.))
                        .relative()
                        .top(px(10. * (1. - reveal(0.34))))
                        .opacity(reveal(0.34))
                        .child(
                            pill("Get started", true, true, p)
                                .id("get-started")
                                .on_click(
                                    cx.listener(|this, _, w, cx| {
                                        this.onboarding_next(false, w, cx)
                                    }),
                                ),
                        ),
                )
                .child(
                    text("Takes about a minute.", 12., p.faint)
                        .mt(px(18.))
                        .mb(px(12.))
                        .opacity(reveal(0.34)),
                ),
            Step::Account => {
                let own = s.onboarding_own_key;
                let waiting = s.signing_in.is_some();
                let email = s.account.as_ref().map(|a| a.email.clone());
                col.child(title("How should Hermes listen?",p)).child(sub("You can change this anytime in Settings.",p))
                .child(div().flex().gap(px(16.)).mt(px(32.)).children([false,true].into_iter().map(|key| {
                    let selected = key == own;
                    div().id(if key {"own-key"} else {"hermes-account"}).w(px(212.)).h(px(132.)).p(px(18.)).rounded(px(14.)).border_1().border_color(if selected {p.accent} else {p.border}).when(selected,|d| d.bg(p.accent.opacity(0.06))).flex().flex_col().text_left().cursor_pointer()
                    .child(icon(if key {"icons/key-round.svg"} else {"icons/sparkles.svg"},18.,p.accent))
                    .child(text(if key {"Your own key"} else {"Hermes account"},14.,p.fg).font_weight(FontWeight::SEMIBOLD).mt(px(12.)))
                    .child(text(if key {"Groq or any OpenAI-compatible endpoint."} else {"No setup. Sign in with your browser."},12.,p.muted).line_height(px(18.)).mt(px(4.)))
                    .child(div().flex_1())
                    .child(div().flex().items_center().gap(px(4.)).text_size(px(12.)).text_color(if key {p.muted} else if email.is_some() {p.green} else {p.accent})
                        .when(!key && email.is_some(),|d| d.child(icon("icons/check.svg",14.,p.green)))
                        .child(if key {"Unlimited, billed by provider".into()} else if waiting {"Waiting for browser".into()} else {email.clone().unwrap_or_else(|| "Free · 2,000 words a week".into())})
                        .when(!key && waiting,|d| d.child(text("Cancel",12.,p.muted).id("cancel-onboarding-signin").on_click(cx.listener(|this,_,_,cx| {cx.stop_propagation(); this.state.update(cx,|s,cx| {if let Some(cancel)=s.signing_in.take() {cancel.store(true,std::sync::atomic::Ordering::SeqCst);} cx.notify();});})))) )
                    .on_click(cx.listener(move |this,_,_,cx| {
                        this.state.update(cx,|s,cx| {s.onboarding_own_key=key; s.config.transcription=if key {Source::OwnKey} else {Source::Hosted}; if key && let Some(cancel)=s.signing_in.take() {cancel.store(true,std::sync::atomic::Ordering::SeqCst);} cx.notify();});
                        if !key && this.state.read(cx).account.is_none() {this.sign_in(cx);}
                    }))
                })))
                .when(own, |d| d.child(div().w_full().mt(px(16.)).child(Input::new(&self.settings.key).min_h(px(34.)).max_h(px(34.)).w_full().rounded(px(8.)).text_size(px(13.)).bg(p.bg))))
            }
            Step::Permissions => {
                let statuses = self.permission_status(cx);
                col.child(title("A few permissions", p))
                    .child(sub("Hermes listens only while you hold your key.", p))
                    .child(
                        div()
                            .w_full()
                            .mt(px(28.))
                            .border_b_1()
                            .border_color(p.divider)
                            .children(
                                [
                                    (
                                        "Microphone",
                                        "To hear you while you hold the key.",
                                        "icons/mic.svg",
                                    ),
                                    (
                                        "Accessibility",
                                        "To paste text where your cursor is.",
                                        "icons/accessibility.svg",
                                    ),
                                    (
                                        "Input Monitoring",
                                        "To notice your shortcut in any app.",
                                        "icons/keyboard.svg",
                                    ),
                                ]
                                .into_iter()
                                .enumerate()
                                .map(|(i, (name, desc, path))| {
                                    div()
                                        .h(px(60.))
                                        .w_full()
                                        .border_t_1()
                                        .border_color(p.divider)
                                        .flex()
                                        .items_center()
                                        .gap(px(14.))
                                        .text_left()
                                        .child(
                                            div()
                                                .size(px(32.))
                                                .rounded_full()
                                                .bg(p.row)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(icon(path, 18., p.accent)),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(
                                                    text(name, 14., p.fg)
                                                        .font_weight(FontWeight::SEMIBOLD),
                                                )
                                                .child(text(desc, 12., p.muted).mt(px(2.))),
                                        )
                                        .child(if statuses[i] {
                                            div()
                                                .flex()
                                                .gap(px(6.))
                                                .items_center()
                                                .child(icon("icons/check.svg", 18., p.green))
                                                .child(text("Allowed", 13., p.green))
                                                .into_any_element()
                                        } else {
                                            pill("Allow", i == 1, false, p)
                                                .h(px(28.))
                                                .px(px(14.))
                                                .text_size(px(12.))
                                                .id(("permission", i))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    #[cfg(target_os = "macos")]
                                                    if !this.state.read(cx).is_demo {
                                                        super::permissions::request_one(i);
                                                    }
                                                    #[cfg(not(target_os = "macos"))]
                                                    let _ = (this, cx);
                                                }))
                                                .into_any_element()
                                        })
                                }),
                            ),
                    )
            }
            Step::Microphone => col
                .child(title("Say something", p))
                .child(sub("The bars should move with your voice.", p))
                .child(div().mt(px(40.)).child(bars(
                    if s.is_demo { 1. } else { s.level },
                    72.,
                    28,
                    p,
                )))
                .child(
                    div().mt(px(28.)).w(px(240.)).child(
                        Select::new(&self.settings.device)
                            .small()
                            .w_full()
                            .min_h(px(30.))
                            .max_h(px(30.))
                            .bg(p.bg)
                            .text_left()
                            .text_color(p.muted)
                            .text_size(px(13.)),
                    ),
                )
                .child(
                    div()
                        .mt(px(16.))
                        .h(px(18.))
                        .flex()
                        .gap(px(6.))
                        .items_center()
                        .when(s.onboarding_heard, |d| {
                            d.child(icon("icons/check.svg", 15., p.green)).child(text(
                                "Sounds great",
                                13.,
                                p.green,
                            ))
                        }),
                ),
            Step::Practice => {
                let listening = matches!(s.phase, Phase::Listening(_));
                let caption = s
                    .onboarding_raw
                    .as_ref()
                    .zip(s.onboarding_transcript.as_ref())
                    .is_some_and(|(a, b)| a != b);
                col.child(
                    div()
                        .h(px(56.))
                        .px(px(22.))
                        .rounded(px(12.))
                        .border_1()
                        .border_b(px(if listening { 1. } else { 3. }))
                        .border_color(if listening {
                            p.accent.opacity(0.5)
                        } else {
                            p.kbd
                        })
                        .bg(if listening {
                            p.accent.opacity(0.14)
                        } else {
                            p.selected
                        })
                        .relative()
                        .top(px(if listening { 2. } else { 0. }))
                        .flex()
                        .items_center()
                        .text_size(px(18.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if listening { p.accent } else { p.fg })
                        .child(key_label(&s.config.hotkey))
                        .when(!listening, |key| {
                            key.child(
                                div()
                                    .absolute()
                                    .bottom(px(-3.))
                                    .left_0()
                                    .right_0()
                                    .h(px(3.))
                                    .rounded_b(px(12.))
                                    .bg(gpui::Hsla {
                                        l: p.kbd.l * 0.7,
                                        ..p.kbd
                                    }),
                            )
                        }),
                )
                .child(
                    text(
                        if self.capture.is_some() {
                            "Press keys…"
                        } else {
                            "Change shortcut"
                        },
                        12.,
                        p.muted,
                    )
                    .mt(px(8.))
                    .id("onboarding-shortcut")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, w, cx| {
                        this.capture = Some(Shortcut::Dictate);
                        this.capture_modifiers = gpui::Modifiers::default();
                        w.focus(&this.focus, cx);
                        cx.notify();
                    })),
                )
                .child(
                    title(
                        if s.config.mode == RecordingMode::Toggle {
                            "Press, read, press."
                        } else {
                            "Hold, read, release."
                        },
                        p,
                    )
                    .mt(px(7.)),
                )
                .child(
                    text(
                        "“Let's meet at two, actually three, and um bring the deck.”",
                        20.,
                        p.fg.opacity(0.82),
                    )
                    .line_height(px(28.))
                    .max_w(px(420.))
                    .mt(px(22.))
                    .mb(px(20.)),
                )
                .child(
                    div()
                        .w_full()
                        .min_h(px(96.))
                        .mt(px(28.))
                        .rounded(px(12.))
                        .bg(p.row)
                        .border_1()
                        .border_color(p.accent.opacity(if listening { 0.6 } else { 0. }))
                        .py(px(16.))
                        .px(px(18.))
                        .text_left()
                        .text_size(px(15.))
                        .line_height(px(22.))
                        .child(if listening {
                            bars(s.level, 24., 18, p).into_any_element()
                        } else if let Some(result) = &s.onboarding_transcript {
                            text(result.clone(), 15., p.fg)
                                .opacity(self.onboarding.result.value.clamp(0., 1.))
                                .into_any_element()
                        } else {
                            text("Your text will appear here.", 15., p.faint).into_any_element()
                        }),
                )
                .child(
                    div()
                        .h(px(16.))
                        .mt(px(10.))
                        .flex()
                        .gap(px(6.))
                        .when(caption, |d| {
                            d.child(text("Filler removed. Your correction kept.", 12., p.muted))
                        })
                        .when(has_result, |d| {
                            d.child(
                                text("Try again", 12., p.accent)
                                    .id("practice-again")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.update(cx, |s, cx| {
                                            s.onboarding_transcript = None;
                                            s.onboarding_raw = None;
                                            cx.notify();
                                        });
                                    })),
                            )
                        }),
                )
            }
            Step::Done => {
                let login = s.config.launch_at_login;
                let usage = s.config.share_usage;
                let scale = 0.6 + 0.4 * self.onboarding.slide.value;
                col.child(
                    div()
                        .size(px(56.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(px(56. * scale))
                                .rounded_full()
                                .bg(p.accent)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon("icons/check.svg", 28. * scale, p.on_accent)),
                        ),
                )
                .child(title("You're all set.", p).mt(px(24.)))
                .child(sub("Hermes stays in your menu bar. Use it in any app.", p))
                .child(
                    div().w(px(360.)).mt(px(28.)).children(
                        [
                            (Some(s.config.hotkey.as_str()), "Hold to dictate"),
                            (
                                s.config.command_hotkey.as_deref(),
                                "Rewrite selected text by voice",
                            ),
                            (
                                s.config.paste_last_hotkey.as_deref(),
                                "Paste your last dictation",
                            ),
                        ]
                        .into_iter()
                        .filter_map(|(key, label)| {
                            key.filter(|k| !k.is_empty()).map(|key| {
                                div()
                                    .h(px(36.))
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(
                                        kbd(key_label(key), p)
                                            .min_w(px(96.))
                                            .bg(p.kbd)
                                            .text_center(),
                                    )
                                    .child(text(label, 13., p.muted))
                            })
                        }),
                    ),
                )
                .child(
                    div()
                        .w(px(360.))
                        .mt(px(14.))
                        .border_t_1()
                        .border_color(p.divider)
                        .child(
                            div()
                                .h(px(40.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(text("Open at login", 13., p.fg))
                                .child(self.toggle("onboarding-login", login, Toggle::Login, cx)),
                        )
                        .child(
                            div()
                                .h(px(40.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(text("Share anonymous usage stats", 13., p.fg))
                                .child(self.toggle(
                                    "onboarding-usage",
                                    usage,
                                    Toggle::ShareUsage,
                                    cx,
                                )),
                        ),
                )
                .child(
                    div().mt(px(28.)).child(
                        pill("Start dictating", true, true, p)
                            .id("finish-onboarding")
                            .on_click(cx.listener(|this, _, w, cx| this.finish_onboarding(w, cx))),
                    ),
                )
            }
        };
        let ready = self.onboarding_ready(cx);
        let full = matches!(step, Step::Welcome | Step::Done);
        let index = self
            .onboarding
            .steps
            .iter()
            .position(|s| *s == step)
            .unwrap_or(0);
        div()
            .size_full()
            .relative()
            .bg(gpui::linear_gradient(
                180.,
                gpui::linear_color_stop(p.accent.opacity(0.07), 0.),
                gpui::linear_color_stop(p.accent.opacity(0.), 0.55),
            ))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom(px(if full { 0. } else { 72. }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        col.relative()
                            .left(px(28.
                                * self.onboarding.direction
                                * (1. - self.onboarding.slide.value)))
                            .opacity(self.onboarding.slide.value.clamp(0., 1.)),
                    ),
            )
            .when(!full, |root| {
                root.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(px(72.))
                        .px(px(32.))
                        .flex()
                        .items_center()
                        .child(
                            div().flex_1().flex().child(
                                pill("Back", false, false, p)
                                    .id("onboarding-back")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.onboarding_back(cx)),
                                    ),
                            ),
                        )
                        .child(
                            div().flex().gap(px(6.)).children(
                                self.onboarding
                                    .steps
                                    .iter()
                                    .filter(|s| **s != Step::Done)
                                    .enumerate()
                                    .map(|(i, _)| {
                                        div()
                                            .w(px(if i == index { 18. } else { 6. }))
                                            .h(px(6.))
                                            .rounded(px(3.))
                                            .bg(if i == index {
                                                p.accent
                                            } else if i < index {
                                                p.faint
                                            } else {
                                                p.fg.opacity(0.14)
                                            })
                                    }),
                            ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .justify_end()
                                .items_center()
                                .gap(px(16.))
                                .when(matches!(step, Step::Permissions | Step::Practice), |d| {
                                    d.child(
                                        text(
                                            if step == Step::Permissions {
                                                "Not now"
                                            } else {
                                                "Skip"
                                            },
                                            13.,
                                            p.muted,
                                        )
                                        .id("onboarding-skip")
                                        .cursor_pointer()
                                        .on_click(
                                            cx.listener(|this, _, w, cx| {
                                                this.onboarding_next(true, w, cx)
                                            }),
                                        ),
                                    )
                                })
                                .child(
                                    pill("Continue", true, false, p)
                                        .opacity(if ready { 1. } else { 0.4 })
                                        .id("onboarding-continue")
                                        .when(ready, |d| {
                                            d.on_click(cx.listener(|this, _, w, cx| {
                                                this.onboarding_next(false, w, cx)
                                            }))
                                        }),
                                ),
                        ),
                )
            })
    }
}
