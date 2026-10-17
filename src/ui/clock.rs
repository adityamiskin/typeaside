use std::sync::OnceLock;

pub fn time_format() -> &'static str {
    static FORMAT: OnceLock<&'static str> = OnceLock::new();
    FORMAT.get_or_init(|| {
        let twelve = match std::env::var("TYPEASIDE_DEMO_CLOCK").as_deref() {
            Ok("12") => true,
            Ok("24") => false,
            _ => system_uses_twelve_hours(),
        };
        if twelve { "%-I:%M %p" } else { "%H:%M" }
    })
}

#[cfg(target_os = "linux")]
fn system_uses_twelve_hours() -> bool {
    unsafe {
        let locale = libc::newlocale(libc::LC_TIME_MASK, c"".as_ptr(), std::ptr::null_mut());
        if locale.is_null() {
            return false;
        }
        let format = libc::nl_langinfo_l(libc::T_FMT, locale);
        let twelve = !format.is_null()
            && std::ffi::CStr::from_ptr(format)
                .to_bytes()
                .windows(2)
                .any(|part| matches!(part, b"%I" | b"%l" | b"%r"));
        libc::freelocale(locale);
        twelve
    }
}

#[cfg(target_os = "macos")]
fn system_uses_twelve_hours() -> bool {
    use core_foundation::{
        base::{CFAllocatorRef, CFIndex, CFRelease, CFTypeRef, TCFType},
        string::{CFString, CFStringRef},
    };
    unsafe extern "C" {
        fn CFLocaleCopyCurrent() -> CFTypeRef;
        fn CFDateFormatterCreate(
            allocator: CFAllocatorRef,
            locale: CFTypeRef,
            date_style: CFIndex,
            time_style: CFIndex,
        ) -> CFTypeRef;
        fn CFDateFormatterGetFormat(formatter: CFTypeRef) -> CFStringRef;
    }
    unsafe {
        let locale = CFLocaleCopyCurrent();
        if locale.is_null() {
            return false;
        }
        let formatter = CFDateFormatterCreate(std::ptr::null(), locale, 0, 1);
        CFRelease(locale);
        if formatter.is_null() {
            return false;
        }
        let format = CFDateFormatterGetFormat(formatter);
        let twelve = !format.is_null()
            && CFString::wrap_under_get_rule(format)
                .to_string()
                .contains(['h', 'K', 'a']);
        CFRelease(formatter);
        twelve
    }
}

#[cfg(target_os = "windows")]
fn system_uses_twelve_hours() -> bool {
    use windows::{
        Win32::Globalization::{GetLocaleInfoEx, LOCALE_STIMEFORMAT},
        core::PCWSTR,
    };
    unsafe {
        let length = GetLocaleInfoEx(PCWSTR::null(), LOCALE_STIMEFORMAT, None);
        if length <= 0 {
            return false;
        }
        let mut format = vec![0u16; length as usize];
        GetLocaleInfoEx(PCWSTR::null(), LOCALE_STIMEFORMAT, Some(&mut format)) > 0
            && format.contains(&(b'h' as u16))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn system_uses_twelve_hours() -> bool {
    false
}
