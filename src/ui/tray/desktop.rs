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
    let open = MenuItem::new("Open Hermes", true, None);
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
    let mut pixels = vec![0; 32 * 32 * 4];
    for (index, height) in [4, 11, 18, 11, 4].into_iter().enumerate() {
        for x in (7 + index * 4)..(9 + index * 4) {
            for y in (16 - height / 2)..(16 + height / 2) {
                pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4]
                    .copy_from_slice(&[214, 180, 124, 255]);
            }
        }
    }
    let icon = TrayIconBuilder::new()
        .with_tooltip("Hermes")
        .with_menu(Box::new(menu))
        .with_icon(tray_icon::Icon::from_rgba(pixels, 32, 32)?)
        .build()?;
    Ok(TrayHandle { _icon: icon })
}
