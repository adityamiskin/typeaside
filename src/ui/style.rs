use super::{home::tone_label, theme::*, window::MainWindow};
use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, Styled, Window, div, prelude::*, px,
};
use gpui_kit::component::{
    Sizable,
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
};
use typeaside::config::{
    AppCategory, AppInstruction, MAX_APP_INSTRUCTIONS, MAX_INSTRUCTION_LEN, Tone, app_key,
};
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
const DESCRIPTION: &str = "Tell TypeAside how to write in a specific app, e.g. 'lowercase, no trailing period' for Slack.";

/// One text input per rule, kept in step with `config.app_instructions`.
pub struct AppInstructions {
    picker: Entity<SelectState<Vec<String>>>,
    inputs: Vec<(String, Entity<InputState>)>,
    recent: Vec<String>,
}
impl AppInstructions {
    pub fn new(window: &mut Window, cx: &mut Context<MainWindow>) -> Self {
        Self {
            picker: cx.new(|cx| SelectState::new(Vec::new(), None, window, cx)),
            inputs: Vec::new(),
            recent: Vec::new(),
        }
    }
}
impl MainWindow {
    pub fn bind_app_instructions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(
            &self.app_instructions.picker,
            window,
            |this, picker, event, window, cx| {
                let SelectEvent::Confirm(Some(app)) = event else {
                    return;
                };
                let app = app.clone();
                this.state.update(cx, |s, cx| {
                    if s.config.app_instructions.len() < MAX_APP_INSTRUCTIONS {
                        s.config.app_instructions.push(AppInstruction {
                            app,
                            text: String::new(),
                        });
                        s.persist();
                        cx.notify();
                    }
                });
                picker.update(cx, |picker, cx| picker.set_selected_index(None, window, cx));
            },
        )
        .detach();
        self.sync_app_instructions(window, cx);
    }

    /// Rebuilds inputs and recent apps after the rules or history changed, including by sync.
    pub fn sync_app_instructions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = self.state.read(cx);
        let rules = s.config.app_instructions.clone();
        let taken: Vec<_> = rules.iter().map(|rule| app_key(&rule.app)).collect();
        let mut seen = taken.clone();
        let recent: Vec<String> = s
            .history
            .iter()
            .filter_map(|item| item.app_id.as_deref())
            .filter(|app| {
                let key = app_key(app);
                !seen.contains(&key) && {
                    seen.push(key);
                    true
                }
            })
            .map(str::to_owned)
            .collect();
        if self.app_instructions.recent != recent {
            self.app_instructions.recent = recent.clone();
            self.app_instructions
                .picker
                .update(cx, |picker, cx| picker.set_items(recent, window, cx));
        }
        let rows = &mut self.app_instructions.inputs;
        if rows
            .iter()
            .map(|(app, _)| app)
            .ne(rules.iter().map(|rule| &rule.app))
        {
            let mut old = std::mem::take(rows);
            for rule in &rules {
                let existing = old.iter().position(|(app, _)| *app == rule.app);
                let input = match existing {
                    Some(at) => old.swap_remove(at).1,
                    None => {
                        let input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .placeholder("How to write in this app")
                                .validate(|text, _| {
                                    text.encode_utf16().count() <= MAX_INSTRUCTION_LEN
                                })
                        });
                        let app = rule.app.clone();
                        cx.subscribe(&input, move |this, input, event, cx| {
                            if !matches!(event, InputEvent::Change) {
                                return;
                            }
                            let value = input.read(cx).value().to_string();
                            this.state.update(cx, |s, cx| {
                                let Some(rule) = s
                                    .config
                                    .app_instructions
                                    .iter_mut()
                                    .find(|rule| rule.app == app)
                                else {
                                    return;
                                };
                                if rule.text != value {
                                    rule.text = value;
                                    s.persist();
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                        input
                    }
                };
                self.app_instructions.inputs.push((rule.app.clone(), input));
            }
        }
        for ((_, input), rule) in self.app_instructions.inputs.iter().zip(&rules) {
            if input.read(cx).value() != rule.text.as_str() {
                input.update(cx, |input, cx| {
                    input.set_value(rule.text.clone(), window, cx)
                });
            }
        }
    }

    fn app_instructions_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let rules = &self.state.read(cx).config.app_instructions;
        let picker = Select::new(&self.app_instructions.picker)
            .small()
            .min_h(px(30.))
            .max_h(px(30.))
            .w(px(200.))
            .bg(p.bg)
            .text_size(px(13.))
            .placeholder("Add app");
        div()
            .pt(px(24.))
            .child(
                div()
                    .text_size(px(15.))
                    .line_height(px(21.75))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("App instructions"),
            )
            .children(self.app_instructions.inputs.iter().enumerate().map(
                |(index, (app, input))| {
                    let app = app.clone();
                    div()
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .min_h(px(44.))
                        .border_b_1()
                        .border_color(p.row)
                        .child(
                            div()
                                .w(px(200.))
                                .flex_shrink_0()
                                .truncate()
                                .child(app.clone()),
                        )
                        .child(
                            Input::new(input)
                                .flex_1()
                                .min_w_0()
                                .min_h(px(30.))
                                .max_h(px(30.))
                                .rounded(px(6.))
                                .text_size(px(13.))
                                .bg(p.bg),
                        )
                        .child(
                            div()
                                .id(("remove-app-instruction", index))
                                .size(px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(|b| b.bg(p.hover))
                                .child(
                                    gpui::svg()
                                        .path("icons/x.svg")
                                        .size(px(14.))
                                        .text_color(p.muted),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.state.update(cx, |s, cx| {
                                        s.config.app_instructions.retain(|rule| rule.app != app);
                                        s.persist();
                                        cx.notify();
                                    });
                                })),
                        )
                },
            ))
            .when(rules.len() < MAX_APP_INSTRUCTIONS, |section| {
                section.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .pt(px(10.))
                        .when(rules.is_empty(), |row| {
                            row.child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.))
                                    .line_height(px(17.4))
                                    .text_color(p.muted)
                                    .child(DESCRIPTION),
                            )
                        })
                        .child(picker),
                )
            })
    }

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
                    )
                    .child(self.app_instructions_section(cx))
                    .child(div().h(px(24.))),
            ),
            p,
        )
    }
}
