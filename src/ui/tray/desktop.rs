use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

use super::Action;

pub struct TrayHandle {
    _icon: TrayIcon,
    paste: MenuItem,
}

impl TrayHandle {
    pub fn enable_paste(&self) {
        self.paste.set_enabled(true);
    }
}

pub fn spawn(
    sender: async_channel::Sender<Action>,
    paste_available: bool,
) -> anyhow::Result<TrayHandle> {
    let menu = Menu::new();
    let open = MenuItem::new("Open TypeAside", true, None);
    let paste = MenuItem::new("Paste last dictation", paste_available, None);
    let paste_id = paste.id().clone();
    let quit = MenuItem::new("Quit", true, None);
    menu.append_items(&[&open, &paste, &quit])?;
    let open_id = open.id().clone();
    let quit_id = quit.id().clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = if event.id == open_id {
            Action::Open
        } else if event.id == paste_id {
            Action::Command(crate::ui::instance::Command::PasteLast)
        } else if event.id == quit_id {
            Action::Quit
        } else {
            return;
        };
        let _ = sender.try_send(action);
    }));
    let icon = TrayIconBuilder::new()
        .with_tooltip("TypeAside")
        .with_menu(Box::new(menu))
        .with_icon(tray_icon::Icon::from_rgba(
            super::ICON_RGBA.to_vec(),
            32,
            32,
        )?)
        .build()?;
    Ok(TrayHandle { _icon: icon, paste })
}
