use super::{theme::*, window::MainWindow};
use gpui::{Context, IntoElement, Styled, Window, div, prelude::*, px};
use gpui_kit::component::input::Input;
impl MainWindow {
    pub fn add_word(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let word = self.word_input.read(cx).value().trim().to_owned();
        if word.is_empty() {
            return;
        }
        self.state.update(cx, |s, cx| {
            if !s
                .config
                .dictionary
                .iter()
                .any(|existing| existing.to_lowercase() == word.to_lowercase())
            {
                s.config.dictionary.push(word);
                s.persist();
                cx.notify();
            }
        });
        self.word_input
            .update(cx, |input, cx| input.set_value("", window, cx));
    }
    pub fn dictionary(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let words = &self.state.read(cx).config.dictionary;
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                header("Dictionary", p)
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(17.4))
                            .text_color(p.muted)
                            .child(format!("{} words", words.len())),
                    )
                    .child(div().flex_1())
                    .child(
                        Input::new(&self.word_input)
                            .w(px(240.))
                            .min_h(px(30.))
                            .max_h(px(30.))
                            .text_size(px(13.))
                            .bg(p.bg),
                    )
                    .child(
                        button("Add", false, p)
                            .id("add-word")
                            .on_click(cx.listener(|this, _, w, cx| this.add_word(w, cx))),
                    ),
            )
            .child(
                div()
                    .id("dictionary-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(INSET))
                    .pt(px(8.))
                    .children(words.chunks(3).enumerate().map(|(row, chunk)| {
                        div().flex().gap(px(24.)).children((0..3).map(|col| {
                            let index = row * 3 + col;
                            let Some(word) = chunk.get(col) else {
                                return div().flex_1().into_any_element();
                            };
                            let word = word.clone();
                            div()
                                .id(("word", index))
                                .flex_1()
                                .min_w_0()
                                .h(px(34.))
                                .flex()
                                .items_center()
                                .border_b_1()
                                .border_color(p.row)
                                .on_hover(cx.listener(move |this, hovered, _, cx| {
                                    this.hovered_word = if *hovered { Some(index) } else { None };
                                    cx.notify();
                                }))
                                .child(div().flex_1().text_ellipsis().child(word.clone()))
                                .when(self.hovered_word == Some(index), |row| {
                                    row.child(
                                        div()
                                            .id(("remove", index))
                                            .text_color(p.muted)
                                            .text_size(px(12.))
                                            .line_height(px(17.4))
                                            .cursor_pointer()
                                            .child("Remove")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.update(cx, |s, cx| {
                                                    s.config.dictionary.retain(|v| v != &word);
                                                    s.persist();
                                                    cx.notify();
                                                });
                                            })),
                                    )
                                })
                                .into_any_element()
                        }))
                    })),
            )
    }
}
