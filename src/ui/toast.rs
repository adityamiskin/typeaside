use super::{theme::*, window::MainWindow};
use gpui::{Context, IntoElement, Styled, div, prelude::*, px};
use std::time::Duration;

pub struct Toast {
    message: &'static str,
}

impl MainWindow {
    pub fn show_toast(&mut self, message: &'static str, cx: &mut Context<Self>) {
        self.toast = Some(Toast { message });
        self.toast_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(2500))
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
                return false;
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
            true
        });
        if deleted {
            self.show_toast("Deleted", cx);
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
                        .child(toast.message),
                )
        })
    }
}
