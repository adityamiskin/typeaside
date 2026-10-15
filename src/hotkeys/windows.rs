use super::desktop::{HotkeyEvent, State};
use crate::config::AppConfig;
use anyhow::{Context, Result, anyhow};
use std::sync::atomic::{AtomicU8, Ordering};
use std::{
    cell::RefCell,
    sync::mpsc,
    thread::{self, JoinHandle},
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::*,
};

const MODIFIERS: [u16; 8] = [0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0x5b, 0x5c];
static PHYSICAL_MODIFIERS: AtomicU8 = AtomicU8::new(0);

pub(crate) fn held_modifiers() -> impl Iterator<Item = u16> {
    let held = PHYSICAL_MODIFIERS.load(Ordering::Relaxed);
    MODIFIERS
        .into_iter()
        .enumerate()
        .filter_map(move |(index, key)| (held & (1 << index) != 0).then_some(key))
}

thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if !event.flags.contains(LLKHF_INJECTED) {
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            if matches!(
                wparam.0 as u32,
                WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP
            ) {
                if let Some(index) = MODIFIERS
                    .iter()
                    .position(|key| u32::from(*key) == event.vkCode)
                {
                    if down {
                        PHYSICAL_MODIFIERS.fetch_or(1 << index, Ordering::Relaxed);
                    } else {
                        PHYSICAL_MODIFIERS.fetch_and(!(1 << index), Ordering::Relaxed);
                    }
                }
                STATE.with_borrow_mut(|state| {
                    if let Some(state) = state {
                        state.key(event.vkCode as u16, down);
                    }
                });
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

pub struct HotkeyService {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl HotkeyService {
    pub fn start(config: &AppConfig, emit: impl Fn(HotkeyEvent) + Send + 'static) -> Result<Self> {
        let state = State::new(config, emit)?;
        let (ready, started) = mpsc::sync_channel(1);
        let thread =
            thread::Builder::new()
                .name("hermes-hotkeys".into())
                .spawn(move || unsafe {
                    let held = MODIFIERS.iter().enumerate().fold(0, |bits, (index, key)| {
                        if GetAsyncKeyState(i32::from(*key)) < 0 {
                            bits | (1 << index)
                        } else {
                            bits
                        }
                    });
                    PHYSICAL_MODIFIERS.store(held, Ordering::Relaxed);
                    STATE.with_borrow_mut(|slot| *slot = Some(state));
                    let mut message = MSG::default();
                    let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
                    let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), None, 0) {
                        Ok(hook) => hook,
                        Err(error) => {
                            let _ = ready.send(Err(error.to_string()));
                            return;
                        }
                    };
                    let _ = ready.send(Ok(GetCurrentThreadId()));
                    while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    let _ = UnhookWindowsHookEx(hook);
                    STATE.with_borrow_mut(|slot| *slot = None);
                })?;
        let thread_id = started
            .recv()
            .context("hotkey thread exited during startup")?
            .map_err(|error| anyhow!(error))?;
        Ok(Self {
            thread_id,
            thread: Some(thread),
        })
    }
}
impl Drop for HotkeyService {
    fn drop(&mut self) {
        let sent = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if sent.is_ok()
            && let Some(thread) = self.thread.take()
        {
            let _ = thread.join();
        }
    }
}
