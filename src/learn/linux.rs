use atspi::{
    Role, State,
    proxy::{
        accessible::{AccessibleProxy, ObjectRefExt},
        bus::StatusProxy,
        proxy_ext::ProxyExt,
        text::TextProxy,
    },
    zbus::{self, proxy::CacheProperties},
};
use std::time::Duration;

/// Apps only join the accessibility bus when it is enabled, and only at launch.
pub(crate) fn enable() {
    let Some(runtime) = runtime() else { return };
    let enabled = runtime.block_on(async {
        let session = zbus::Connection::session().await?;
        StatusProxy::new(&session).await?.set_is_enabled(true).await
    });
    if let Err(error) = enabled {
        eprintln!("typeaside: cannot enable accessibility: {error}");
    }
}

pub(super) struct FocusedElement {
    runtime: tokio::runtime::Runtime,
    element: AccessibleProxy<'static>,
    text: TextProxy<'static>,
}

impl FocusedElement {
    pub fn capture() -> Option<Self> {
        let runtime = runtime()?;
        let (element, text) = runtime.block_on(async {
            tokio::time::timeout(Duration::from_millis(500), focused())
                .await
                .ok()?
        })?;
        Some(Self {
            runtime,
            element,
            text,
        })
    }

    pub fn is_focused(&self) -> bool {
        self.block(async { self.element.get_state().await.ok() })
            .is_some_and(|state| state.contains(State::Focused))
    }

    pub fn value(&self) -> Option<String> {
        self.block(async {
            let length = self.text.character_count().await.ok()?;
            self.text.get_text(0, length).await.ok()
        })
    }

    /// UTF-16 start and length.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let (value, (start, end)) = self.block(async {
            let length = self.text.character_count().await.ok()?;
            let value = self.text.get_text(0, length).await.ok()?;
            // atspi-proxies calls GetNselections, which Chromium rejects; an empty range means no selection.
            let range = match self.text.get_selection(0).await {
                Ok((start, end)) if start != end => (start, end),
                _ => {
                    let caret = self.text.caret_offset().await.ok()?;
                    (caret, caret)
                }
            };
            Some((value, range))
        })?;
        // AT-SPI offsets count characters, the shared code counts UTF-16 units.
        let utf16 = |offset: i32| {
            let offset = usize::try_from(offset).ok()?;
            (value.chars().count() >= offset)
                .then(|| value.chars().take(offset).map(char::len_utf16).sum())
        };
        let (start, end) = (utf16(start.min(end))?, utf16(start.max(end))?);
        Some((start, end - start))
    }

    fn block<T>(&self, future: impl Future<Output = Option<T>>) -> Option<T> {
        self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_millis(500), future)
                .await
                .ok()?
        })
    }
}

fn runtime() -> Option<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .ok()
}

async fn focused() -> Option<(AccessibleProxy<'static>, TextProxy<'static>)> {
    let connection = crate::text_injector::atspi_connection().await.ok()?;
    let registry = AccessibleProxy::builder(connection)
        .destination("org.a11y.atspi.Registry")
        .ok()?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .ok()?;
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
                return editable(connection, window).await;
            }
        }
    }
    None
}

async fn editable(
    connection: &'static zbus::Connection,
    window: AccessibleProxy<'static>,
) -> Option<(AccessibleProxy<'static>, TextProxy<'static>)> {
    // Collection would be one call, but Chromium answers it with nothing once unlocked.
    let element = walk(connection, window).await?;
    if element.get_role().await.ok()? == Role::PasswordText {
        return None;
    }
    let text = element.proxies().await.ok()?.text().await.ok()?;
    Some((element, text))
}

async fn walk(
    connection: &'static zbus::Connection,
    window: AccessibleProxy<'static>,
) -> Option<AccessibleProxy<'static>> {
    let mut stack = vec![window];
    for _ in 0..2000 {
        let node = stack.pop()?;
        let Ok(state) = node.get_state().await else {
            continue;
        };
        // Chromium also marks the focused field's document as focused.
        if state.contains(State::Focused) && state.contains(State::Editable) {
            return Some(node);
        }
        if !state.contains(State::Showing) {
            continue;
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
