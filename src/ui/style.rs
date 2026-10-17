use super::{home::tone_label, theme::*, window::MainWindow};
use gpui::{Context, IntoElement, Styled, div, prelude::*, px};
use typeaside::config::{AppCategory, Tone};
const EXAMPLES: [[&str; 3]; 5] = [
    [
        "Would you like to meet for lunch tomorrow at noon?",
        "Hey, want to grab lunch tomorrow at 12?",
        "lunch tomorrow at 12?",
    ],
    [
        "Are you available for lunch tomorrow at noon? Please let me know.",
        "Hey, are you free for lunch tomorrow? Let's do 12 if that works for you.",
        "free for lunch tomorrow? let's do 12",
    ],
    [
        "Thank you for your help. I will review the document this afternoon.",
        "Thanks for your help. I'll look at the doc this afternoon.",
        "thanks! i'll check the doc this afternoon",
    ],
    [
        "Please rename load_history to read_history in engine.rs.",
        "Rename load_history to read_history in engine.rs.",
        "rename load_history to read_history in engine.rs",
    ],
    [
        "Please update the pagination cursor and verify the empty page.",
        "Update the pagination cursor and check the empty page.",
        "fix pagination cursor, check empty page",
    ],
];
const ROWS: [(AppCategory, &str, &str); 5] = [
    (
        AppCategory::Personal,
        "Personal messages",
        "Telegram, WhatsApp, Signal, Discord",
    ),
    (AppCategory::Work, "Work messages", "Slack, Teams, Linear"),
    (AppCategory::Email, "Email", "Thunderbird, Geary, Gmail"),
    (
        AppCategory::Code,
        "Code",
        "VS Code, Zed, JetBrains, terminals",
    ),
    (
        AppCategory::Other,
        "Everything else",
        "Browsers and other apps",
    ),
];
impl MainWindow {
    pub fn style(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let styles = self.state.read(cx).config.styles;
        let selected = ROWS
            .iter()
            .position(|(c, _, _)| *c == self.style_category)
            .unwrap_or(1);
        let tone = styles.get(self.style_category);
        let tone_index = match tone {
            Tone::Formal => 0,
            Tone::Casual => 1,
            Tone::VeryCasual => 2,
        };
        page(
            FORM_WIDTH,
            header("Style"),
            div().id("style-scroll").child(
                column(FORM_WIDTH)
                    .pt(px(8.))
                    .children(
                        ROWS.into_iter()
                            .enumerate()
                            .map(|(i, (category, label, apps))| {
                                div()
                                    .id(("style-row", i))
                                    .flex()
                                    .items_center()
                                    .gap(px(16.))
                                    .min_h(px(52.))
                                    .mx(px(-8.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .border_b_1()
                                    .border_color(p.row)
                                    .bg(if category == self.style_category {
                                        p.tint
                                    } else {
                                        p.bg
                                    })
                                    .on_hover(cx.listener(move |this, hovered, _, cx| {
                                        if *hovered {
                                            this.style_category = category;
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        div().w(px(200.)).flex_shrink_0().child(label).child(
                                            div()
                                                .text_size(px(12.))
                                                .line_height(px(17.4))
                                                .text_color(p.muted)
                                                .text_ellipsis()
                                                .child(apps),
                                        ),
                                    )
                                    .child(div().flex_1())
                                    .child(
                                        div()
                                            .flex()
                                            .h(px(26.))
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(p.border)
                                            .overflow_hidden()
                                            .children(
                                                [Tone::Formal, Tone::Casual, Tone::VeryCasual]
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(t, tone)| {
                                                        let on = styles.get(category) == tone;
                                                        div()
                                                            .id(("tone", i * 3 + t))
                                                            .flex()
                                                            .items_center()
                                                            .px(px(10.))
                                                            .text_size(px(12.))
                                                            .line_height(px(17.4))
                                                            .cursor_pointer()
                                                            .font_weight(if on {
                                                                gpui::FontWeight::MEDIUM
                                                            } else {
                                                                gpui::FontWeight::NORMAL
                                                            })
                                                            .bg(if on { p.accent } else { p.bg })
                                                            .text_color(if on {
                                                                p.on_accent
                                                            } else {
                                                                p.fg
                                                            })
                                                            .child(tone_label(tone))
                                                            .on_click(cx.listener(
                                                                move |this, _, _, cx| {
                                                                    this.style_category = category;
                                                                    this.state.update(
                                                                        cx,
                                                                        |s, cx| {
                                                                            *s.config
                                                                                .styles
                                                                                .get_mut(
                                                                                    category,
                                                                                ) = tone;
                                                                            s.persist();
                                                                            cx.notify();
                                                                        },
                                                                    );
                                                                },
                                                            ))
                                                    }),
                                            ),
                                    )
                            }),
                    )
                    .child(
                        div()
                            .mt(px(14.))
                            .py(px(14.))
                            .border_t_1()
                            .border_color(p.divider)
                            .flex()
                            .gap(px(16.))
                            .child(
                                div()
                                    .w(px(200.))
                                    .text_size(px(12.))
                                    .line_height(px(17.4))
                                    .text_color(p.muted)
                                    .child(ROWS[selected].1),
                            )
                            .child(div().flex_1().child(EXAMPLES[selected][tone_index])),
                    ),
            ),
            p,
        )
    }
}
