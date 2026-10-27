use super::{
    AppState,
    motion::Spring,
    settings::{DownloadTarget, Toggle},
    theme::*,
    window::{MainWindow, Page, Shortcut},
};
use gpui::{
    AnyElement, Context, Entity, FontWeight, IntoElement, Styled, Window, div, img, prelude::*, px,
};
use gpui_kit::component::input::{Input, InputState};
use std::time::Instant;
use typeaside::{
    config::{RecordingMode, Source, SpeechSource},
    engine::{Command, Phase},
    local::{CleanupModel, Status},
};

const COLUMN: f32 = 440.;
const PRACTICE: &str = "Yes, um, let's meet at two, actually three. I'll bring the deck.";
const EXAMPLE_RAW: &str = "so um can we move it to two, actually three";
const EXAMPLE_CLEAN: &str = "Can we move it to 3?";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Engine,
    Permissions,
    Microphone,
    Practice,
}
impl Step {
    pub fn initial(demo: bool) -> Self {
        if !demo {
            return Self::Engine;
        }
        match std::env::var("TYPEASIDE_DEMO_STEP").as_deref() {
            Ok("1") => Self::Permissions,
            Ok("2") => Self::Microphone,
            Ok("3") => Self::Practice,
            _ => Self::Engine,
        }
    }
}
pub struct Onboarding {
    steps: Vec<Step>,
    /// The key being saved before Continue advances; true for the editing key.
    pub pending_key: Option<bool>,
    /// A real field the practice dictation is typed into, so the whole path is proven.
    pub reply: Entity<InputState>,
    slide: Spring,
    direction: f32,
    frame: Instant,
    result: Spring,
    had_result: bool,
}
impl Onboarding {
    pub fn visible_steps(demo: bool) -> Vec<Step> {
        let permissions = demo && std::env::var("TYPEASIDE_DEMO_STEP").as_deref() == Ok("1");
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let permissions = permissions || !super::permissions::statuses().into_iter().all(|s| s);
        let mut steps = vec![Step::Engine];
        if permissions {
            steps.push(Step::Permissions);
        }
        steps.extend([Step::Microphone, Step::Practice]);
        steps
    }
    pub fn new(steps: Vec<Step>, reply: Entity<InputState>) -> Self {
        let mut slide = Spring::new(0.);
        slide.target = 1.;
        Self {
            steps,
            pending_key: None,
            reply,
            slide,
            direction: 1.,
            frame: Instant::now(),
            result: Spring::new(0.),
            had_result: false,
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
    text(value, 26., p.fg)
        .line_height(px(32.))
        .font_weight(FontWeight::SEMIBOLD)
}
fn sub(value: impl Into<gpui::SharedString>, p: Palette) -> gpui::Div {
    text(value, 14., p.muted).line_height(px(21.)).mt(px(8.))
}
fn link(value: &'static str, p: Palette) -> gpui::Div {
    text(value, 13., p.blue).cursor_pointer()
}
fn tile(
    id: &'static str,
    selected: bool,
    name: &'static str,
    detail: impl Into<gpui::SharedString>,
    aside: Option<String>,
    p: Palette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w_full()
        .mt(px(10.))
        .px(px(14.))
        .py(px(12.))
        .rounded(px(8.))
        .border_1()
        .border_color(if selected { p.blue } else { p.border })
        .when(selected, |d| d.bg(p.tint))
        .cursor_pointer()
        .child(
            div()
                .flex()
                .justify_between()
                .child(text(name, 14., p.fg).font_weight(FontWeight::SEMIBOLD))
                .children(aside.map(|aside| text(aside, 12., p.muted))),
        )
        .child(text(detail, 12., p.muted).mt(px(3.)))
}
fn field(label: &'static str, input: &Entity<InputState>, p: Palette) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(text(label, 13., p.muted).w(px(64.)))
        .child(
            Input::new(input)
                .min_h(px(32.))
                .max_h(px(32.))
                .flex_1()
                .rounded(px(6.))
                .text_size(px(13.))
                .bg(p.bg),
        )
}
fn download_aside(status: Status, size_mb: u64) -> String {
    match status {
        Status::Installed => "Installed".into(),
        Status::Downloading(percent) => format!("{percent}%"),
        Status::Failed => "Failed".into(),
        Status::NotInstalled => format!("{size_mb} MB"),
    }
}
/// The chosen model's status while on-device speech has nothing installed to dictate with.
fn speech_download(s: &AppState) -> Option<Status> {
    (s.config.transcription == SpeechSource::Local && !s.models.contains(&Status::Installed))
        .then(|| s.models[s.config.local_model.index()])
}
/// The download still running, named for the header so it stays visible on every step.
fn downloading(s: &AppState) -> Option<String> {
    let speech = speech_download(s).and_then(|status| match status {
        Status::Downloading(percent) => Some((s.config.local_model.label(), percent)),
        _ => None,
    });
    let cleanup = match s.cleanup_model {
        Status::Downloading(percent) if s.config.local_editing() => {
            Some((CleanupModel::LABEL, percent))
        }
        _ => None,
    };
    speech
        .or(cleanup)
        .map(|(name, percent)| format!("{name} · {percent}%"))
}
/// Marks each spoken word that did not survive into the typed text, by longest common subsequence.
fn struck(raw: &str, cleaned: &str) -> Vec<(String, bool)> {
    let norm = |word: &str| {
        word.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let spoken: Vec<&str> = raw.split_whitespace().collect();
    let a: Vec<String> = spoken.iter().map(|w| norm(w)).collect();
    let b: Vec<String> = cleaned.split_whitespace().map(norm).collect();
    let mut table = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i][j] = if a[i] == b[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut words = Vec::with_capacity(a.len());
    while i < a.len() {
        if j < b.len() && a[i] == b[j] {
            words.push((spoken[i].to_owned(), false));
            i += 1;
            j += 1;
        } else if j < b.len() && table[i][j + 1] > table[i + 1][j] {
            j += 1;
        } else {
            words.push((spoken[i].to_owned(), true));
            i += 1;
        }
    }
    words
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
        .gap(px(4.))
        .children((0..count).map(|i| {
            div()
                .w(px(5.))
                .h(px((height * SHAPE[i] * level.clamp(0., 1.)).max(5.)))
                .rounded(px(3.))
                .bg(p.blue)
        }))
}
impl MainWindow {
    pub fn init_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.read(cx).config.onboarded {
            return;
        }
        if !self.state.read(cx).is_demo {
            let source = if typeaside::local::AVAILABLE {
                Source::Local
            } else {
                Source::OwnKey
            };
            self.state.update(cx, |s, _| {
                s.config.transcription = source.into();
                s.config.cleanup.source = source;
            });
        } else if self.state.read(cx).onboarding_step == Step::Practice
            && speech_download(self.state.read(cx)).is_none()
        {
            if std::env::var("TYPEASIDE_DEMO_PRACTICE").as_deref() == Ok("listening") {
                self.state.update(cx, |s, _| {
                    s.phase = Phase::Listening(typeaside::engine::Mode::Dictate)
                });
            } else {
                let cleaned = "Yes, let's meet at 3. I'll bring the deck.";
                self.state.update(cx, |s, _| {
                    s.onboarding_transcript = Some(cleaned.into());
                    s.onboarding_raw = Some(PRACTICE.into());
                });
                self.onboarding
                    .reply
                    .update(cx, |input, cx| input.set_value(cleaned, window, cx));
            }
        }
        window.focus(&self.focus, cx);
        self.onboarding_enter(window, cx);
    }
    fn onboarding_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.onboarding.frame = Instant::now();
        self.onboarding.slide = Spring::new(0.);
        self.onboarding.slide.target = 1.;
        self.capture = None;
        self.onboarding.pending_key = None;
        let step = self.state.read(cx).onboarding_step;
        if step == Step::Practice {
            self.start_downloads(cx);
            self.onboarding
                .reply
                .update(cx, |input, cx| input.focus(window, cx));
        } else {
            window.focus(&self.focus, cx);
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
        cx.notify();
    }
    /// What the engine step still needs before Continue: a speech key, a Wispr account, or an editing key.
    fn engine_ready(&self, cx: &Context<Self>) -> bool {
        let s = self.state.read(cx);
        let typed = |input: &Entity<InputState>| !input.read(cx).value().is_empty();
        let speech = match s.config.transcription {
            SpeechSource::Local => true,
            SpeechSource::OwnKey => typed(&self.settings.key) || s.key_hint == "Saved in keyring",
            SpeechSource::Wispr => s.wispr_account.is_some(),
        };
        speech
            && (!self.editing_key_needed(cx)
                || typed(&self.settings.cleanup_key)
                || s.editing_key_hint == "Saved in keyring")
    }
    /// Editing through a key on a different host than speech asks for its own key.
    fn editing_key_needed(&self, cx: &Context<Self>) -> bool {
        let c = &self.state.read(cx).config;
        let host = typeaside::credentials::endpoint_host;
        c.transcription != SpeechSource::Wispr
            && c.cleanup.enabled
            && c.cleanup.source == Source::OwnKey
            && (c.transcription != SpeechSource::OwnKey
                || host(&c.cleanup.endpoint.base_url).ok() != host(&c.stt.base_url).ok())
    }
    fn onboarding_ready(&self, cx: &Context<Self>) -> bool {
        let s = self.state.read(cx);
        match s.onboarding_step {
            Step::Engine => self.engine_ready(cx),
            Step::Permissions => self.permission_status(cx).into_iter().all(|s| s),
            Step::Microphone => true,
            Step::Practice => s.onboarding_transcript.is_some(),
        }
    }
    fn permission_status(&self, cx: &Context<Self>) -> [bool; 2] {
        if self.state.read(cx).is_demo {
            return [true, false];
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            super::permissions::statuses()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            [true; 2]
        }
    }
    pub fn onboarding_next(&mut self, skip: bool, w: &mut Window, cx: &mut Context<Self>) {
        if !skip && !self.onboarding_ready(cx) {
            return;
        }
        let step = self.state.read(cx).onboarding_step;
        if step == Step::Engine {
            let s = self.state.read(cx);
            let unsaved = |input: &Entity<InputState>| !input.read(cx).value().is_empty();
            let editing =
                if s.config.transcription == SpeechSource::OwnKey && unsaved(&self.settings.key) {
                    Some(false)
                } else if self.editing_key_needed(cx) && unsaved(&self.settings.cleanup_key) {
                    Some(true)
                } else {
                    None
                };
            if let Some(editing) = editing {
                self.onboarding.pending_key = Some(editing);
                self.save_key(editing, w, cx);
                return;
            }
            self.start_downloads(cx);
            self.state.update(cx, |s, _| s.persist());
        }
        let index = self.onboarding.steps.iter().position(|s| *s == step);
        match index.and_then(|i| self.onboarding.steps.get(i + 1).copied()) {
            Some(next) => {
                self.state.update(cx, |s, _| s.onboarding_step = next);
                self.onboarding.direction = 1.;
                self.onboarding_enter(w, cx);
            }
            None => self.finish_onboarding(w, cx),
        }
    }
    /// Starts the on-device downloads chosen so far; they continue through the later steps.
    fn start_downloads(&mut self, cx: &mut Context<Self>) {
        let s = self.state.read(cx);
        if s.is_demo {
            return;
        }
        let missing = |status| matches!(status, Status::NotInstalled | Status::Failed);
        let speech = speech_download(s)
            .filter(|status| missing(*status))
            .map(|_| DownloadTarget::Speech(s.config.local_model));
        let cleanup = (s.config.local_editing() && missing(s.cleanup_model))
            .then_some(DownloadTarget::Cleanup);
        for target in speech.into_iter().chain(cleanup) {
            self.download_model(target, cx);
        }
    }
    pub fn onboarding_back(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let step = self.state.read(cx).onboarding_step;
        if let Some(index) = self.onboarding.steps.iter().position(|s| *s == step)
            && index > 0
        {
            let previous = self.onboarding.steps[index - 1];
            self.state.update(cx, |s, _| s.onboarding_step = previous);
            self.onboarding.direction = -1.;
            self.onboarding_enter(w, cx);
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
        w.focus(&self.focus, cx);
        self.set_page(Page::Home, w, cx);
    }
    fn practice_again(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.onboarding_transcript = None;
            s.onboarding_raw = None;
            cx.notify();
        });
        self.onboarding.reply.update(cx, |input, cx| {
            input.set_value("", w, cx);
            input.focus(w, cx);
        });
    }
    fn engine_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let enabled = self.state.read(cx).config.cleanup.enabled;
        let editing_toggle = self
            .toggle("onboarding-editing", enabled, Toggle::Editing, cx)
            .into_any_element();
        let s = self.state.read(cx);
        let c = &s.config;
        let source = c.transcription;
        let model = c.local_model;
        let set = |source: SpeechSource| {
            cx.listener(move |this, _, _, cx| {
                this.state.update(cx, |s, cx| {
                    s.config.transcription = source;
                    match source {
                        SpeechSource::Local => s.config.cleanup.source = Source::Local,
                        SpeechSource::OwnKey => s.config.cleanup.source = Source::OwnKey,
                        SpeechSource::Wispr => {}
                    }
                    cx.notify();
                });
            })
        };
        let cloud = source != SpeechSource::Local;
        let cleanup_aside = (c.local_editing() && c.cleanup.enabled)
            .then(|| download_aside(s.cleanup_model, CleanupModel::size_mb()));
        let destination = match source {
            SpeechSource::Local => "Nothing leaves this computer.".to_string(),
            SpeechSource::Wispr => "Audio goes to Wispr Flow.".into(),
            SpeechSource::OwnKey => typeaside::credentials::endpoint_host(&c.stt.base_url)
                .map(|host| format!("Audio goes to {host}."))
                .unwrap_or_else(|_| "Audio goes to your endpoint.".into()),
        };
        let example = if c.cleanup.enabled {
            format!("\u{201c}{EXAMPLE_RAW}\u{201d} becomes \u{201c}{EXAMPLE_CLEAN}\u{201d}")
        } else {
            "Types exactly what you said.".into()
        };
        div()
            .flex()
            .flex_col()
            .child(title("Where should your voice go?", p))
            .child(sub(destination, p))
            .when(typeaside::local::AVAILABLE, |d| {
                d.child(
                    tile(
                        "engine-local",
                        source == SpeechSource::Local,
                        "On this computer",
                        format!("Private and offline. {}.", model.description()),
                        Some(download_aside(s.models[model.index()], model.size_mb())),
                        p,
                    )
                    .mt(px(20.))
                    .on_click(set(SpeechSource::Local)),
                )
            })
            .child(
                tile(
                    "engine-cloud",
                    cloud,
                    "Cloud",
                    "Your API key, like Groq or OpenAI, or a Wispr Flow account.",
                    None,
                    p,
                )
                .on_click(set(SpeechSource::OwnKey)),
            )
            .when(cloud, |d| {
                d.child(
                    div().mt(px(14.)).flex().gap(px(16.)).children(
                        [
                            ("cloud-key", SpeechSource::OwnKey, "API key"),
                            ("cloud-wispr", SpeechSource::Wispr, "Wispr Flow"),
                        ]
                        .map(|(id, option, label)| {
                            text(label, 13., if source == option { p.fg } else { p.muted })
                                .when(source == option, |d| d.font_weight(FontWeight::SEMIBOLD))
                                .pb(px(3.))
                                .border_b_2()
                                .border_color(if source == option {
                                    p.blue
                                } else {
                                    gpui::transparent_black()
                                })
                                .id(id)
                                .cursor_pointer()
                                .on_click(set(option))
                        }),
                    ),
                )
                .child(match source {
                    SpeechSource::Wispr => div()
                        .mt(px(12.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(text(
                            s.save_error.clone().unwrap_or_else(|| {
                                if s.wispr_login.is_some() {
                                    "Finish signing in through your browser.".into()
                                } else {
                                    s.wispr_account.clone().unwrap_or_default()
                                }
                            }),
                            12.,
                            if s.save_error.is_some() {
                                p.red
                            } else {
                                p.muted
                            },
                        ))
                        .child(
                            button(
                                if s.wispr_login.is_some() {
                                    "Cancel"
                                } else {
                                    "Sign in"
                                },
                                false,
                                p,
                            )
                            .id("onboarding-wispr-login")
                            .on_click(cx.listener(|this, _, _, cx| this.connect_wispr(cx))),
                        ),
                    _ => div()
                        .mt(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(field("Endpoint", &self.settings.endpoint, p))
                        .child(field("API key", &self.settings.key, p)),
                })
            })
            .child(
                div()
                    .mt(px(18.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(editing_toggle)
                    .child(text("Clean up fillers and corrections", 13., p.fg).flex_1())
                    .children(cleanup_aside.map(|aside| text(aside, 12., p.muted))),
            )
            .child(text(example, 12., p.muted).mt(px(6.)).pl(px(44.)))
            .when(self.editing_key_needed(cx), |d| {
                d.child(
                    div()
                        .mt(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(field("Endpoint", &self.settings.cleanup_endpoint, p))
                        .child(field("API key", &self.settings.cleanup_key, p)),
                )
            })
            .into_any_element()
    }
    fn permissions_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let statuses = self.permission_status(cx);
        let rows = [
            ("Microphone", "To hear you"),
            (
                if cfg!(target_os = "linux") {
                    "Keyboard"
                } else {
                    "Accessibility"
                },
                "To catch your key and type for you",
            ),
        ];
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let help = super::permissions::help(self.state.read(cx));
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let help: Option<&'static str> = None;
        div()
            .flex()
            .flex_col()
            .child(title("Let TypeAside hear and type.", p))
            .child(sub("It only listens while you hold your key.", p))
            .child(
                div()
                    .mt(px(22.))
                    .border_b_1()
                    .border_color(p.divider)
                    .children(
                        rows.into_iter()
                            .enumerate()
                            .filter(|(i, _)| !cfg!(target_os = "linux") || *i == 1)
                            .map(|(i, (name, purpose))| {
                                div()
                                    .h(px(58.))
                                    .border_t_1()
                                    .border_color(p.divider)
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(
                                                text(name, 14., p.fg)
                                                    .font_weight(FontWeight::SEMIBOLD),
                                            )
                                            .child(text(purpose, 12., p.muted).mt(px(2.))),
                                    )
                                    .child(if statuses[i] {
                                        div()
                                            .flex()
                                            .gap(px(6.))
                                            .items_center()
                                            .child(icon("icons/check.svg", 16., p.green))
                                            .child(text("Allowed", 13., p.green))
                                            .into_any_element()
                                    } else {
                                        button(
                                            if cfg!(target_os = "macos") && i == 1 {
                                                "Open Settings"
                                            } else {
                                                "Allow"
                                            },
                                            true,
                                            p,
                                        )
                                        .id(("permission", i))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            #[cfg(any(target_os = "macos", target_os = "linux"))]
                                            if !this.state.read(cx).is_demo {
                                                super::permissions::allow(i, &this.state, cx);
                                            }
                                            #[cfg(not(any(
                                                target_os = "macos",
                                                target_os = "linux"
                                            )))]
                                            let _ = (this, cx);
                                        }))
                                        .into_any_element()
                                    })
                            }),
                    ),
            )
            .child(sub(
                if cfg!(target_os = "linux") {
                    "Your system asks for your password once."
                } else {
                    "Turn TypeAside on in System Settings, then come back. This updates on its own."
                },
                p,
            ))
            .children(help.map(|help| sub(help, p)))
            .into_any_element()
    }
    fn microphone_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let level = {
            let s = self.state.read(cx);
            if s.is_demo { 1. } else { s.level }
        };
        div()
            .flex()
            .flex_col()
            .child(title("Say something.", p))
            .child(sub("Do the bars move when you talk?", p))
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .justify_center()
                    .child(bars(level, 88., 24, p)),
            )
            .child(text("Microphone", 12., p.muted).mt(px(36.)))
            .child(div().mt(px(8.)).w_full().child(self.microphones(cx)))
            .into_any_element()
    }
    fn practice_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let s = self.state.read(cx);
        let listening = matches!(s.phase, Phase::Listening(_));
        let result = s
            .onboarding_raw
            .clone()
            .zip(s.onboarding_transcript.clone());
        let reveal = self.onboarding.result.value.clamp(0., 1.);
        let landed = !self.onboarding.reply.read(cx).value().is_empty();
        let hotkey = s.config.hotkey.clone();
        let commands = [
            (Some(hotkey.clone()), "Hold to dictate"),
            (
                s.config.command_hotkey.clone(),
                "Edit selected text, or ask",
            ),
            (s.config.note_hotkey.clone(), "Quick note"),
        ];
        let reply = match speech_download(s) {
            Some(status) => {
                let model = s.config.local_model;
                let percent = match status {
                    Status::Downloading(percent) => percent,
                    _ => 0,
                };
                div()
                    .w_full()
                    .child(text(format!("Getting {} ready", model.label()), 14., p.fg))
                    .child(
                        div()
                            .w_full()
                            .h(px(4.))
                            .mt(px(14.))
                            .rounded(px(2.))
                            .bg(p.fg.opacity(0.14))
                            .child(
                                div()
                                    .h_full()
                                    .w(gpui::relative(f32::from(percent) / 100.))
                                    .rounded(px(2.))
                                    .bg(p.blue),
                            ),
                    )
                    .child(match status {
                        Status::Failed => div()
                            .mt(px(10.))
                            .flex()
                            .gap(px(8.))
                            .child(text("Download failed.", 12., p.red))
                            .child(
                                text("Retry", 12., p.blue)
                                    .id("retry-download")
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.download_model(DownloadTarget::Speech(model), cx)
                                    })),
                            ),
                        _ => text(
                            format!(
                                "{percent}% of {} MB. Practice starts when it finishes.",
                                model.size_mb()
                            ),
                            12.,
                            p.muted,
                        )
                        .mt(px(10.)),
                    })
                    .into_any_element()
            }
            None => div()
                .w_full()
                .child(
                    div()
                        .flex()
                        .gap(px(10.))
                        .child(
                            div()
                                .size(px(26.))
                                .rounded_full()
                                .bg(p.tint)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.))
                                .text_color(p.blue)
                                .child("S"),
                        )
                        .child(
                            div()
                                .child(text("Sam", 13., p.fg).font_weight(FontWeight::SEMIBOLD))
                                .child(text("Still on for today?", 13., p.fg).mt(px(2.))),
                        ),
                )
                .child(
                    Input::new(&self.onboarding.reply)
                        .mt(px(16.))
                        .min_h(px(40.))
                        .w_full()
                        .rounded(px(8.))
                        .text_size(px(14.))
                        .bg(p.bg)
                        .border_color(if listening { p.blue } else { p.border }),
                )
                .child(
                    div()
                        .h(px(18.))
                        .mt(px(8.))
                        .children(result.as_ref().map(|_| {
                            if landed {
                                div()
                                    .flex()
                                    .gap(px(6.))
                                    .items_center()
                                    .child(icon("icons/check.svg", 14., p.green))
                                    .child(text("Typed by TypeAside", 12., p.green))
                            } else {
                                div().child(text(
                                    "It didn't land here, so TypeAside copied it.",
                                    12.,
                                    p.muted,
                                ))
                            }
                        })),
                )
                .into_any_element(),
        };
        let change_key = text(
            if self.capture.is_some() {
                "Press keys…"
            } else {
                "Change key"
            },
            13.,
            p.blue,
        )
        .id("onboarding-shortcut")
        .cursor_pointer()
        .on_click(cx.listener(|this, _, w, cx| {
            this.capture = Some(Shortcut::Dictate);
            typeaside::hotkeys::set_shortcut_recording(true);
            this.shortcut_down = None;
            this.shortcut_tap = None;
            this.shortcut_task = None;
            this.shortcut_key = None;
            w.focus(&this.focus, cx);
            cx.notify();
        }));
        match result {
            None => div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .child(
                            div()
                                .h(px(56.))
                                .px(px(22.))
                                .rounded(px(12.))
                                .border_1()
                                .border_color(if listening { p.blue } else { p.kbd })
                                .bg(if listening { p.tint } else { p.selected })
                                .shadow(vec![gpui::BoxShadow {
                                    color: gpui::Hsla {
                                        l: p.kbd.l * 0.8,
                                        ..p.kbd
                                    },
                                    offset: gpui::point(
                                        px(0.),
                                        px(if listening { 1. } else { 3. }),
                                    ),
                                    blur_radius: px(0.),
                                    spread_radius: px(0.),
                                    inset: false,
                                }])
                                .relative()
                                .top(px(if listening { 2. } else { 0. }))
                                .flex()
                                .items_center()
                                .text_size(px(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if listening { p.blue } else { p.fg })
                                .child(key_label(&hotkey)),
                        )
                        .child(change_key),
                )
                .child(
                    title(
                        if s.config.mode == RecordingMode::Toggle {
                            "Press it, read, press again:"
                        } else {
                            "Hold it and read:"
                        },
                        p,
                    )
                    .mt(px(22.)),
                )
                .child(
                    text(
                        format!("\u{201c}{PRACTICE}\u{201d}"),
                        17.,
                        p.fg.opacity(0.85),
                    )
                    .line_height(px(26.))
                    .mt(px(10.)),
                )
                .child(div().mt(px(26.)).child(reply))
                .into_any_element(),
            Some((raw, cleaned)) => div()
                .flex()
                .flex_col()
                .opacity(reveal)
                .child(title("That's it.", p))
                .child(
                    div()
                        .mt(px(10.))
                        .flex()
                        .flex_wrap()
                        .gap_x(px(4.))
                        .text_size(px(14.))
                        .line_height(px(21.))
                        .children(struck(&raw, &cleaned).into_iter().map(|(word, removed)| {
                            div()
                                .text_color(if removed { p.red } else { p.muted })
                                .when(removed, |d| d.line_through())
                                .child(word)
                        })),
                )
                .child(div().mt(px(20.)).child(reply))
                .child(
                    div()
                        .mt(px(20.))
                        .pt(px(14.))
                        .border_t_1()
                        .border_color(p.divider)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .children(commands.into_iter().filter_map(|(key, label)| {
                            key.filter(|k| !k.is_empty()).map(|key| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(div().w(px(96.)).child(kbd(&key, p)))
                                    .child(text(label, 13., p.muted))
                            })
                        })),
                )
                .into_any_element(),
        }
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
        let has_result = s.onboarding_transcript.is_some();
        let progress = downloading(s);
        if has_result && !self.onboarding.had_result {
            self.onboarding.result = Spring::new(0.);
            self.onboarding.result.target = 1.;
        }
        self.onboarding.had_result = has_result;
        let result_moving = self.onboarding.result.step(dt, 260., 28.);
        if moving || result_moving {
            w.request_animation_frame();
        }
        let body = match step {
            Step::Engine => self.engine_step(cx),
            Step::Permissions => self.permissions_step(cx),
            Step::Microphone => self.microphone_step(cx),
            Step::Practice => self.practice_step(cx),
        };
        let ready = self.onboarding_ready(cx);
        let index = self
            .onboarding
            .steps
            .iter()
            .position(|s| *s == step)
            .unwrap_or(0);
        let count = self.onboarding.steps.len();
        let fraction = if has_result {
            1.
        } else {
            (index as f32 + 0.5) / count as f32
        };
        let bloom = if p.mode == gpui_kit::component::ThemeMode::Dark {
            0.5
        } else {
            0.95
        };
        let slide = self.onboarding.slide.value;
        let offset = px(20. * self.onboarding.direction * (1. - slide));
        let actions = div()
            .mt_auto()
            .h(px(CONTROL))
            .flex()
            .items_center()
            .gap(px(10.))
            .when(index > 0 && !has_result, |d| {
                d.child(
                    link("Back", p)
                        .id("onboarding-back")
                        .on_click(cx.listener(|this, _, w, cx| this.onboarding_back(w, cx))),
                )
            })
            .when(has_result, |d| {
                d.child(
                    link("Try again", p)
                        .id("practice-again")
                        .on_click(cx.listener(|this, _, w, cx| this.practice_again(w, cx))),
                )
            })
            .child(div().flex_1())
            .when(
                matches!(step, Step::Permissions | Step::Practice) && !has_result,
                |d| {
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
                        .on_click(cx.listener(|this, _, w, cx| this.onboarding_next(true, w, cx))),
                    )
                },
            )
            .when(step != Step::Practice || has_result, |d| {
                d.child(
                    button(
                        match step {
                            Step::Microphone => "Yes, continue",
                            Step::Practice => "Start using TypeAside",
                            _ => "Continue",
                        },
                        true,
                        p,
                    )
                    .px(px(16.))
                    .opacity(if ready { 1. } else { 0.4 })
                    .id("onboarding-continue")
                    .when(ready, |d| {
                        d.on_click(cx.listener(|this, _, w, cx| this.onboarding_next(false, w, cx)))
                    }),
                )
            });
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(gpui::linear_gradient(
                180.,
                gpui::linear_color_stop(p.tint, 0.),
                gpui::linear_color_stop(p.tint.opacity(0.), 0.6),
            ))
            .child(
                img("onboarding/bloom.png")
                    .absolute()
                    .right(px(-130.))
                    .bottom_0()
                    .size(px(360.))
                    .opacity(bloom),
            )
            .child(
                img("onboarding/sprig.png")
                    .absolute()
                    .left(px(-24.))
                    .bottom_0()
                    .w(px(180.))
                    .h(px(210.))
                    .opacity(bloom),
            )
            .child(
                div()
                    .relative()
                    .h(px(46.))
                    .flex_shrink_0()
                    .px(px(28.))
                    .flex()
                    .items_center()
                    .gap(px(18.))
                    .text_size(px(12.))
                    .child(
                        text(
                            "Set up",
                            12.,
                            if step == Step::Practice {
                                p.muted
                            } else {
                                p.fg
                            },
                        )
                        .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text(
                            "Try it",
                            12.,
                            if step == Step::Practice {
                                p.fg
                            } else {
                                p.faint
                            },
                        )
                        .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(div().flex_1())
                    .child(text(
                        progress.unwrap_or_else(|| {
                            if index == 0 {
                                "About a minute".into()
                            } else {
                                String::new()
                            }
                        }),
                        12.,
                        p.muted,
                    ))
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .h(px(1.))
                            .bg(p.divider),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .h(px(2.))
                            .w(gpui::relative(fraction))
                            .bg(p.blue),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .justify_center()
                    .relative()
                    .left(offset)
                    .opacity(slide.clamp(0., 1.))
                    .child(
                        div()
                            .w(px(COLUMN))
                            .h_full()
                            .pt(px(48.))
                            .pb(px(28.))
                            .flex()
                            .flex_col()
                            .child(body)
                            .child(actions),
                    ),
            )
    }
}
