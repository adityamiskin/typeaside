use ksni::blocking::{Handle, TrayMethods};
use ksni::{Tray, menu::StandardItem};

use super::Action;
struct TypeAsideTray {
    sender: async_channel::Sender<Action>,
    paste_available: bool,
}

impl Tray for TypeAsideTray {
    fn id(&self) -> String {
        "typeaside".into()
    }
    fn title(&self) -> String {
        "TypeAside".into()
    }
    fn icon_name(&self) -> String {
        "typeaside".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let mut pixels = super::ICON_RGBA.to_vec();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.rotate_right(1);
        }
        vec![ksni::Icon {
            width: 32,
            height: 32,
            data: pixels,
        }]
    }
    fn activate(&mut self, _: i32, _: i32) {
        let _ = self.sender.try_send(Action::Open);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Open TypeAside".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.sender.try_send(Action::Open);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Paste last dictation".into(),
                enabled: self.paste_available,
                activate: Box::new(|this: &mut Self| {
                    let _ = this
                        .sender
                        .try_send(Action::Command(crate::ui::instance::Command::PasteLast));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.sender.try_send(Action::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub struct TrayHandle {
    _handle: Handle<TypeAsideTray>,
}

impl TrayHandle {
    pub fn enable_paste(&self) {
        self._handle.update(|tray| tray.paste_available = true);
    }
}

pub fn spawn(
    sender: async_channel::Sender<Action>,
    paste_available: bool,
) -> anyhow::Result<TrayHandle> {
    let handle = TypeAsideTray {
        sender,
        paste_available,
    }
    .spawn()?;
    Ok(TrayHandle { _handle: handle })
}
