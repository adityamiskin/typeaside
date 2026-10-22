use gpui::{
    AnyElement, App, Div, FontWeight, Global, Hsla, SharedString, Stateful, Styled,
    WindowAppearance, div, prelude::*, px, rgb,
};
use gpui_kit::component::{Theme, ThemeMode};
use typeaside::config::Theme as Preference;

pub const MONO: &str = "DejaVu Sans Mono";
pub const SIDEBAR: f32 = 240.;
pub const INSET: f32 = 24.;
pub const FORM_WIDTH: f32 = 720.;
pub const LIST_WIDTH: f32 = 880.;
pub const CONTROL: f32 = 30.;
pub const RADIUS: f32 = 6.;
pub const PILL_HEIGHT: f32 = 36.;
pub const LISTEN_WIDTH: f32 = 100.;
pub const COMMAND_WIDTH: f32 = 124.;
pub const WORK_WIDTH: f32 = 76.;
pub const SHADOW_SPACE: f32 = 24.;
pub const BARS: usize = 13;
pub const BAR_CENTER: f32 = 6.;
pub const BAR_ENVELOPE: f32 = 5.;
pub const RIPPLE: usize = BARS / 2 + 1;
pub const VOICE_GATE: f32 = 0.04;
pub const VOICE_RANGE: f32 = 0.3;
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
    pub panel: Hsla,
    pub border: Hsla,
    pub hover: Hsla,
    pub selected: Hsla,
    pub kbd: Hsla,
    pub accent: Hsla,
    pub tint: Hsla,
    pub blue: Hsla,
    pub lilac: Hsla,
    pub on_accent: Hsla,
    pub green: Hsla,
    pub red: Hsla,
    pub pill: Hsla,
    pub pill_border: Hsla,
    pub shadow: Hsla,
}
impl Global for Palette {}

impl Palette {
    pub fn note_tint(self, color: typeaside::notes::NoteColor) -> Hsla {
        use typeaside::notes::NoteColor;
        let dark = self.mode == ThemeMode::Dark;
        match color {
            NoteColor::None => self.bg,
            NoteColor::Sky => rgb(if dark { 0x1c2533 } else { 0xe6effa }).into(),
            NoteColor::Lilac => rgb(if dark { 0x282133 } else { 0xefe7f5 }).into(),
            NoteColor::Sage => rgb(if dark { 0x1c2a22 } else { 0xe5f1e8 }).into(),
            NoteColor::Sand => rgb(if dark { 0x2d281d } else { 0xf6efdf }).into(),
            NoteColor::Rose => rgb(if dark { 0x2e1f22 } else { 0xf8e6e6 }).into(),
        }
    }

    pub fn note_swatch(self, color: typeaside::notes::NoteColor) -> Hsla {
        use typeaside::notes::NoteColor;
        if self.mode != ThemeMode::Dark {
            return self.note_tint(color);
        }
        match color {
            NoteColor::None => self.bg,
            NoteColor::Sky => rgb(0x5f86b3).into(),
            NoteColor::Lilac => rgb(0x8a6aa3).into(),
            NoteColor::Sage => rgb(0x5f9472).into(),
            NoteColor::Sand => rgb(0xa88d55).into(),
            NoteColor::Rose => rgb(0xa86369).into(),
        }
    }

    pub fn dark() -> Self {
        let fg: Hsla = rgb(0xf2f2f1).into();
        Self {
            mode: ThemeMode::Dark,
            frame: rgb(0).alpha(0.4).into(),
            bg: rgb(0x131416).into(),
            side: rgb(0x18191b).into(),
            fg,
            muted: fg.opacity(0.58),
            faint: fg.opacity(0.36),
            divider: rgb(0x242528).into(),
            row: rgb(0x1c1d20).into(),
            panel: rgb(0x18191b).into(),
            border: rgb(0x2f3034).into(),
            hover: rgb(0x1b1c1f).into(),
            selected: rgb(0x26272b).into(),
            kbd: rgb(0x3b3d42).into(),
            accent: fg,
            tint: rgb(0x1c2533).into(),
            blue: rgb(0x8ab8e8).into(),
            lilac: rgb(0xc6a4d6).into(),
            on_accent: rgb(0x16171a).into(),
            green: rgb(0x8fc095).into(),
            red: rgb(0xe27a66).into(),
            pill: rgb(0x18191b).into(),
            pill_border: fg.opacity(0.14),
            shadow: rgb(0).alpha(0.55).into(),
        }
    }
    pub fn light() -> Self {
        let fg: Hsla = rgb(0x16171a).into();
        Self {
            mode: ThemeMode::Light,
            frame: rgb(0).alpha(0.4).into(),
            bg: rgb(0xf9f9f8).into(),
            side: rgb(0xf3f3f1).into(),
            fg,
            muted: rgb(0x6b6e76).into(),
            faint: rgb(0xa3a6ad).into(),
            divider: rgb(0xe9e9e7).into(),
            row: rgb(0xf1f1ef).into(),
            panel: rgb(0xffffff).into(),
            border: rgb(0xe2e2df).into(),
            hover: rgb(0xefefed).into(),
            selected: rgb(0xffffff).into(),
            kbd: rgb(0xd4d5d8).into(),
            accent: fg,
            tint: rgb(0xe6effa).into(),
            blue: rgb(0x4a84c4).into(),
            lilac: rgb(0x9a6fb3).into(),
            on_accent: rgb(0xffffff).into(),
            green: rgb(0x3d7d55).into(),
            red: rgb(0xb4473a).into(),
            pill: rgb(0xffffff).into(),
            pill_border: fg.opacity(0.10),
            shadow: rgb(0x1e325a).alpha(0.2).into(),
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
        t.colors.ring = p.blue;
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
        t.colors.list_active = p.tint;
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
/// Pane-wide header rule and scroll area around a centered column, so wide windows
/// center content without cutting rules or the scroll area short.
pub fn page(width: f32, header: Div, body: Stateful<Div>, p: Palette) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .justify_center()
                .px(px(INSET))
                .border_b_1()
                .border_color(p.divider)
                .child(header.w_full().max_w(px(width))),
        )
        .child(
            body.flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .items_center(),
        )
}
pub fn column(width: f32) -> Div {
    div().w_full().max_w(px(width + 2. * INSET)).px(px(INSET))
}
pub fn header(title: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(16.))
        .pt(px(18.))
        .pb(px(14.))
        .child(
            div()
                .text_size(px(15.))
                .line_height(px(21.75))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
}
/// A titled, bordered group of settings rows separated by hairlines.
pub fn panel(rows: impl IntoIterator<Item = Setting>, p: Palette) -> Div {
    div()
        .mt(px(10.))
        .border_1()
        .border_color(p.border)
        .rounded(px(8.))
        .bg(p.panel)
        .children(rows.into_iter().enumerate().map(|(i, row)| {
            div()
                .when(i > 0, |cell| cell.border_t_1().border_color(p.divider))
                .child(row)
        }))
}
/// A settings row: label and optional description on the left, its children as controls on the right.
#[derive(IntoElement)]
pub struct Setting {
    label: SharedString,
    description: Option<AnyElement>,
    controls: Vec<AnyElement>,
}
impl Setting {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            controls: Vec::new(),
        }
    }
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self
    }
}
impl ParentElement for Setting {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.controls.extend(elements);
    }
}
impl RenderOnce for Setting {
    fn render(self, _: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        div()
            .flex()
            .items_center()
            .gap(px(16.))
            .min_h(px(44.))
            .px(px(14.))
            .py(px(7.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .whitespace_nowrap()
                            .child(self.label),
                    )
                    .children(self.description.map(|description| {
                        div()
                            .text_size(px(12.))
                            .line_height(px(17.4))
                            .text_color(p.muted)
                            .child(description)
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .flex_shrink_0()
                    .children(self.controls),
            )
    }
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
pub fn kbd(value: &str, p: Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .children(key_names(value).map(|key| {
            div()
                .text_size(px(11.))
                .line_height(px(13.))
                .border_1()
                .border_color(p.kbd)
                .rounded(px(4.))
                .px(px(5.))
                .py(px(1.))
                .child(key)
        }))
}
pub fn key_label(value: &str) -> String {
    key_names(value).collect::<Vec<_>>().join(" + ")
}
fn key_names(value: &str) -> impl Iterator<Item = String> + '_ {
    value.split('+').map(|key| {
        let key = key.trim().to_ascii_uppercase();
        let key = key.trim_start_matches("KEY_");
        let double = key.ends_with("*2");
        let key = key.strip_suffix("*2").unwrap_or(key);
        let modifier = |base: &&str| ["CTRL", "ALT", "SHIFT", "META"].contains(base);
        let (side, base) = match (key.strip_prefix("RIGHT"), key.strip_prefix("LEFT")) {
            (Some(base), _) if modifier(&base) => ("Right ", base),
            (_, Some(base)) if modifier(&base) => ("Left ", base),
            _ => ("", key),
        };
        let name = match base {
            "CTRL" | "CONTROL" => "Ctrl",
            "ALT" => "Alt",
            "SHIFT" => "Shift",
            "META" | "SUPER" if cfg!(target_os = "macos") => "Cmd",
            "META" | "SUPER" if cfg!(target_os = "windows") => "Win",
            "META" | "SUPER" => "Super",
            "ESC" => "Esc",
            "PAGEUP" => "Page Up",
            "PAGEDOWN" => "Page Down",
            "CAPSLOCK" => "Caps Lock",
            "LEFTBRACE" => "[",
            "RIGHTBRACE" => "]",
            "FN" => "Fn",
            "" => "Unbound",
            other
                if other.chars().count() == 1
                    || other.starts_with('F') && other[1..].parse::<u8>().is_ok() =>
            {
                return format!("{other}{}", if double { " ×2" } else { "" });
            }
            other => {
                let mut chars = other.chars();
                let first = chars.next().unwrap_or_default();
                return format!(
                    "{first}{}{}",
                    chars.as_str().to_ascii_lowercase(),
                    if double { " ×2" } else { "" }
                );
            }
        };
        format!("{side}{name}{}", if double { " ×2" } else { "" })
    })
}
pub const SURFACE_WIDTH: f32 = 640.;
pub const SURFACE_HEIGHT: f32 = 180.;
pub const ENTER_FADE: f32 = 0.12;
pub const OPEN_DELAY: f32 = 0.075;
pub const STAGGER: f32 = 0.015;
pub const EXIT_TIME: f32 = 0.16;
pub const CHECK_TIME: f32 = 0.22;
pub const SUCCESS_HOLD: f32 = 0.35;
pub const CANCEL_TIME: f32 = 0.12;
pub const ERROR_HOLD: f32 = 1.8;
pub const COPIED_HOLD: f32 = 2.4;
/// Long enough to reach Undo.
pub const LEARNED_HOLD: f32 = 6.;
pub const COPIED_GAP: f32 = 6.;
pub const ERROR_TEXT_TIME: f32 = 0.15;
pub const SHAKE_TIME: f32 = 0.36;
pub const WAVE_PERIOD: f32 = 0.9;
pub const WAVE_STEP: f32 = 0.08;
pub const QUIET_PERIOD: f32 = 1.2;
pub const PILL_STIFFNESS: f32 = 420.;
pub const PILL_DAMPING: f32 = 32.;
pub const BAR_STIFFNESS: f32 = 900.;
pub const BAR_DAMPING: f32 = 26.;
