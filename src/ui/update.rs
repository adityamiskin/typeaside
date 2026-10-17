use super::{AppState, theme::*, window::MainWindow};
use gpui::{App, Context, Div, Entity, FontWeight, Stateful, Styled, div, prelude::*, px};
use typeaside::update::Progress;

#[derive(Clone)]
pub enum Toast {
    Available,
    Downloading(u8),
    Installing,
    Failed(String),
}

impl Toast {
    fn busy(&self) -> bool {
        matches!(self, Self::Downloading(_) | Self::Installing)
    }
}

impl MainWindow {
    pub fn update_toast(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let state = self.state.read(cx);
        let release = state.update.as_ref().filter(|_| !state.update_dismissed)?;
        let p = palette(cx);
        let title = match &state.update_status {
            Toast::Available => format!("TypeAside {} is available", release.version),
            Toast::Downloading(percent) => format!("Downloading {percent}%"),
            Toast::Installing => "Installing…".into(),
            Toast::Failed(reason) => format!("Update failed: {reason}"),
        };
        let managed = release.installer.is_none();
        let busy = state.update_status.busy();
        let retry = matches!(state.update_status, Toast::Failed(_));
        Some(
            div()
                .id("update-toast")
                .absolute()
                .bottom(px(16.))
                .right(px(16.))
                .w(px(340.))
                .p(px(16.))
                .rounded(px(RADIUS))
                .border_1()
                .border_color(p.border)
                .bg(p.bg)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if retry { p.red } else { p.fg })
                        .child(title),
                )
                .when(managed, |toast| {
                    toast.child(
                        div()
                            .text_color(p.muted)
                            .child("Update with your package manager"),
                    )
                })
                .when(!busy, |toast| {
                    toast.child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .when(!managed, |buttons| {
                                buttons.child(
                                    button(if retry { "Retry" } else { "Update now" }, true, p)
                                        .id("update-now")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| install(&this.state, cx)),
                                        ),
                                )
                            })
                            .child(button("Later", false, p).id("update-later").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.state.update(cx, |s, cx| {
                                        s.update_dismissed = true;
                                        cx.notify();
                                    });
                                }),
                            )),
                    )
                }),
        )
    }
}

pub fn schedule(state: &Entity<AppState>, cx: &mut App) {
    let state = state.downgrade();
    cx.spawn(async move |cx| {
        let mut delay = std::time::Duration::from_secs(10);
        loop {
            cx.background_executor().timer(delay).await;
            let result = cx
                .background_executor()
                .spawn(async { typeaside::update::check() })
                .await;
            let Some(state) = state.upgrade() else { break };
            if let Ok(release) = result {
                state.update(cx, |s, cx| {
                    if !s.update_dismissed && !s.update_status.busy() {
                        s.update = release;
                        s.update_status = Toast::Available;
                        cx.notify();
                    }
                });
            }
            delay = std::time::Duration::from_secs(6 * 60 * 60);
        }
    })
    .detach();
}

fn install(state: &Entity<AppState>, cx: &mut App) {
    let current = state.read(cx);
    if current.is_demo || current.update_status.busy() {
        return;
    }
    let Some(release) = current.update.clone() else {
        return;
    };
    state.update(cx, |s, cx| {
        s.update_status = Toast::Downloading(0);
        cx.notify();
    });
    let (tx, rx) = async_channel::unbounded();
    let task = cx.background_executor().spawn(async move {
        typeaside::update::install(&release, |progress| {
            let _ = tx.try_send(progress);
        })
    });
    let state = state.clone();
    cx.spawn(async move |cx| {
        while let Ok(progress) = rx.recv().await {
            state.update(cx, |s, cx| {
                s.update_status = match progress {
                    Progress::Downloading(percent) => Toast::Downloading(percent),
                    Progress::Installing => Toast::Installing,
                };
                cx.notify();
            });
        }
        match task.await {
            Ok(()) => typeaside::update::quit_after_install(),
            Err(error) => state.update(cx, |s, cx| {
                eprintln!("typeaside: update failed: {error:#}");
                s.update_status = Toast::Failed(error.to_string().chars().take(100).collect());
                cx.notify();
            }),
        }
    })
    .detach();
}
