//! macOS runs the Globe key's own action (emoji picker, dictation, input switching) ahead of
//! every event tap, so while Fn dictates we hold "Press Globe key to" at Do Nothing.
//! `TISUpdateFnUsageType` is what System Settings calls; writing the preference directly
//! only applies after the next login. Text input source calls must stay on the main thread.

use core_foundation::{
    base::{CFRelease, CFTypeRef, TCFType},
    string::{CFString, CFStringRef},
};
use std::{
    ffi::{c_char, c_int, c_void},
    path::PathBuf,
    sync::{Mutex, OnceLock, PoisonError},
};

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFPreferencesCurrentUser: CFStringRef;
    static kCFPreferencesAnyHost: CFStringRef;
    fn CFPreferencesCopyValue(
        key: CFStringRef,
        application: CFStringRef,
        user: CFStringRef,
        host: CFStringRef,
    ) -> CFTypeRef;
    fn CFPreferencesSetValue(
        key: CFStringRef,
        value: CFTypeRef,
        application: CFStringRef,
        user: CFStringRef,
        host: CFStringRef,
    );
    fn CFPreferencesSynchronize(
        application: CFStringRef,
        user: CFStringRef,
        host: CFStringRef,
    ) -> bool;
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const DO_NOTHING: i32 = 0;

/// The user's own setting, and whether it was stored or just the macOS default.
#[derive(Clone, Copy)]
struct Original {
    value: i32,
    stored: bool,
}

static OWNED: Mutex<Option<Original>> = Mutex::new(None);

type GetUsage = unsafe extern "C" fn() -> i32;
type UpdateUsage = unsafe extern "C" fn(i32);

// Private API: resolved at runtime so a macOS without it still launches.
fn usage() -> Option<(GetUsage, UpdateUsage)> {
    static USAGE: OnceLock<Option<(GetUsage, UpdateUsage)>> = OnceLock::new();
    *USAGE.get_or_init(|| unsafe {
        let carbon = dlopen(
            c"/System/Library/Frameworks/Carbon.framework/Carbon".as_ptr(),
            1,
        );
        if carbon.is_null() {
            return None;
        }
        let get = dlsym(carbon, c"TISGetFnUsageType".as_ptr());
        let update = dlsym(carbon, c"TISUpdateFnUsageType".as_ptr());
        (!get.is_null() && !update.is_null()).then(|| {
            (
                std::mem::transmute::<*mut c_void, GetUsage>(get),
                std::mem::transmute::<*mut c_void, UpdateUsage>(update),
            )
        })
    })
}

/// Holds the Globe key exactly while a shortcut uses Fn.
pub fn follow(config: &crate::config::AppConfig) {
    let bound = [
        Some(&config.hotkey),
        config.command_hotkey.as_ref(),
        config.translate_hotkey.as_ref(),
        config.pause_hotkey.as_ref(),
    ]
    .into_iter()
    .flatten()
    .flat_map(|combo| combo.split('+'))
    .any(|key| key.trim().eq_ignore_ascii_case("FN"));
    hold(bound);
}

/// Holds the Globe key while `claim` is true and gives the user's setting back otherwise.
pub fn hold(claim: bool) {
    let Some((get, update)) = usage() else {
        return;
    };
    let mut owned = OWNED.lock().unwrap_or_else(PoisonError::into_inner);
    // A run that crashed while holding the key left its user's setting on disk.
    if owned.is_none()
        && let Some(saved) = read_marker()
    {
        if unsafe { get() } == DO_NOTHING {
            *owned = Some(saved);
        } else {
            remove_marker();
        }
    }
    if claim {
        let value = unsafe { get() };
        if owned.is_some() || value == DO_NOTHING {
            return;
        }
        let original = Original {
            value,
            stored: stored(),
        };
        if write_marker(original) {
            *owned = Some(original);
            unsafe { update(DO_NOTHING) };
        }
    } else if let Some(original) = owned.take() {
        // A setting the user picked while we held the key wins.
        if unsafe { get() } == DO_NOTHING {
            unsafe { update(original.value) };
            if !original.stored {
                clear();
            }
        }
        remove_marker();
    }
}

fn preference<T>(apply: impl FnOnce(CFStringRef, CFStringRef, CFStringRef, CFStringRef) -> T) -> T {
    let key = CFString::new("AppleFnUsageType");
    let domain = CFString::new("com.apple.HIToolbox");
    unsafe {
        apply(
            key.as_concrete_TypeRef(),
            domain.as_concrete_TypeRef(),
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        )
    }
}

fn stored() -> bool {
    preference(|key, domain, user, host| unsafe {
        CFPreferencesSynchronize(domain, user, host);
        let value = CFPreferencesCopyValue(key, domain, user, host);
        if value.is_null() {
            return false;
        }
        CFRelease(value);
        true
    })
}

// An unset value tracks the keyboard and input sources, so restore it as unset.
fn clear() {
    preference(|key, domain, user, host| unsafe {
        CFPreferencesSetValue(key, std::ptr::null(), domain, user, host);
        CFPreferencesSynchronize(domain, user, host);
    });
}

fn marker() -> Option<PathBuf> {
    crate::paths::AppPaths::discover()
        .ok()
        .map(|paths| paths.data_dir.join("globe-key"))
}

fn read_marker() -> Option<Original> {
    let text = std::fs::read_to_string(marker()?).ok()?;
    let (value, stored) = text.trim().split_once(' ')?;
    Some(Original {
        value: value.parse().ok()?,
        stored: stored == "stored",
    })
}

fn write_marker(original: Original) -> bool {
    let Some(path) = marker() else {
        return false;
    };
    let stored = if original.stored { "stored" } else { "default" };
    path.parent()
        .is_some_and(|dir| std::fs::create_dir_all(dir).is_ok())
        && std::fs::write(path, format!("{} {stored}\n", original.value)).is_ok()
}

fn remove_marker() {
    if let Some(path) = marker() {
        let _ = std::fs::remove_file(path);
    }
}
