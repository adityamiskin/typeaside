use gpui::{
    App, Div, FontWeight, Global, Hsla, Styled, WindowAppearance, div, prelude::*, px, rgb,
};
use gpui_kit::component::{Theme, ThemeMode};
use hermes::config::Theme as Preference;

pub const MONO: &str = "DejaVu Sans Mono";
pub const SIDEBAR: f32 = 184.;
pub const INSET: f32 = 24.;
pub const CONTROL: f32 = 30.;
pub const RADIUS: f32 = 6.;
pub const LABEL: f32 = 200.;
pub const PILL_HEIGHT: f32 = 36.;
pub const LISTEN_WIDTH: f32 = 120.;
pub const COMMAND_WIDTH: f32 = 148.;
pub const WORK_WIDTH: f32 = 76.;
pub const SHADOW_SPACE: f32 = 24.;
pub const BARS: usize = 13;
pub const BAR_CENTER: f32 = 6.;
pub const BAR_ENVELOPE: f32 = 4.;
pub const PROCESS_START: usize = 3;
pub const PROCESS_END: usize = 9;

#[derive(Clone, Copy)]
pub struct Palette {
    pub mode: ThemeMode,
    pub bg: Hsla,
    pub frame: Hsla,
    pub side: Hsla,
    pub fg: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub divider: Hsla,
    pub row: Hsla,
    pub border: Hsla,
    pub hover: Hsla,
    pub selected: Hsla,
    pub kbd: Hsla,
    pub accent: Hsla,
    pub on_accent: Hsla,
    pub green: Hsla,
    pub red: Hsla,
    pub pill: Hsla,
    pub pill_border: Hsla,
    pub shadow: Hsla,
}
impl Global for Palette {}

impl Palette {
    pub fn dark() -> Self {
        let fg: Hsla = rgb(0xf2ece1).into();
        let accent: Hsla = rgb(0xd6b47c).into();
        Self {
            mode: ThemeMode::Dark,
            frame: rgb(0).alpha(0.4).into(),
            bg: rgb(0x13110e).into(),
            side: rgb(0x181511).into(),
            fg,
            muted: fg.opacity(0.56),
            faint: fg.opacity(0.34),
            divider: rgb(0x26221d).into(),
            row: rgb(0x1d1a16).into(),
            border: rgb(0x363027).into(),
            hover: rgb(0x181512).into(),
            selected: rgb(0x221e19).into(),
            kbd: rgb(0x3d362c).into(),
            accent,
            on_accent: rgb(0x1b150b).into(),
            green: rgb(0x8fc095).into(),
            red: rgb(0xe27a66).into(),
            pill: rgb(0x171512).into(),
            pill_border: accent.opacity(0.24),
            shadow: rgb(0).alpha(0.55).into(),
        }
    }
    pub fn light() -> Self {
        let fg: Hsla = rgb(0x1d1a16).into();
        let accent: Hsla = rgb(0x9c7535).into();
        Self {
            mode: ThemeMode::Light,
            frame: rgb(0).alpha(0.4).into(),
            bg: rgb(0xf8f4ec).into(),
            side: rgb(0xf1ebe0).into(),
            fg,
            muted: fg.opacity(0.58),
            faint: fg.opacity(0.36),
            divider: rgb(0xe4dccd).into(),
            row: rgb(0xebe4d7).into(),
            border: rgb(0xd6ccbb).into(),
            hover: rgb(0xf1ebdf).into(),
            selected: rgb(0xe7decd).into(),
            kbd: rgb(0xcfc4b1).into(),
            accent,
            on_accent: rgb(0xfffaf0).into(),
            green: rgb(0x3d7d55).into(),
            red: rgb(0xb4473a).into(),
            pill: rgb(0xfbf8f2).into(),
            pill_border: accent.opacity(0.30),
            shadow: rgb(0x3c2d14).alpha(0.18).into(),
        }
    }
}

pub fn palette(cx: &App) -> Palette {
    *cx.global::<Palette>()
}
pub fn apply(preference: Preference, appearance: WindowAppearance, cx: &mut App) {
    let light = match preference {
        Preference::Light => true,
        Preference::Dark => false,
        Preference::System => matches!(
            appearance,
            WindowAppearance::Light | WindowAppearance::VibrantLight
        ),
    };
    let p = if light {
        Palette::light()
    } else {
        Palette::dark()
    };
    if cx.has_global::<Palette>() && palette(cx).mode == p.mode {
        return;
    }
    Theme::change(p.mode, None, cx);
    Theme::update(cx, |t| {
        t.font_family = "Inter".into();
        t.font_size = px(13.);
        t.mono_font_size = px(12.);
        t.mono_font_family = MONO.into();
        t.radius = px(RADIUS);
        t.colors.background = p.bg;
        t.colors.foreground = p.fg;
        t.colors.input = p.border;
        t.colors.border = p.border;
        t.colors.ring = p.border;
        t.colors.muted_foreground = p.faint;
        t.colors.popover = p.bg;
        t.colors.popover_foreground = p.fg;
        t.colors.accent = p.selected;
        t.colors.accent_foreground = p.fg;
        t.colors.primary = p.accent;
        t.colors.primary_foreground = p.on_accent;
        t.colors.secondary = p.row;
        t.colors.secondary_foreground = p.fg;
        t.colors.caret = p.fg;
        t.colors.muted = p.row;
        t.colors.list = p.bg;
        t.colors.list_active = p.selected;
        t.colors.list_active_border = p.border;
        t.colors.list_hover = p.hover;
        t.colors.link = p.fg;
        t.colors.link_hover = p.fg;
        t.colors.scrollbar = p.bg;
        t.colors.scrollbar_thumb = p.border;
        t.colors.danger = p.red;
        t.colors.danger_foreground = p.on_accent;
    });
    cx.set_global(p);
    cx.refresh_windows();
}
pub fn init(preference: Preference, cx: &mut App) {
    gpui_kit::init(cx);
    apply(preference, cx.window_appearance(), cx);
}
pub fn header(title: &'static str, p: Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(16.))
        .px(px(INSET))
        .pt(px(18.))
        .pb(px(14.))
        .border_b_1()
        .border_color(p.divider)
        .child(
            div()
                .text_size(px(15.))
                .line_height(px(21.75))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
}
pub fn section(title: &'static str) -> Div {
    div()
        .pt(px(18.))
        .pb(px(6.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(title)
}
pub fn form_row(label: &'static str, control: impl IntoElement, p: Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(16.))
        .min_h(px(38.))
        .border_b_1()
        .border_color(p.row)
        .child(div().w(px(LABEL)).flex_shrink_0().child(label))
        .child(div().flex_1().min_w_0().child(control))
}
pub fn button(label: &'static str, primary: bool, p: Palette) -> Div {
    div()
        .h(px(CONTROL))
        .px(px(12.))
        .flex()
        .items_center()
        .rounded(px(RADIUS))
        .border_1()
        .border_color(if primary { p.accent } else { p.border })
        .bg(if primary { p.accent } else { p.bg })
        .text_color(if primary { p.on_accent } else { p.fg })
        .font_weight(if primary {
            FontWeight::MEDIUM
        } else {
            FontWeight::NORMAL
        })
        .cursor_pointer()
        .child(label)
}
pub fn kbd(value: String, p: Palette) -> Div {
    div()
        .font_family(MONO)
        .text_size(px(11.))
        .line_height(px(13.))
        .border_1()
        .border_color(p.kbd)
        .rounded(px(4.))
        .px(px(5.))
        .py(px(1.))
        .child(value)
}
pub fn key_label(value: &str) -> String {
    value
        .split('+')
        .map(|key| match key {
            "RIGHTCTRL" => "Right Ctrl",
            "RIGHTALT" => "Right Alt",
            "LEFTALT" => "Alt",
            "LEFTSHIFT" => "Shift",
            "LEFTCTRL" => "Ctrl",
            "ESCAPE" => "Escape",
            other => other,
        })
        .collect::<Vec<_>>()
        .join("+")
}
pub const SURFACE_WIDTH: f32 = 640.;
pub const SURFACE_HEIGHT: f32 = 96.;
pub const ENTER_FADE: f32 = 0.12;
pub const OPEN_DELAY: f32 = 0.075;
pub const STAGGER: f32 = 0.015;
pub const EXIT_TIME: f32 = 0.16;
pub const CHECK_TIME: f32 = 0.22;
pub const SUCCESS_HOLD: f32 = 0.35;
pub const CANCEL_TIME: f32 = 0.12;
pub const ERROR_HOLD: f32 = 1.8;
pub const ERROR_TEXT_TIME: f32 = 0.15;
pub const SHAKE_TIME: f32 = 0.36;
pub const WAVE_PERIOD: f32 = 0.9;
pub const WAVE_STEP: f32 = 0.08;
pub const QUIET_PERIOD: f32 = 1.2;
pub const PILL_STIFFNESS: f32 = 420.;
pub const PILL_DAMPING: f32 = 32.;
pub const BAR_STIFFNESS: f32 = 600.;
pub const BAR_DAMPING: f32 = 30.;
