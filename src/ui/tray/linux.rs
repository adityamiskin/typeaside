use ksni::blocking::{Handle, TrayMethods};
use ksni::{Tray, menu::StandardItem};

pub enum Action {
    Open,
    Quit,
}
struct TypeAsideTray {
    sender: async_channel::Sender<Action>,
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
        let mut pixels = typeaside::icons::TRAY_RGBA.to_vec();
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

pub fn spawn(sender: async_channel::Sender<Action>) -> anyhow::Result<TrayHandle> {
    let handle = TypeAsideTray { sender }.spawn()?;
    Ok(TrayHandle { _handle: handle })
}
