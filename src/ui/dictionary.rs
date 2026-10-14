use super::{theme::*, window::MainWindow};
use gpui::{Context, IntoElement, Styled, Window, div, prelude::*, px, rgb, rgba};
use gpui_kit::component::input::Input;

impl MainWindow {
    pub fn add_word(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let word = self.word_input.read(cx).value().trim().to_owned();
        if word.is_empty() {
            return;
        }
        self.state.update(cx, |state, cx| {
            if !state
                .preferences
                .dictionary
                .iter()
                .any(|existing| existing.to_lowercase() == word.to_lowercase())
            {
                state.preferences.dictionary.push(word);
                state
                    .preferences
                    .dictionary
                    .sort_by_key(|word| word.to_lowercase());
                state.persist("");
                cx.notify();
            }
        });
        self.word_input
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    pub fn dictionary(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                header("Dictionary").child(div().flex_1()).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            Input::new(&self.word_input)
                                .w(px(240.))
                                .min_h(px(CONTROL))
                                .max_h(px(CONTROL))
                                .text_size(px(13.))
                                .rounded(px(RADIUS))
                                .bg(rgb(BLACK)),
                        )
                        .child(
                            div()
                                .id("add-word")
                                .h(px(CONTROL))
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .rounded(px(RADIUS))
                                .bg(rgb(WHITE))
                                .text_color(rgb(BLACK))
                                .cursor_pointer()
                                .child("Add")
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.add_word(window, cx)),
                                ),
                        ),
                ),
            )
            .child(
                div()
                    .id("dictionary-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(
                        self.state
                            .read(cx)
                            .preferences
                            .dictionary
                            .iter()
                            .enumerate()
                            .map(|(index, word)| {
                                let word = word.clone();
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .px(px(INSET))
                                    .py(px(8.))
                                    .border_b_1()
                                    .border_color(rgb(ROW))
                                    .child(word.clone())
                                    .child(
                                        div()
                                            .id(("remove-word", index))
                                            .text_color(rgba(0xffffff66))
                                            .cursor_pointer()
                                            .hover(|style| style.text_color(rgb(WHITE)))
                                            .child("✕")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.update(cx, |state, cx| {
                                                    state
                                                        .preferences
                                                        .dictionary
                                                        .retain(|existing| *existing != word);
                                                    state.persist("");
                                                    cx.notify();
                                                });
                                            })),
                                    )
                            }),
                    ),
            )
    }
}
