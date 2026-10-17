use super::{theme::*, window::MainWindow};
use gpui::{Context, IntoElement, Styled, div, prelude::*, px};
use std::time::Duration;
use typeaside::engine::Dictation;

pub struct Toast {
    message: &'static str,
    deleted: Option<(usize, Dictation)>,
}

impl MainWindow {
    pub fn show_toast(
        &mut self,
        message: &'static str,
        deleted: Option<(usize, Dictation)>,
        cx: &mut Context<Self>,
    ) {
        let duration = if deleted.is_some() { 4000 } else { 2500 };
        self.toast = Some(Toast { message, deleted });
        self.toast_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(duration))
                .await;
            let _ = this.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub fn delete_history(&mut self, index: usize, cx: &mut Context<Self>) {
        let deleted = self.state.update(cx, |s, cx| {
            if index >= s.history.len() {
                return None;
            }
            let item = s.history.remove(index);
            if s.expanded == Some(item.at) {
                s.expanded = None;
            }
            if !s.is_demo {
                s.save_error = typeaside::engine::save_history(&s.paths, &s.history)
                    .err()
                    .map(|e| format!("Could not save history: {e}"));
            }
            cx.notify();
            Some(item)
        });
        if let Some(item) = deleted {
            self.show_toast("Deleted", Some((index, item)), cx);
        }
    }

    pub fn history_toast(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let p = palette(cx);
        self.toast.as_ref().map(|toast| {
            div()
                .absolute()
                .bottom(px(16.))
                .left_0()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(18.))
                        .px(px(14.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .bg(p.pill)
                        .border_1()
                        .border_color(p.pill_border)
                        .text_size(px(13.))
                        .text_color(p.fg)
                        .child(toast.message)
                        .when(toast.deleted.is_some(), |toast| {
                            toast.child(
                                div()
                                    .id("undo-history-delete")
                                    .cursor_pointer()
                                    .text_color(p.blue)
                                    .hover(|style| style.underline())
                                    .child("Undo")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let deleted =
                                            this.toast.take().and_then(|toast| toast.deleted);
                                        this.toast_task = None;
                                        if let Some((index, item)) = deleted {
                                            this.state.update(cx, |s, cx| {
                                                s.history.insert(index.min(s.history.len()), item);
                                                if !s.is_demo {
                                                    s.save_error =
                                                        typeaside::engine::save_history(
                                                            &s.paths, &s.history,
                                                        )
                                                        .err()
                                                        .map(|e| {
                                                            format!("Could not save history: {e}")
                                                        });
                                                }
                                                cx.notify();
                                            });
                                        }
                                        cx.notify();
                                    })),
                            )
                        }),
                )
        })
    }
}
