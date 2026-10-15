use ksni::blocking::{Handle, TrayMethods};
use ksni::{Tray, menu::StandardItem};

pub enum Action {
    Open,
    Quit,
}
struct HermesTray {
    sender: async_channel::Sender<Action>,
}

impl Tray for HermesTray {
    fn id(&self) -> String {
        "hermes".into()
    }
    fn title(&self) -> String {
        "Hermes".into()
    }
    fn icon_name(&self) -> String {
        "hermes".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let mut pixels = vec![0; 32 * 32 * 4];
        for (index, height) in [4, 11, 18, 11, 4].into_iter().enumerate() {
            for x in (7 + index * 4)..(9 + index * 4) {
                for y in (16 - height / 2)..(16 + height / 2) {
                    pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4]
                        .copy_from_slice(&[255, 214, 180, 124]);
                }
            }
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
                label: "Open Hermes".into(),
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
    _handle: Handle<HermesTray>,
}

pub fn spawn() -> anyhow::Result<(TrayHandle, async_channel::Receiver<Action>)> {
    let (sender, receiver) = async_channel::unbounded();
    let handle = HermesTray { sender }.spawn()?;
    Ok((TrayHandle { _handle: handle }, receiver))
}
