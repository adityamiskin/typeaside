use super::{theme::*, window::MainWindow};
use gpui::{AppContext, Context, Entity, IntoElement, Styled, Window, div, prelude::*, px};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use typeaside::config::Snippet;
pub struct SnippetEditor {
    pub index: Option<usize>,
    pub trigger: Entity<InputState>,
    pub text: Entity<TextareaState>,
}
impl MainWindow {
    pub fn edit_snippet(
        &mut self,
        index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let snippet = index
            .and_then(|i| self.state.read(cx).config.snippets.get(i))
            .cloned();
        let trigger = snippet
            .as_ref()
            .map(|s| s.trigger.clone())
            .unwrap_or_default();
        let text = snippet.map(|s| s.text).unwrap_or_default();
        self.snippet_editor = Some(SnippetEditor {
            index,
            trigger: cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(trigger)
                    .placeholder("Trigger")
            }),
            text: cx.new(|cx| TextareaState::new(window, cx).default_value(text).rows(3)),
        });
        cx.notify();
    }
    pub fn snippets(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let mut rows = Vec::new();
        if self
            .snippet_editor
            .as_ref()
            .is_some_and(|e| e.index.is_none())
        {
            rows.push(self.snippet_form(cx).into_any_element());
        }
        let snippets = self.state.read(cx).config.snippets.clone();
        for (i, snippet) in snippets.iter().enumerate() {
            if self
                .snippet_editor
                .as_ref()
                .is_some_and(|e| e.index == Some(i))
            {
                rows.push(self.snippet_form(cx).into_any_element());
                continue;
            }
            rows.push(
                div()
                    .id(("snippet", i))
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .min_h(px(38.))
                    .border_b_1()
                    .border_color(p.row)
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(200.))
                            .flex_shrink_0()
                            .text_ellipsis()
                            .child(snippet.trigger.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .line_height(px(17.4))
                            .text_color(p.muted)
                            .text_ellipsis()
                            .child(snippet.text.clone()),
                    )
                    .on_click(cx.listener(move |this, _, w, cx| this.edit_snippet(Some(i), w, cx)))
                    .into_any_element(),
            );
        }
        page(
            LIST_WIDTH,
            header("Snippets").child(div().flex_1()).child(
                button("New snippet", true, p)
                    .id("new-snippet")
                    .on_click(cx.listener(|this, _, w, cx| this.edit_snippet(None, w, cx))),
            ),
            div()
                .id("snippets-scroll")
                .child(column(LIST_WIDTH).pt(px(8.)).children(rows)),
            p,
        )
    }
    fn snippet_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let e = self.snippet_editor.as_ref().expect("active editor");
        div()
            .flex()
            .items_start()
            .gap(px(16.))
            .py(px(8.))
            .border_b_1()
            .border_color(p.row)
            .child(
                Input::new(&e.trigger)
                    .w(px(200.))
                    .min_h(px(30.))
                    .max_h(px(30.))
                    .bg(p.bg)
                    .text_size(px(13.)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        Textarea::new(&e.text)
                            .h(px(64.))
                            .bg(p.bg)
                            .text_size(px(13.)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(10.))
                            .mt(px(8.))
                            .when(e.index.is_some(), |row| {
                                row.child(
                                    button("Delete", false, p)
                                        .text_color(p.red)
                                        .id("delete-snippet")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Some(e) = this.snippet_editor.take()
                                                && let Some(i) = e.index
                                            {
                                                this.state.update(cx, |s, cx| {
                                                    s.config.snippets.remove(i);
                                                    s.persist();
                                                    cx.notify();
                                                });
                                            }
                                            cx.notify();
                                        })),
                                )
                            })
                            .child(button("Cancel", false, p).id("cancel-snippet").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.snippet_editor = None;
                                    cx.notify();
                                }),
                            ))
                            .child(button("Save", true, p).id("save-snippet").on_click(
                                cx.listener(|this, _, _, cx| {
                                    let Some(e) = &this.snippet_editor else {
                                        return;
                                    };
                                    let trigger = e.trigger.read(cx).value().trim().to_owned();
                                    let text = e.text.read(cx).value().to_string();
                                    if trigger.is_empty() || text.trim().is_empty() {
                                        return;
                                    }
                                    let i = e.index;
                                    this.state.update(cx, |s, cx| {
                                        let snippet = Snippet { trigger, text };
                                        if let Some(i) = i {
                                            s.config.snippets[i] = snippet;
                                        } else {
                                            s.config.snippets.insert(0, snippet);
                                        }
                                        s.persist();
                                        cx.notify();
                                    });
                                    this.snippet_editor = None;
                                    cx.notify();
                                }),
                            )),
                    ),
            )
    }
}
