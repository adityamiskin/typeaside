use super::super::AppState;
use gpui::{App, Entity};
use std::{fs::OpenOptions, process::Command};

pub fn statuses() -> [bool; 2] {
    [true, keyboard_access()]
}

fn keyboard_access() -> bool {
    OpenOptions::new().write(true).open("/dev/uinput").is_ok()
        && std::fs::read_dir("/dev/input").is_ok_and(|entries| {
            entries.filter_map(Result::ok).any(|entry| {
                entry.file_name().as_encoded_bytes().starts_with(b"event")
                    && std::fs::File::open(entry.path()).is_ok()
            })
        })
}

pub fn request_one(index: usize) {
    if index != 1 {
        return;
    }
    std::thread::spawn(|| {
        let result = Command::new("pkexec")
            .args([
                "sh",
                "-c",
                r#"set -e
mkdir -p /etc/udev/rules.d /etc/modules-load.d
printf %s "$1" > /etc/udev/rules.d/70-typeaside.rules
printf 'uinput\n' > /etc/modules-load.d/typeaside.conf
modprobe uinput
udevadm control --reload-rules
udevadm trigger --subsystem-match=input --subsystem-match=misc --action=change"#,
                "sh",
                include_str!("../../../packaging/linux/70-typeaside.rules"),
            ])
            .status();
        match result {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!("typeaside: keyboard access request failed: {status}"),
            Err(error) => eprintln!("typeaside: cannot request keyboard access: {error}"),
        }
    });
}

pub fn allow(index: usize, _state: &Entity<AppState>, _cx: &mut App) {
    request_one(index);
}

pub fn notice() -> Option<String> {
    (!keyboard_access())
        .then(|| "Allow keyboard access so TypeAside can hear your shortcut and type.".into())
}

pub fn help(_state: &AppState) -> Option<&'static str> {
    None
}

pub fn request() {
    if let Some(message) = notice() {
        eprintln!("typeaside: {message}");
    }
}
