use super::{theme::*, window::MainWindow};
use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, Styled, Subscription, Window, div,
    prelude::*, px,
};
use gpui_kit::component::input::{InputEvent, Position, Textarea, TextareaState};
use std::{
    collections::HashSet,
    time::{Duration, Instant, SystemTime},
};
use typeaside::corrections;

pub struct CorrectionEditor {
    pub at: SystemTime,
    original: String,
    pub input: Entity<TextareaState>,
    pub excluded: HashSet<String>,
    _subscription: Subscription,
}

impl MainWindow {
    pub fn edit_history(&mut self, at: SystemTime, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self
            .state
            .read(cx)
            .history
            .iter()
            .find(|item| item.at == at)
        else {
            return;
        };
        let original = item.cleaned.clone();
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .default_value(original.clone())
                .auto_grow(2, 8)
        });
        let subscription = cx.subscribe(&input, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        input.update(cx, |input, cx| {
            let last = original.split('\n').next_back().unwrap_or_default();
            input.set_cursor_position(
                Position::new(
                    original.matches('\n').count() as u32,
                    last.chars().count() as u32,
                ),
                window,
                cx,
            );
        });
        self.correction_editor = Some(CorrectionEditor {
            at,
            original,
            input,
            excluded: HashSet::new(),
            _subscription: subscription,
        });
        cx.notify();
    }

    pub fn save_correction(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.correction_editor else {
            return;
        };
        let text = editor.input.read(cx).value().to_string();
        if text == editor.original {
            self.correction_editor = None;
            cx.notify();
            return;
        }
        let at = editor.at;
        let selected = corrections::candidates(
            &editor.original,
            &text,
            &self.state.read(cx).config.dictionary,
        )
        .into_iter()
        .filter(|word| !editor.excluded.contains(&word.to_lowercase()))
        .collect::<Vec<_>>();
        let result = self.state.update(cx, |s, cx| {
            let result = if s.is_demo {
                if let Some(item) = s.history.iter_mut().find(|item| item.at == at) {
                    item.cleaned = text;
                }
                s.config.dictionary.extend(selected.clone());
                Ok(selected)
            } else {
                corrections::save(
                    &s.paths,
                    &mut s.history,
                    &mut s.config,
                    at,
                    &text,
                    &selected,
                )
            };
            match &result {
                Ok(_) => {
                    s.save_error = None;
                    s.command(typeaside::engine::Command::Reload(Box::new(
                        s.config.clone(),
                    )));
                }
                Err(error) => s.save_error = Some(format!("Could not save correction: {error}")),
            }
            cx.notify();
            result
        });
        if let Ok(added) = result {
            self.correction_editor = None;
            self.learned_notice = if added.is_empty() {
                None
            } else {
                Some((at, Instant::now(), format!("Added {}", added.join(", "))))
            };
            if let Some((_, started, _)) = self.learned_notice.clone() {
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Duration::from_secs(3)).await;
                    let _ = this.update(cx, |this, cx| {
                        if this
                            .learned_notice
                            .as_ref()
                            .is_some_and(|(_, time, _)| *time == started)
                        {
                            this.learned_notice = None;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    pub fn correction_form(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let editor = self.correction_editor.as_ref().expect("active correction");
        let candidates = corrections::candidates(
            &editor.original,
            &editor.input.read(cx).value(),
            &self.state.read(cx).config.dictionary,
        );
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(8.))
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "enter" if !event.keystroke.modifiers.shift => {
                        this.save_correction(cx);
                        if this.correction_editor.is_none() {
                            this.focus.focus(window, cx);
                        }
                        cx.stop_propagation();
                        window.prevent_default();
                    }
                    "escape" => {
                        this.correction_editor = None;
                        this.focus.focus(window, cx);
                        cx.notify();
                        cx.stop_propagation();
                        window.prevent_default();
                    }
                    _ => {}
                }
            }))
            .child(
                Textarea::new(&editor.input)
                    .bg(p.bg)
                    .text_size(px(13.))
                    .line_height(px(18.85)),
            )
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .when(!candidates.is_empty(), |row| {
                                row.child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(10.))
                                        .text_size(px(12.))
                                        .line_height(px(19.))
                                        .child(
                                            div().text_color(p.muted).child("Add to dictionary:"),
                                        )
                                        .children(candidates.into_iter().enumerate().map(
                                            |(index, word)| {
                                                let key = word.to_lowercase();
                                                let selected = !editor.excluded.contains(&key);
                                                div()
                                                    .id(("learn-word", index))
                                                    .cursor_pointer()
                                                    .child(word)
                                                    .text_color(if selected {
                                                        p.blue
                                                    } else {
                                                        p.faint
                                                    })
                                                    .when(selected, |word| {
                                                        word.font_weight(FontWeight::MEDIUM)
                                                    })
                                                    .when(!selected, |word| word.line_through())
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        if let Some(editor) =
                                                            &mut this.correction_editor
                                                            && !editor.excluded.remove(&key)
                                                        {
                                                            editor.excluded.insert(key.clone());
                                                        }
                                                        cx.notify();
                                                    }))
                                            },
                                        )),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(10.))
                            .child(button("Cancel", false, p).id("cancel-correction").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.correction_editor = None;
                                    this.focus.focus(window, cx);
                                    cx.notify();
                                }),
                            ))
                            .child(button("Save", true, p).id("save-correction").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.save_correction(cx);
                                    if this.correction_editor.is_none() {
                                        this.focus.focus(window, cx);
                                    }
                                }),
                            )),
                    ),
            )
    }
}
