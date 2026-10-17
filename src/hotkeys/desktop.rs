use crate::config::AppConfig;
use anyhow::{Result, bail};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub enum HotkeyEvent {
    Main(bool),
    Cancel(bool),
    Command(bool),
    Translate(bool),
    PasteLast(bool),
    /// A key pressed while Fn is held: Fn+Left is Home, not dictation.
    #[cfg(target_os = "macos")]
    Interrupt,
}

struct Combo {
    keys: Vec<Vec<u16>>,
    active: bool,
    event: fn(bool) -> HotkeyEvent,
}

pub(super) struct State {
    combos: Vec<Combo>,
    pressed: HashSet<u16>,
    emit: Box<dyn Fn(HotkeyEvent) + Send>,
}

impl State {
    pub(super) fn new(
        config: &AppConfig,
        emit: impl Fn(HotkeyEvent) + Send + 'static,
    ) -> Result<Self> {
        let mut combos = Vec::new();
        for (value, event) in [
            (
                Some(&config.hotkey),
                HotkeyEvent::Main as fn(bool) -> HotkeyEvent,
            ),
            (config.cancel_hotkey.as_ref(), HotkeyEvent::Cancel),
            (config.command_hotkey.as_ref(), HotkeyEvent::Command),
            (config.translate_hotkey.as_ref(), HotkeyEvent::Translate),
            (config.paste_last_hotkey.as_ref(), HotkeyEvent::PasteLast),
        ] {
            if let Some(value) = value {
                let keys = value
                    .split('+')
                    .map(|part| {
                        let upper = part.trim().to_ascii_uppercase();
                        let name = upper.strip_prefix("KEY_").unwrap_or(&upper);
                        let names: &[&str] = match name {
                            "CTRL" | "CONTROL" => &["LEFTCTRL", "RIGHTCTRL"],
                            "ALT" => &["LEFTALT", "RIGHTALT"],
                            "SUPER" | "META" | "CMD" | "COMMAND" => &["LEFTMETA", "RIGHTMETA"],
                            "SHIFT" => &["LEFTSHIFT", "RIGHTSHIFT"],
                            "ESC" => &["ESCAPE"],
                            "RETURN" => &["ENTER"],
                            _ => &[name],
                        };
                        names
                            .iter()
                            .map(|name| key_code(name))
                            .collect::<Result<Vec<_>>>()
                    })
                    .collect::<Result<Vec<_>>>()?;
                combos.push(Combo {
                    keys,
                    active: false,
                    event,
                });
            }
        }
        Ok(Self {
            combos,
            pressed: HashSet::new(),
            emit: Box::new(emit),
        })
    }

    #[cfg(target_os = "macos")]
    pub(super) fn interrupt_fn(&self) {
        if self
            .combos
            .iter()
            .any(|combo| combo.active && combo.keys == [vec![63]])
        {
            (self.emit)(HotkeyEvent::Interrupt);
        }
    }

    pub(super) fn key(&mut self, code: u16, down: bool) {
        if down {
            self.pressed.insert(code);
        } else {
            self.pressed.remove(&code);
        }
        for combo in &mut self.combos {
            let active = combo
                .keys
                .iter()
                .all(|group| group.iter().any(|key| self.pressed.contains(key)));
            if active != combo.active {
                combo.active = active;
                (self.emit)((combo.event)(active));
            }
        }
    }
}

fn key_code(name: &str) -> Result<u16> {
    let (mac, windows) = match name {
        "A" => (0, 0x41),
        "S" => (1, 0x53),
        "D" => (2, 0x44),
        "F" => (3, 0x46),
        "H" => (4, 0x48),
        "G" => (5, 0x47),
        "Z" => (6, 0x5a),
        "X" => (7, 0x58),
        "C" => (8, 0x43),
        "V" => (9, 0x56),
        "B" => (11, 0x42),
        "Q" => (12, 0x51),
        "W" => (13, 0x57),
        "E" => (14, 0x45),
        "R" => (15, 0x52),
        "Y" => (16, 0x59),
        "T" => (17, 0x54),
        "1" => (18, 0x31),
        "2" => (19, 0x32),
        "3" => (20, 0x33),
        "4" => (21, 0x34),
        "6" => (22, 0x36),
        "5" => (23, 0x35),
        "EQUAL" => (24, 0xbb),
        "9" => (25, 0x39),
        "7" => (26, 0x37),
        "MINUS" => (27, 0xbd),
        "8" => (28, 0x38),
        "0" => (29, 0x30),
        "RIGHTBRACE" => (30, 0xdd),
        "O" => (31, 0x4f),
        "U" => (32, 0x55),
        "LEFTBRACE" => (33, 0xdb),
        "I" => (34, 0x49),
        "P" => (35, 0x50),
        "ENTER" => (36, 0x0d),
        "L" => (37, 0x4c),
        "J" => (38, 0x4a),
        "APOSTROPHE" => (39, 0xde),
        "K" => (40, 0x4b),
        "SEMICOLON" => (41, 0xba),
        "BACKSLASH" => (42, 0xdc),
        "COMMA" => (43, 0xbc),
        "SLASH" => (44, 0xbf),
        "N" => (45, 0x4e),
        "M" => (46, 0x4d),
        "DOT" => (47, 0xbe),
        "TAB" => (48, 9),
        "SPACE" => (49, 0x20),
        "GRAVE" => (50, 0xc0),
        "BACKSPACE" => (51, 8),
        "ESCAPE" | "ESC" => (53, 0x1b),
        "RIGHTMETA" => (54, 0x5c),
        "LEFTMETA" => (55, 0x5b),
        "LEFTSHIFT" => (56, 0xa0),
        "CAPSLOCK" => (57, 0x14),
        "LEFTALT" => (58, 0xa4),
        "LEFTCTRL" => (59, 0xa2),
        "RIGHTSHIFT" => (60, 0xa1),
        "RIGHTALT" => (61, 0xa5),
        "RIGHTCTRL" => (62, 0xa3),
        "FN" if cfg!(target_os = "macos") => (63, 0),
        "F1" => (122, 0x70),
        "F2" => (120, 0x71),
        "F3" => (99, 0x72),
        "F4" => (118, 0x73),
        "F5" => (96, 0x74),
        "F6" => (97, 0x75),
        "F7" => (98, 0x76),
        "F8" => (100, 0x77),
        "F9" => (101, 0x78),
        "F10" => (109, 0x79),
        "F11" => (103, 0x7a),
        "F12" => (111, 0x7b),
        "F13" => (105, 0x7c),
        "F14" => (107, 0x7d),
        "F15" => (113, 0x7e),
        "F16" => (106, 0x7f),
        "F17" => (64, 0x80),
        "F18" => (79, 0x81),
        "F19" => (80, 0x82),
        "F20" => (90, 0x83),
        "HOME" => (115, 0x24),
        "PAGEUP" => (116, 0x21),
        "DELETE" => (117, 0x2e),
        "END" => (119, 0x23),
        "PAGEDOWN" => (121, 0x22),
        "LEFT" => (123, 0x25),
        "RIGHT" => (124, 0x27),
        "DOWN" => (125, 0x28),
        "UP" => (126, 0x26),
        "INSERT" => (114, 0x2d),
        "KP0" => (82, 0x60),
        "KP1" => (83, 0x61),
        "KP2" => (84, 0x62),
        "KP3" => (85, 0x63),
        "KP4" => (86, 0x64),
        "KP5" => (87, 0x65),
        "KP6" => (88, 0x66),
        "KP7" => (89, 0x67),
        "KP8" => (91, 0x68),
        "KP9" => (92, 0x69),
        "KPDOT" => (65, 0x6e),
        "KPMINUS" => (78, 0x6d),
        "KPPLUS" => (69, 0x6b),
        "KPASTERISK" => (67, 0x6a),
        "KPSLASH" => (75, 0x6f),
        "KPENTER" => (76, 0x0d),
        _ => bail!("unsupported hotkey key: {name}"),
    };
    Ok(if cfg!(target_os = "macos") {
        mac
    } else {
        windows
    })
}
