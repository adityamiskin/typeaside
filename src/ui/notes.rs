use super::{AppState, home::history_button, theme::*, window::MainWindow};
use gpui::{
    App, AppContext, ClipboardItem, Context, Entity, Focusable, FontWeight, IntoElement, Styled,
    Window, div, prelude::*, px,
};
use gpui_kit::component::input::{Enter, Escape, Input, InputEvent, TextareaState};
use typeaside::notes::{
    Note, NoteColor, Op, checklist_item, hashtag_ranges, labels, toggle_checklist,
};

impl AppState {
    pub fn change_note(&mut self, op: Op) -> bool {
        let mut next = self.notes.clone();
        next.change(op);
        if !self.is_demo
            && let Err(error) = next.save(&self.paths)
        {
            self.save_error = Some(format!("Could not save note: {error}"));
            return false;
        }
        self.notes = next;
        let _ = self.notes_sync.try_send(());
        true
    }
}

pub fn schedule_sync(state: &Entity<AppState>, wake: async_channel::Receiver<()>, cx: &mut App) {
    let _ = state.read(cx).notes_sync.try_send(());
    let state = state.downgrade();
    cx.spawn(async move |cx| {
        while wake.recv().await.is_ok() {
            let Some(state) = state.upgrade() else { break };
            let (pending, revision) = cx.update(|cx| {
                (
                    state.read(cx).notes.pending.clone(),
                    state.read(cx).account_revision,
                )
            });
            let result = cx
                .background_executor()
                .spawn(async move {
                    let url = typeaside::account::hosted_url();
                    let host = typeaside::credentials::endpoint_host(&url).ok()?;
                    let token = typeaside::credentials::get_credential(&host).ok()??;
                    Some(typeaside::notes::synchronize(pending, &url, &token))
                })
                .await;
            if let Some((sent, remote)) = result {
                state.update(cx, |s, cx| {
                    if s.account_revision != revision {
                        return;
                    }
                    let mut next = s.notes.clone();
                    next.complete(&sent, remote);
                    match next.save(&s.paths) {
                        Ok(()) => s.notes = next,
                        Err(error) => s.save_error = Some(format!("Could not save notes: {error}")),
                    }
                    cx.notify();
                });
            }
        }
    })
    .detach();
}

pub struct Editor {
    note: Note,
    input: Entity<TextareaState>,
}

pub fn hints(p: Palette) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_end()
        .gap(px(14.))
        .text_size(px(12.))
        .text_color(p.faint)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(kbd("ENTER", p))
                .child("Save"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(kbd("SHIFT", p))
                .child(kbd("ENTER", p))
                .child("New line"),
        )
}

fn hashtag_text(text: &str, p: Palette) -> gpui::StyledText {
    gpui::StyledText::new(text.to_owned()).with_highlights(hashtag_ranges(text).into_iter().map(
        |range| {
            (
                range,
                gpui::HighlightStyle {
                    color: Some(p.muted.opacity(1.)),
                    fade_out: Some(1. - p.muted.a),
                    ..Default::default()
                },
            )
        },
    ))
}

impl MainWindow {
    fn put_note(&mut self, mut note: Note, cx: &mut Context<Self>) -> bool {
        note.updated_at = chrono::Utc::now()
            .timestamp_millis()
            .max(note.updated_at + 1);
        self.state.update(cx, |s, cx| {
            let saved = s.change_note(Op::Put(note));
            cx.notify();
            saved
        })
    }

    pub fn bind_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(
            &self.note_composer,
            window,
            |_, input, event, window, cx| {
                if matches!(event, InputEvent::Focus | InputEvent::Blur) {
                    input.update(cx, |input, cx| {
                        input.set_placeholder(
                            if matches!(event, InputEvent::Focus) {
                                ""
                            } else {
                                "Write a note…"
                            },
                            window,
                            cx,
                        )
                    });
                }
                cx.notify();
            },
        )
        .detach();
        cx.subscribe(&self.note_search, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
    }
    pub fn save_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.note_composer.read(cx).value().trim().to_owned();
        if text.is_empty() || text.chars().count() > 20_000 {
            return;
        }
        match typeaside::notes::new_note(text) {
            Ok(note) => {
                if self.state.update(cx, |s, cx| {
                    let saved = s.change_note(Op::Put(note));
                    cx.notify();
                    saved
                }) {
                    self.note_composer
                        .update(cx, |input, cx| input.set_value("", window, cx));
                }
            }
            Err(error) => self.state.update(cx, |s, cx| {
                s.save_error = Some(error.to_string());
                cx.notify();
            }),
        }
    }
    fn edit_note(&mut self, note: Note, window: &mut Window, cx: &mut Context<Self>) {
        self.save_note(cx);
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .default_value(note.text.clone())
                .auto_grow(1, usize::MAX)
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        cx.subscribe_in(&input, window, |this, _, event, _, cx| {
            if matches!(event, InputEvent::Blur) {
                this.save_note(cx);
            }
        })
        .detach();
        self.note_editor = Some(Editor { note, input });
        cx.notify();
    }
    pub fn save_note(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.note_editor.take() else {
            return;
        };
        let text = editor.input.read(cx).value().trim().to_owned();
        if text == editor.note.text {
            cx.notify();
            return;
        }
        if text.chars().count() > 20_000 {
            self.note_editor = Some(editor);
            return;
        }
        let mut note = editor.note.clone();
        let op = if text.is_empty() {
            Op::Delete(note.id.clone())
        } else {
            note.text = text;
            note.updated_at = chrono::Utc::now()
                .timestamp_millis()
                .max(note.updated_at + 1);
            Op::Put(note)
        };
        let deleting = matches!(op, Op::Delete(_));
        if !self.state.update(cx, |s, cx| {
            let saved = s.change_note(op);
            cx.notify();
            saved
        }) {
            self.note_editor = Some(editor);
        } else if deleting {
            self.show_toast("Deleted", cx);
        }
        cx.notify();
    }
    pub fn notes(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let state = self.state.read(cx);
        let focused = self
            .note_composer
            .read(cx)
            .focus_handle(cx)
            .is_focused(window);
        let mut rows = Vec::new();
        let composer = div()
            .id("note-composer")
            .rounded(px(10.))
            .border_1()
            .border_color(if focused {
                p.border.blend(p.fg.opacity(0.28))
            } else {
                p.border
            })
            .bg(p.panel)
            .px(px(14.))
            .py(px(11.))
            .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                if action.shift {
                    cx.propagate();
                } else {
                    this.save_composer(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(21.))
                    .child(self.note_composer.clone()),
            )
            .when(focused, |row| row.child(hints(p).mt(px(8.))));
        let view: Vec<_> = state
            .notes
            .notes
            .iter()
            .filter(|note| note.archived == self.note_archive)
            .collect();
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for note in &view {
            for label in labels(&note.text) {
                *counts.entry(label).or_default() += 1;
            }
        }
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_by(|(a, ac), (b, bc)| bc.cmp(ac).then(a.cmp(b)));
        let filter = |name: String, label: Option<String>| {
            let selected = self.note_label == label;
            div()
                .id(gpui::SharedString::from(format!("label-{name}")))
                .cursor_pointer()
                .text_color(if selected { p.fg } else { p.faint })
                .when(selected, |el| el.font_weight(FontWeight::MEDIUM))
                .hover(|el| el.text_color(if selected { p.fg } else { p.muted }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.save_note(cx);
                    this.note_label = if this.note_label == label {
                        None
                    } else {
                        label.clone()
                    };
                    cx.notify();
                }))
                .child(name)
        };
        rows.push(
            div()
                .flex()
                .items_center()
                .h(px(28.))
                .text_size(px(13.))
                .gap(px(14.))
                .mb(px(12.))
                .when(!counts.is_empty(), |row| {
                    row.child(filter("All".into(), None)).children(
                        counts
                            .iter()
                            .map(|(name, _)| filter(format!("#{name}"), Some(name.clone()))),
                    )
                })
                .child(div().flex_1())
                .child(
                    div()
                        .id("archive-filter")
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .cursor_pointer()
                        .text_color(if self.note_archive { p.fg } else { p.faint })
                        .when(self.note_archive, |el| el.font_weight(FontWeight::MEDIUM))
                        .hover(|el| el.text_color(if self.note_archive { p.fg } else { p.muted }))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.save_note(cx);
                            this.note_archive = !this.note_archive;
                            this.note_label = None;
                            this.note_color_menu = None;
                            cx.notify();
                        }))
                        .child(
                            gpui_kit::component::Icon::default()
                                .path("icons/archive.svg")
                                .size(px(13.)),
                        )
                        .child("Archive"),
                )
                .into_any_element(),
        );
        if !self.note_archive {
            rows.push(composer.into_any_element());
        }
        let query = self.note_search.read(cx).value().to_lowercase();
        let mut notes: Vec<_> = view
            .iter()
            .copied()
            .filter(|note| {
                note.text.to_lowercase().contains(&query)
                    && self
                        .note_label
                        .as_ref()
                        .is_none_or(|label| labels(&note.text).contains(label))
            })
            .collect();
        notes.sort_by_key(|note| (!note.pinned, std::cmp::Reverse(note.created_at)));
        let mut previous = None;
        for (index, note) in notes.into_iter().enumerate() {
            let Some(at) = chrono::DateTime::from_timestamp_millis(note.created_at)
                .map(|at| at.with_timezone(&chrono::Local))
            else {
                continue;
            };
            let group = if note.pinned && !self.note_archive {
                "Pinned".to_owned()
            } else {
                super::home::day_label(at)
            };
            if previous.as_ref() != Some(&group) {
                rows.push(
                    div()
                        .pt(px(18.))
                        .pb(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_size(px(12.))
                        .when(note.pinned && !self.note_archive, |row| {
                            row.child(
                                gpui_kit::component::Icon::default()
                                    .path("icons/pin.svg")
                                    .size(px(12.))
                                    .text_color(p.faint),
                            )
                        })
                        .child(group.clone())
                        .into_any_element(),
                );
                previous = Some(group);
            }
            if let Some(editor) = self
                .note_editor
                .as_ref()
                .filter(|editor| editor.note.id == note.id)
            {
                rows.push(
                    div()
                        .id("note-editor")
                        .flex()
                        .items_start()
                        .gap(px(16.))
                        .px(px(7.))
                        .mx(px(-8.))
                        .py(px(7.))
                        .rounded(px(6.))
                        .bg(p.panel)
                        .border_1()
                        .border_color(p.border.blend(p.fg.opacity(0.28)))
                        .capture_action(cx.listener(|this, action: &Enter, _, cx| {
                            if action.shift {
                                cx.propagate();
                            } else {
                                this.save_note(cx);
                                cx.stop_propagation();
                            }
                        }))
                        .capture_action(cx.listener(|this, _: &Escape, _, cx| {
                            this.note_editor = None;
                            cx.notify();
                            cx.stop_propagation();
                        }))
                        .child(div().flex_1().min_w_0().child(editor.input.clone()))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child("Enter to save"),
                        )
                        .into_any_element(),
                );
                continue;
            }
            let hovered = self.hovered_note.as_deref() == Some(&note.id)
                || self.note_color_menu.as_deref() == Some(&note.id);
            let mut text = div().flex_1().min_w_0();
            let lines: Vec<_> = note.text.split('\n').collect();
            if lines.iter().any(|line| checklist_item(line).is_some()) {
                let mut shown = 0;
                let total = lines
                    .iter()
                    .filter(|line| checklist_item(line).is_some())
                    .count();
                for (line_index, line) in lines.iter().enumerate() {
                    if let Some((checked, item)) = checklist_item(line) {
                        shown += 1;
                        if shown > 8 {
                            continue;
                        }
                        let note = note.clone();
                        text = text.child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .id(("check", line_index))
                                        .cursor_pointer()
                                        .h(px(21.))
                                        .flex()
                                        .items_center()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            let mut note = note.clone();
                                            note.text = toggle_checklist(&note.text, line_index);
                                            this.put_note(note, cx);
                                        }))
                                        .child(
                                            gpui_kit::component::Icon::default()
                                                .path(if checked {
                                                    "icons/square-check.svg"
                                                } else {
                                                    "icons/square.svg"
                                                })
                                                .size(px(14.))
                                                .text_color(if checked {
                                                    p.muted
                                                } else {
                                                    p.faint
                                                }),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_color(if checked { p.muted } else { p.fg })
                                        .when(checked, |el| el.line_through())
                                        .child(hashtag_text(item, p)),
                                ),
                        );
                    } else {
                        text = text.child(
                            div()
                                .when(line_index == 0, |el| el.font_weight(FontWeight::MEDIUM))
                                .when(line_index > 0, |el| el.text_color(p.muted))
                                .child(hashtag_text(line, p)),
                        );
                    }
                }
                if total > 8 {
                    text = text.child(
                        div()
                            .text_color(p.faint)
                            .child(format!("+{} more", total - 8)),
                    );
                }
            } else if let Some((title, more)) = note.text.split_once('\n') {
                text = text
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(hashtag_text(title, p)),
                    )
                    .child(
                        div()
                            .text_color(p.muted)
                            .line_clamp(3)
                            .child(hashtag_text(more, p)),
                    );
            } else {
                text = text.child(hashtag_text(&note.text, p));
            }
            let actions = div()
                .flex()
                .w(px(138.))
                .flex_shrink_0()
                .justify_end()
                .gap(px(2.))
                .mt(px(-4.))
                .mb(px(-4.))
                .children(
                    [
                        (
                            "pin",
                            if note.pinned {
                                "icons/pin-filled.svg"
                            } else {
                                "icons/pin.svg"
                            },
                            if note.pinned { "Unpin" } else { "Pin" },
                        ),
                        ("color", "icons/palette.svg", "Color"),
                        (
                            "archive",
                            if note.archived {
                                "icons/archive-restore.svg"
                            } else {
                                "icons/archive.svg"
                            },
                            if note.archived {
                                "Unarchive"
                            } else {
                                "Archive"
                            },
                        ),
                        ("copy", "icons/copy.svg", "Copy"),
                        ("delete", "icons/trash.svg", "Delete"),
                    ]
                    .into_iter()
                    .filter(|(action, _, _)| *action != "pin" || !note.archived)
                    .map(|(action, icon, label)| {
                        let note = note.clone();
                        let action_note = note.clone();
                        let button = history_button(
                            (action, index),
                            icon,
                            label,
                            action == "pin" && note.pinned,
                            cx,
                        )
                        .size(px(26.))
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                let note = &action_note;
                                match action {
                                    "copy" => {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            note.text.clone(),
                                        ));
                                        this.show_toast("Copied", cx);
                                    }
                                    "pin" => {
                                        let mut note = note.clone();
                                        note.pinned = !note.pinned;
                                        this.put_note(note, cx);
                                    }
                                    "color" => {
                                        this.note_color_menu = Some(note.id.clone());
                                        this.focus.focus(window, cx);
                                        cx.notify();
                                    }
                                    "archive" => {
                                        let mut note = note.clone();
                                        note.archived = !note.archived;
                                        note.pinned = false;
                                        let message = if note.archived {
                                            "Archived"
                                        } else {
                                            "Restored"
                                        };
                                        if this.put_note(note, cx) {
                                            this.show_toast(message, cx);
                                        }
                                    }
                                    _ => {
                                        if this.state.update(cx, |s, cx| {
                                            let saved = s.change_note(Op::Delete(note.id.clone()));
                                            cx.notify();
                                            saved
                                        }) {
                                            this.show_toast("Deleted", cx);
                                        }
                                    }
                                }
                            },
                        ));
                        let mut wrapper = div().relative().child(button);
                        if action == "color" && self.note_color_menu.as_deref() == Some(&note.id) {
                            let swatches =
                                NoteColor::ALL.into_iter().enumerate().map(|(i, color)| {
                                    let note = note.clone();
                                    let selected = note.color == color;
                                    div()
                                        .id(("swatch", i))
                                        .size(px(18.))
                                        .rounded_full()
                                        .bg(p.note_tint(color))
                                        .border_1()
                                        .border_color(p.border)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            let mut note = note.clone();
                                            note.color = color;
                                            this.put_note(note, cx);
                                            this.note_color_menu = None;
                                            cx.notify();
                                        }))
                                        .when(selected, |el| {
                                            el.child(
                                                gpui_kit::component::Icon::default()
                                                    .path("icons/check.svg")
                                                    .size(px(12.))
                                                    .text_color(p.fg),
                                            )
                                        })
                                });
                            wrapper = wrapper.child(
                                gpui::deferred(
                                    gpui::anchored().offset(gpui::point(px(0.), px(4.))).child(
                                        div()
                                            .id("note-colors")
                                            .flex()
                                            .gap(px(6.))
                                            .p(px(6.))
                                            .rounded(px(8.))
                                            .bg(p.panel)
                                            .border_1()
                                            .border_color(p.border)
                                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                                this.note_color_menu = None;
                                                cx.notify();
                                            }))
                                            .children(swatches),
                                    ),
                                )
                                .with_priority(1),
                            );
                        }
                        wrapper
                    }),
                );
            let note = note.clone();
            let pinned = note.pinned;
            let id = note.id.clone();
            rows.push(
                div()
                    .id(("note", index))
                    .flex()
                    .items_start()
                    .gap(px(16.))
                    .px(px(8.))
                    .mx(px(-8.))
                    .py(px(8.))
                    .rounded(px(6.))
                    .bg(if hovered {
                        if note.color == NoteColor::None {
                            p.hover
                        } else {
                            p.note_tint(note.color).blend(p.hover.opacity(0.2))
                        }
                    } else {
                        p.note_tint(note.color)
                    })
                    .on_hover(cx.listener(move |this, hovered, _, cx| {
                        if *hovered {
                            this.hovered_note = Some(id.clone());
                        } else if this.hovered_note.as_ref() == Some(&id) {
                            this.hovered_note = None;
                        }
                        cx.notify();
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.edit_note(note.clone(), window, cx)
                    }))
                    .child(text)
                    .child(if hovered {
                        actions.into_any_element()
                    } else {
                        div()
                            .w(px(138.))
                            .flex_shrink_0()
                            .text_right()
                            .font_family(MONO)
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(
                                at.format(if pinned {
                                    "%b %d"
                                } else {
                                    super::clock::time_format()
                                })
                                .to_string(),
                            )
                            .into_any_element()
                    })
                    .into_any_element(),
            );
        }
        if view.is_empty() {
            rows.push(
                div()
                    .py(px(56.))
                    .text_center()
                    .text_color(p.muted)
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.fg)
                            .text_size(px(14.))
                            .mb(px(4.))
                            .child(if self.note_archive {
                                "No archived notes"
                            } else {
                                "No notes yet"
                            }),
                    )
                    .when(!self.note_archive, |el| {
                        el.child(
                            div()
                                .flex()
                                .justify_center()
                                .items_center()
                                .gap(px(5.))
                                .child("Press")
                                .child(kbd("LEFTSHIFT", p))
                                .child("twice anywhere to write one."),
                        )
                    })
                    .into_any_element(),
            );
        }
        page(
            LIST_WIDTH,
            header("Notes")
                .child(
                    div()
                        .ml(px(-8.))
                        .text_size(px(13.))
                        .text_color(p.faint)
                        .child(view.len().to_string()),
                )
                .child(div().flex_1())
                .child(
                    Input::new(&self.note_search)
                        .prefix(
                            gpui_kit::component::Icon::default()
                                .path("icons/search.svg")
                                .size(px(13.))
                                .text_color(p.faint),
                        )
                        .w(px(240.))
                        .min_h(px(30.))
                        .max_h(px(30.))
                        .text_size(px(13.))
                        .bg(p.bg),
                ),
            div()
                .id("notes-scroll")
                .child(column(LIST_WIDTH).pt(px(16.)).children(rows)),
            p,
        )
    }
}
