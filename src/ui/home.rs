use super::{
    theme::*,
    window::{MainWindow, Page},
};
use chrono::{DateTime, Local};
use gpui::{
    ClipboardItem, Context, FontFeatures, FontWeight, IntoElement, Styled, div, prelude::*, px,
};
use gpui_kit::component::input::Input;
use std::collections::BTreeSet;
use typeaside::{
    account,
    config::{AppCategory, Tone},
    engine::{Command, Dictation, Mode, Phase},
};

struct Stats {
    words: usize,
    wpm: u64,
    saved_minutes: u64,
    streak: usize,
    median: u32,
}
impl Stats {
    fn from_history(history: &[Dictation]) -> Self {
        let history = history.iter().filter(|d| d.mode != Mode::File);
        let words = history
            .clone()
            .map(|d| d.cleaned.split_whitespace().count())
            .sum::<usize>();
        let audio_ms = history.clone().map(|d| u64::from(d.audio_ms)).sum::<u64>();
        let mut latencies = history.clone().map(|d| d.latency_ms).collect::<Vec<_>>();
        latencies.sort_unstable();
        let median = if latencies.is_empty() {
            0
        } else if latencies.len() % 2 == 0 {
            ((u64::from(latencies[latencies.len() / 2 - 1])
                + u64::from(latencies[latencies.len() / 2]))
                / 2) as u32
        } else {
            latencies[latencies.len() / 2]
        };
        let days = history
            .clone()
            .map(|d| DateTime::<Local>::from(d.at).date_naive())
            .collect::<BTreeSet<_>>();
        let today = Local::now().date_naive();
        let mut day = if days.contains(&today) {
            Some(today)
        } else {
            today.pred_opt()
        };
        let mut streak = 0;
        while let Some(current) = day {
            if !days.contains(&current) {
                break;
            }
            streak += 1;
            day = current.pred_opt();
        }
        Self {
            words,
            wpm: if audio_ms == 0 {
                0
            } else {
                (words as f64 * 60000. / audio_ms as f64).round() as u64
            },
            saved_minutes: (words as f64 / 40. - audio_ms as f64 / 60000.)
                .max(0.)
                .floor() as u64,
            streak,
            median,
        }
    }
}
impl MainWindow {
    pub fn transcribe_file(&self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        if !state.is_demo && state.file_transcribing.is_none() && state.phase == Phase::Idle {
            state.command(Command::TranscribeFile(path));
        }
    }
    pub fn home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let state = self.state.read(cx);
        let stats = Stats::from_history(&state.history);
        let values = [
            (number(stats.words), "words"),
            (stats.wpm.to_string(), "wpm"),
            (
                format!(
                    "{}h {}m",
                    stats.saved_minutes / 60,
                    stats.saved_minutes % 60
                ),
                "saved",
            ),
            (stats.streak.to_string(), "day streak"),
            (format!("{} ms", stats.median), "median latency"),
        ];
        let share_url = (stats.words > 0).then(|| {
            format!(
                "{}/share?w={}&m={}&wpm={}&d={}",
                account::web_url(),
                stats.words,
                stats.saved_minutes,
                stats.wpm,
                stats.streak
            )
        });
        let stats = div()
            .flex()
            .items_center()
            .gap(px(16.))
            .pt(px(18.))
            .pb(px(16.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(40.))
                    .children(values.into_iter().map(|(v, label)| {
                        div()
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .line_height(px(24.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(v),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.4))
                                    .text_color(p.muted)
                                    .child(label),
                            )
                    })),
            )
            .child(
                div()
                    .id("transcribe-file")
                    .ml_auto()
                    .min_w_0()
                    .text_ellipsis()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(
                        state
                            .file_transcribing
                            .as_ref()
                            .map(|name| format!("Transcribing {name}…"))
                            .unwrap_or_else(|| "Transcribe file".into()),
                    )
                    .when(state.file_transcribing.is_none(), |link| {
                        link.hover(|style| style.text_color(p.fg))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.state.read(cx).is_demo {
                                    return;
                                }
                                let paths = cx.prompt_for_paths(gpui::PathPromptOptions {
                                    files: true,
                                    directories: false,
                                    multiple: false,
                                    prompt: None,
                                });
                                cx.spawn(async move |this, cx| {
                                    if let Ok(Ok(Some(paths))) = paths.await
                                        && let Some(path) = paths.into_iter().next()
                                    {
                                        let _ = this
                                            .update(cx, |this, cx| this.transcribe_file(path, cx));
                                    }
                                })
                                .detach();
                            }))
                    }),
            )
            .when_some(share_url, |row, url| {
                row.child(
                    div()
                        .id("share-stats")
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .hover(|style| style.text_color(p.fg))
                        .cursor_pointer()
                        .child("Share")
                        .on_click(move |_, _, _| {
                            let _ = account::open(&url);
                        }),
                )
            });
        let mut rows = Vec::new();
        if let Some(failed) = &state.failed {
            let at: DateTime<Local> = failed.at.into();
            let seconds = failed.audio_ms / 1000;
            let actions =
                if failed.retrying {
                    div().text_color(p.muted).child("Retrying")
                } else {
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(
                            button("Retry", true, p)
                                .id("retry-failed")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.update(cx, |s, cx| {
                                        if !s.is_demo {
                                            s.command(Command::RetryFailed);
                                        }
                                        cx.notify();
                                    });
                                })),
                        )
                        .child(button("Discard", false, p).id("discard-failed").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |s, cx| {
                                    if s.is_demo {
                                        s.failed = None;
                                    } else {
                                        s.command(Command::DiscardFailed);
                                    }
                                    cx.notify();
                                });
                            }),
                        ))
                };
            rows.push(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .px(px(8.))
                    .mx(px(-8.))
                    .py(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .flex_1()
                            .min_w_0()
                            .child(div().size(px(6.)).flex_shrink_0().rounded_full().bg(p.red))
                            .child(div().text_color(p.fg).child("Recording didn't transcribe"))
                            .child(div().text_color(p.muted).child(format!(
                                "{}:{:02} · {}",
                                seconds / 60,
                                seconds % 60,
                                at.format("%H:%M")
                            ))),
                    )
                    .child(actions)
                    .into_any_element(),
            );
        }
        let mut previous = None;
        let today = Local::now().date_naive();
        let query = state.search.to_lowercase();
        for (index, item) in state.history.iter().enumerate().filter(|(_, d)| {
            d.cleaned.to_lowercase().contains(&query) || d.raw.to_lowercase().contains(&query)
        }) {
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
                        .pt(px(16.))
                        .pb(px(6.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_size(px(12.))
                        .line_height(px(17.4))
                        .child(label)
                        .into_any_element(),
                );
                previous = Some(day);
            }
            let hovered = self.hovered_history == Some(index);
            let text = item.cleaned.clone();
            let raw = item.raw.clone();
            let timestamp = item.at;
            let editing = self
                .correction_editor
                .as_ref()
                .is_some_and(|editor| editor.at == timestamp);
            let notice = self
                .learned_notice
                .as_ref()
                .filter(|(at, _, _)| *at == timestamp);
            let actions = div()
                .flex()
                .gap(px(14.))
                .text_size(px(12.))
                .line_height(px(19.))
                .child(
                    div()
                        .id(("edit", index))
                        .cursor_pointer()
                        .child("Edit")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.edit_history(timestamp, window, cx)
                        })),
                )
                .child(
                    div()
                        .id(("copy", index))
                        .cursor_pointer()
                        .child("Copy")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(text.clone()))
                        })),
                )
                .child(
                    div()
                        .id(("details", index))
                        .cursor_pointer()
                        .child("Details")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.state.update(cx, |s, cx| {
                                s.expanded = if s.expanded == Some(timestamp) {
                                    None
                                } else {
                                    Some(timestamp)
                                };
                                cx.notify();
                            });
                        })),
                )
                .child(
                    div()
                        .id(("delete", index))
                        .text_color(p.red)
                        .cursor_pointer()
                        .child("Delete")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.state.update(cx, |s, cx| {
                                s.history.retain(|d| d.at != timestamp);
                                if s.expanded == Some(timestamp) {
                                    s.expanded = None;
                                }
                                if !s.is_demo {
                                    s.save_error =
                                        typeaside::engine::save_history(&s.paths, &s.history)
                                            .err()
                                            .map(|e| format!("Could not save history: {e}"));
                                }
                                cx.notify();
                            });
                        })),
                );
            rows.push(
                div()
                    .id(("history", index))
                    .flex()
                    .items_start()
                    .gap(px(16.))
                    .px(px(8.))
                    .mx(px(-8.))
                    .py(px(8.))
                    .rounded(px(6.))
                    .bg(if hovered { p.hover } else { p.bg })
                    .on_hover(cx.listener(move |this, hovered, _, cx| {
                        // The next row's hover-in can arrive before this row's hover-out.
                        if *hovered {
                            this.hovered_history = Some(index);
                        } else if this.hovered_history == Some(index) {
                            this.hovered_history = None;
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(52.))
                            .flex_shrink_0()
                            .font_family(MONO)
                            .text_size(px(12.))
                            .line_height(px(19.))
                            .text_color(p.muted)
                            .child(at.format("%H:%M").to_string()),
                    )
                    .child(if editing {
                        self.correction_form(cx).into_any_element()
                    } else {
                        div()
                            .flex_1()
                            .min_w_0()
                            .line_clamp(2)
                            .child(item.cleaned.clone())
                            .into_any_element()
                    })
                    .child(if editing {
                        div().into_any_element()
                    } else if let Some((_, _, message)) = notice {
                        div()
                            .text_size(px(12.))
                            .line_height(px(19.))
                            .text_color(p.muted)
                            .child(message.clone())
                            .into_any_element()
                    } else if hovered {
                        actions.into_any_element()
                    } else {
                        div()
                            .text_size(px(12.))
                            .line_height(px(19.))
                            .text_color(p.muted)
                            .child(friendly_app(item.app_id.as_deref()).to_owned())
                            .into_any_element()
                    })
                    .into_any_element(),
            );
            if state.expanded == Some(timestamp) {
                let category = AppCategory::of(item.app_id.as_deref());
                let tone = state.config.styles.get(category);
                let app = format!(
                    "{} · {} · {}",
                    friendly_app(item.app_id.as_deref()),
                    category_label(category),
                    tone_label(tone)
                );
                let timing = format!(
                    "{:.1} s audio · {} ms",
                    item.audio_ms as f32 / 1000.,
                    item.latency_ms
                );
                let mut details = div()
                    .ml(px(68.))
                    .mt(px(2.))
                    .mb(px(6.))
                    .pt(px(8.))
                    .pb(px(4.))
                    .text_size(px(12.))
                    .line_height(px(17.4))
                    .flex()
                    .flex_col()
                    .gap(px(6.));
                for (label, text) in [("Raw", item.raw.clone()), ("App", app), ("Timing", timing)] {
                    details = details.child(
                        div()
                            .flex()
                            .gap(px(16.))
                            .child(
                                div()
                                    .w(px(64.))
                                    .flex_shrink_0()
                                    .text_color(p.muted)
                                    .child(label),
                            )
                            .child(div().flex_1().child(text)),
                    );
                }
                details = details.child(
                    div()
                        .ml(px(80.))
                        .flex()
                        .gap(px(14.))
                        .child(
                            div()
                                .id(("copy-raw", index))
                                .cursor_pointer()
                                .child("Copy raw")
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(raw.clone()))
                                })),
                        )
                        .child(
                            div()
                                .id(("add-unknown", index))
                                .cursor_pointer()
                                .child("Add word to dictionary")
                                .on_click(cx.listener(|this, _, w, cx| {
                                    this.set_page(Page::Dictionary, w, cx);
                                    this.word_input.update(cx, |input, cx| {
                                        input.set_value("", w, cx);
                                        input.focus(w, cx);
                                    });
                                })),
                        ),
                );
                rows.push(details.into_any_element());
            }
        }
        if state.history.is_empty() {
            rows.push(
                div()
                    .pt(px(18.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_color(p.muted)
                    .child("Hold")
                    .child(kbd(key_label(&state.config.hotkey), p))
                    .child("and speak.")
                    .into_any_element(),
            );
        } else if rows.is_empty() {
            rows.push(
                div()
                    .pt(px(18.))
                    .text_color(p.muted)
                    .child("No matching dictations.")
                    .into_any_element(),
            );
        }
        page(
            LIST_WIDTH,
            header("Home").child(div().flex_1()).child(
                Input::new(&self.search_input)
                    .w(px(240.))
                    .min_h(px(30.))
                    .max_h(px(30.))
                    .text_size(px(13.))
                    .bg(p.bg),
            ),
            div()
                .id("history-scroll")
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .border_b_1()
                        .border_color(p.divider)
                        .child(column(LIST_WIDTH).child(stats)),
                )
                .child(column(LIST_WIDTH).children(rows)),
            p,
        )
        .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
    }
}
fn number(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
pub fn category_label(category: AppCategory) -> &'static str {
    match category {
        AppCategory::Personal => "Personal",
        AppCategory::Work => "Work",
        AppCategory::Email => "Email",
        AppCategory::Code => "Code",
        AppCategory::Other => "Other",
    }
}
pub fn tone_label(tone: Tone) -> &'static str {
    match tone {
        Tone::Formal => "Formal",
        Tone::Casual => "Casual",
        Tone::VeryCasual => "Very casual",
    }
}
fn friendly_app(app: Option<&str>) -> &str {
    match app {
        Some("com.slack.Slack" | "slack") => "Slack",
        Some("thunderbird") => "Thunderbird",
        Some("Alacritty" | "alacritty") => "Alacritty",
        Some(other) => other,
        None => "",
    }
}
