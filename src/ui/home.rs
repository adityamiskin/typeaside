use super::{theme::*, window::MainWindow};
use chrono::{DateTime, Local};
use gpui::{
    ClipboardItem, Context, FontFeatures, FontWeight, IntoElement, Styled, div, prelude::*, px,
    rgb, rgba,
};
use std::time::Duration;

impl MainWindow {
    pub fn home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let count = state.history.len();
        let words: usize = state
            .history
            .iter()
            .map(|item| item.cleaned.split_whitespace().count())
            .sum();
        let avg = if count == 0 {
            0
        } else {
            state
                .history
                .iter()
                .map(|item| u64::from(item.latency_ms))
                .sum::<u64>()
                / count as u64
        };
        let key = match state.preferences.hotkey.as_str() {
            "RIGHTCTRL" => "Right Ctrl",
            other => other,
        };
        let prompt = div()
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(13.))
            .child(if state.preferences.toggle {
                "Press"
            } else {
                "Hold"
            })
            .child(
                div()
                    .text_size(px(11.))
                    .font_family(MONO)
                    .border_1()
                    .border_color(rgb(0x333333))
                    .rounded(px(4.))
                    .px(px(5.))
                    .py(px(1.))
                    .child(key.to_owned()),
            )
            .child(if state.preferences.toggle {
                "to start and stop"
            } else {
                "to dictate"
            });
        let head = header("Home")
            .flex_wrap()
            .child(prompt)
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgba(MUTED))
                    .child(if self.copied {
                        "Copied".into()
                    } else {
                        format!("{count} dictations · {words} words · {avg} ms avg")
                    }),
            );
        let mut rows = Vec::new();
        let mut previous = None;
        let today = Local::now().date_naive();
        for (index, item) in state.history.iter().enumerate() {
            let at: DateTime<Local> = item.at.into();
            let day = at.date_naive();
            if previous != Some(day) {
                let label = if day == today {
                    "Today".into()
                } else if Some(day) == today.pred_opt() {
                    "Yesterday".into()
                } else {
                    at.format("%a %d %b").to_string()
                };
                rows.push(
                    div()
                        .px(px(INSET))
                        .pt(px(14.))
                        .pb(px(6.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_size(px(12.))
                        .child(label)
                        .into_any_element(),
                );
                previous = Some(day);
            }
            let text = item.cleaned.clone();
            rows.push(
                div()
                    .id(("history", index))
                    .flex()
                    .items_start()
                    .gap(px(16.))
                    .px(px(INSET))
                    .py(px(10.))
                    .border_b_1()
                    .border_color(rgb(ROW))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(HOVER)))
                    .child(
                        div()
                            .w(px(72.))
                            .flex_shrink_0()
                            .pt(px(1.))
                            .text_size(px(12.))
                            .text_color(rgba(MUTED))
                            .child(at.format("%H:%M").to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .line_clamp(2)
                            .child(item.cleaned.clone()),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .flex_shrink_0()
                            .pt(px(1.))
                            .text_size(px(12.))
                            .text_color(rgba(MUTED))
                            .text_right()
                            .text_ellipsis()
                            .child(friendly_app(item.app_id.as_deref()).to_owned()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                        this.copied = true;
                        this.copy_generation += 1;
                        let generation = this.copy_generation;
                        cx.notify();
                        cx.spawn(async move |this, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(1200))
                                .await;
                            let _ = this.update(cx, |this, cx| {
                                if this.copy_generation == generation {
                                    this.copied = false;
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                    }))
                    .into_any_element(),
            );
        }
        div()
            .flex()
            .flex_col()
            .size_full()
            .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
            .child(head)
            .child(if count == 0 {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(MUTED))
                    .child("Your dictations will show up here.")
                    .into_any_element()
            } else {
                div()
                    .id("history-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(rows)
                    .into_any_element()
            })
    }
}

fn friendly_app(app: Option<&str>) -> &str {
    match app {
        Some("com.slack.Slack" | "slack") => "Slack",
        Some("com.mitchellh.ghostty" | "ghostty") => "ghostty",
        Some("gmail") => "Gmail",
        Some("notes" | "org.gnome.Notes") => "Notes",
        Some("t3-code" | "T3 Code" | "com.t3tools.T3Code") => "T3 Code",
        Some("code" | "com.visualstudio.code") => "Visual Studio Code",
        Some("firefox" | "org.mozilla.firefox") => "Firefox",
        Some("google-chrome") => "Chrome",
        Some(other) => other,
        None => "",
    }
}
