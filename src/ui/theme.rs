use gpui::{App, Div, FontWeight, Styled, div, prelude::*, px, rgb, rgba};
use gpui_kit::component::{Theme, ThemeMode};

pub const MONO: &str = "DejaVu Sans Mono";
pub const BLACK: u32 = 0x000000;
pub const WHITE: u32 = 0xffffff;
pub const DIVIDER: u32 = 0x1a1a1a;
pub const ROW: u32 = 0x111111;
pub const BORDER: u32 = 0x262626;
pub const HOVER: u32 = 0x0b0b0b;
pub const GREEN: u32 = 0x30d158;
pub const RED: u32 = 0xff453a;
pub const MUTED: u32 = 0xffffff8c;
pub const SIDEBAR: f32 = 184.;
pub const INSET: f32 = 24.;
pub const CONTROL: f32 = 30.;
pub const RADIUS: f32 = 6.;
pub const LABEL: f32 = 200.;
pub const PILL_HEIGHT: f32 = 36.;
pub const LISTEN_WIDTH: f32 = 120.;
pub const WORK_WIDTH: f32 = 76.;
pub const SHADOW_SPACE: f32 = 24.;
pub const BARS: usize = 13;
pub const BAR_CENTER: f32 = 6.;
pub const BAR_ENVELOPE: f32 = 4.;
pub const PROCESS_START: usize = 3;
pub const PROCESS_END: usize = 9;

pub fn init(cx: &mut App) {
    gpui_kit::init(cx);
    Theme::change(ThemeMode::Dark, None, cx);
    let theme = Theme::global_mut(cx);
    theme.font_family = "Inter".into();
    theme.font_size = px(13.);
    theme.mono_font_size = px(12.);
    theme.mono_font_family = MONO.into();
    theme.radius = px(RADIUS);
    theme.colors.background = rgb(BLACK).into();
    theme.colors.foreground = rgb(WHITE).into();
    theme.colors.input = rgb(BORDER).into();
    theme.colors.border = rgb(BORDER).into();
    theme.colors.ring = rgb(0x555555).into();
    theme.colors.muted_foreground = rgba(MUTED).into();
    theme.colors.popover = rgb(BLACK).into();
    theme.colors.popover_foreground = rgb(WHITE).into();
    theme.colors.accent = rgb(ROW).into();
    theme.colors.accent_foreground = rgb(WHITE).into();
}

pub fn header(title: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(16.))
        .px(px(INSET))
        .pt(px(18.))
        .pb(px(14.))
        .border_b_1()
        .border_color(rgb(DIVIDER))
        .child(
            div()
                .text_size(px(15.))
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

pub fn form_row(label: &'static str, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(16.))
        .py(px(7.))
        .border_b_1()
        .border_color(rgb(ROW))
        .child(div().w(px(LABEL)).flex_shrink_0().child(label))
        .child(div().flex_1().min_w_0().child(control))
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
