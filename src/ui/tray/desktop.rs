use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

pub enum Action {
    Open,
    Quit,
}

pub struct TrayHandle {
    _icon: TrayIcon,
}

pub fn spawn(sender: async_channel::Sender<Action>) -> anyhow::Result<TrayHandle> {
    let menu = Menu::new();
    let open = MenuItem::new("Open TypeAside", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append_items(&[&open, &quit])?;
    let open_id = open.id().clone();
    let quit_id = quit.id().clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = if event.id == open_id {
            Action::Open
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
            typeaside::icons::TRAY_RGBA.to_vec(),
            32,
            32,
        )?)
        .build()?;
    Ok(TrayHandle { _icon: icon })
}
