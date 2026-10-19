use super::{AppState, home::history_button, theme::*, window::MainWindow};
use gpui::{
    App, AppContext, ClipboardItem, Context, Entity, Focusable, FontWeight, IntoElement, Styled,
    Window, div, prelude::*, px,
};
use gpui_kit::component::input::{Enter, Escape, Input, InputEvent, TextareaState};
use typeaside::notes::{Note, Op};

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

impl MainWindow {
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
        rows.push(composer.into_any_element());
        let query = self.note_search.read(cx).value().to_lowercase();
        let mut notes: Vec<_> = state
            .notes
            .notes
            .iter()
            .filter(|note| note.text.to_lowercase().contains(&query))
            .collect();
        notes.sort_by_key(|note| (!note.pinned, std::cmp::Reverse(note.created_at)));
        let mut previous = None;
        for (index, note) in notes.into_iter().enumerate() {
            let Some(at) = chrono::DateTime::from_timestamp_millis(note.created_at)
                .map(|at| at.with_timezone(&chrono::Local))
            else {
                continue;
            };
            let group = if note.pinned {
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
                        .when(note.pinned, |row| {
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
            let hovered = self.hovered_note.as_deref() == Some(&note.id);
            let mut text = div().flex_1().min_w_0();
            if let Some((title, more)) = note.text.split_once('\n') {
                text = text
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(title.to_owned()),
                    )
                    .child(
                        div()
                            .text_color(p.muted)
                            .line_clamp(3)
                            .child(more.to_owned()),
                    );
            } else {
                text = text.child(note.text.clone());
            }
            let actions = div().flex().gap(px(2.)).mt(px(-4.)).mb(px(-4.)).children(
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
                    ("copy", "icons/copy.svg", "Copy"),
                    ("delete", "icons/trash.svg", "Delete"),
                ]
                .into_iter()
                .map(|(action, icon, label)| {
                    let note = note.clone();
                    history_button(
                        (action, index),
                        icon,
                        label,
                        action == "pin" && note.pinned,
                        cx,
                    )
                    .size(px(26.))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        match action {
                            "copy" => {
                                cx.write_to_clipboard(ClipboardItem::new_string(note.text.clone()));
                                this.show_toast("Copied", cx);
                            }
                            "pin" => {
                                let mut note = note.clone();
                                note.pinned = !note.pinned;
                                note.updated_at = chrono::Utc::now()
                                    .timestamp_millis()
                                    .max(note.updated_at + 1);
                                this.state.update(cx, |s, cx| {
                                    s.change_note(Op::Put(note));
                                    cx.notify();
                                });
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
                    }))
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
                    .when(hovered, |row| row.bg(p.hover))
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
                            .w(px(64.))
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
        if state.notes.notes.is_empty() {
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
                            .child("No notes yet"),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .items_center()
                            .gap(px(5.))
                            .child("Press")
                            .child(kbd("LEFTSHIFT", p))
                            .child("twice anywhere to write one."),
                    )
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
                        .child(state.notes.notes.len().to_string()),
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
