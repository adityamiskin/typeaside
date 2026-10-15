use super::desktop::{HotkeyEvent, State};
use crate::config::AppConfig;
use anyhow::{Context, Result, anyhow};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopDefaultMode};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField,
};
use std::{
    cell::{Cell, RefCell},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
}

pub struct HotkeyService {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HotkeyService {
    pub fn start(config: &AppConfig, emit: impl Fn(HotkeyEvent) + Send + 'static) -> Result<Self> {
        let state = RefCell::new(State::new(config, emit)?);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let (ready, started) = mpsc::sync_channel(1);
        let thread = thread::Builder::new().name("hermes-hotkeys".into()).spawn(move || {
            let disabled = Cell::new(false);
            let tap = CGEventTap::new(CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsertEventTap, CGEventTapOptions::ListenOnly,
                vec![CGEventType::KeyDown, CGEventType::KeyUp, CGEventType::FlagsChanged],
                |_, kind, event| {
                    if matches!(kind, CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput) {
                        disabled.set(true);
                        return None;
                    }
                    if event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA)
                        == crate::text_injector::INJECTED_EVENT_TAG
                    { return None; }
                    let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
                    if matches!(kind, CGEventType::FlagsChanged) {
                        for modifier in [54, 55, 56, 58, 59, 60, 61, 62] {
                            state.borrow_mut().key(modifier, unsafe { CGEventSourceKeyState(0, modifier) });
                        }
                    } else {
                        state.borrow_mut().key(code, matches!(kind, CGEventType::KeyDown));
                    }
                    None
                });
            let Ok(tap) = tap else {
                eprintln!("hermes: global hotkeys unavailable; enable Accessibility and Input Monitoring in System Settings > Privacy & Security, then restart Hermes");
                let _ = ready.send(Err("global hotkeys unavailable: grant Hermes Accessibility and Input Monitoring permission in System Settings, then restart Hermes"));
                return;
            };
            let Ok(source) = tap.mach_port.create_runloop_source(0) else {
                let _ = ready.send(Err("cannot create hotkey run loop source"));
                return;
            };
            let runloop = CFRunLoop::get_current();
            unsafe { runloop.add_source(&source, kCFRunLoopDefaultMode); }
            tap.enable();
            let _ = ready.send(Ok(()));
            while !stopping.load(Ordering::Relaxed) {
                unsafe { CFRunLoop::run_in_mode(kCFRunLoopDefaultMode, Duration::from_millis(100), false); }
                if disabled.replace(false) { tap.enable(); }
            }
            unsafe { runloop.remove_source(&source, kCFRunLoopDefaultMode); }
        })?;
        started
            .recv()
            .context("hotkey thread exited during startup")?
            .map_err(|error| anyhow!(error))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
