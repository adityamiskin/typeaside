use super::{Editability, Focus};
use atspi::{
    Role, State,
    proxy::{
        accessible::{AccessibleProxy, ObjectRefExt},
        proxy_ext::ProxyExt,
    },
    zbus::proxy::CacheProperties,
};
use std::time::Duration;

pub(super) fn capture() -> Option<Focus> {
    let compositor = crate::text_injector::compositor_focused_window();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .ok()?;
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(60), focused(compositor))
            .await
            .ok()?
    })
}

async fn focused(compositor: Option<Option<crate::text_injector::FocusedApp>>) -> Option<Focus> {
    let connection = crate::text_injector::atspi_connection().await.ok()?;
    let registry = AccessibleProxy::builder(connection)
        .destination("org.a11y.atspi.Registry")
        .ok()?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .ok()?;
    let mut active = Vec::new();
    for application in registry.get_children().await.ok()? {
        let Ok(application) = application.into_accessible_proxy(connection).await else {
            continue;
        };
        // Chromium hides web content until an assistive tool asks for attributes.
        let _ = application.get_attributes().await;
        for window in application.get_children().await.unwrap_or_default() {
            let Ok(window) = window.into_accessible_proxy(connection).await else {
                continue;
            };
            if window
                .get_state()
                .await
                .is_ok_and(|state| state.contains(State::Active))
            {
                active.push(window);
            }
        }
    }
    // Wayland Chromium keeps Active after losing focus, so only trust a window the compositor agrees on.
    let [window] = <[_; 1]>::try_from(active).ok()?;
    if let Some(focused) = compositor {
        let title = focused?.title?;
        if window.name().await.ok()? != title {
            return None;
        }
    }
    let mut stack = vec![window];
    let mut button = None;
    for _ in 0..2000 {
        // A focused editable anywhere in the window wins over a focused button.
        let Some(node) = stack.pop() else {
            return button;
        };
        let Ok(state) = node.get_state().await else {
            continue;
        };
        if state.contains(State::Focused) && state.contains(State::Editable) {
            return Some(editable(&node).await);
        }
        if !state.contains(State::Showing) {
            continue;
        }
        if state.contains(State::Focused)
            && node.get_role().await.is_ok_and(|role| {
                matches!(
                    role,
                    Role::Button
                        | Role::ToggleButton
                        | Role::CheckBox
                        | Role::RadioButton
                        | Role::MenuItem
                        | Role::CheckMenuItem
                        | Role::RadioMenuItem
                        | Role::Slider
                        | Role::ScrollBar
                )
            })
        {
            button = Some(Focus {
                editability: Editability::NotEditable,
                ..Focus::default()
            });
        }
        for child in node
            .get_children()
            .await
            .unwrap_or_default()
            .into_iter()
            .rev()
        {
            if let Ok(child) = child.into_accessible_proxy(connection).await {
                stack.push(child);
            }
        }
    }
    None
}

async fn editable(node: &AccessibleProxy<'static>) -> Focus {
    let mut focus = Focus {
        editability: Editability::Editable,
        identity: format!("{}{}", node.inner().destination(), node.inner().path()),
        ..Focus::default()
    };
    if node
        .get_role()
        .await
        .is_ok_and(|role| role == Role::PasswordText)
    {
        return focus;
    }
    if let Ok(proxies) = node.proxies().await
        && let Ok(text) = proxies.text().await
        && let Ok(length) = text.character_count().await
        && length <= 65536
    {
        focus.value = text.get_text(0, length).await.ok();
        focus.caret = text.caret_offset().await.ok();
    }
    focus
}
