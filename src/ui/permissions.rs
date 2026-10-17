use super::AppState;
use core_foundation::{
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    bundle::{CFBundleGetMainBundle, CFBundleGetValueForInfoDictionaryKey},
    dictionary::{CFDictionary, CFDictionaryRef},
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use gpui::{App, Entity};
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};
use std::time::Duration;
use typeaside::engine::Command;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFPreferencesCopyAppValue(key: CFStringRef, application: CFStringRef) -> CFTypeRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

pub fn request() {
    if !statuses()[1] {
        prompt_accessibility();
    }
    unsafe {
        if AVMediaTypeAudio.is_some_and(|audio| {
            AVCaptureDevice::authorizationStatusForMediaType(audio)
                == AVAuthorizationStatus::NotDetermined
        }) {
            request_one(0);
        }
    }
    if let Some(message) = notice() {
        eprintln!("typeaside: {message}");
    }
}

pub fn statuses() -> [bool; 2] {
    [
        unsafe {
            AVMediaTypeAudio.is_some_and(|audio| {
                AVCaptureDevice::authorizationStatusForMediaType(audio)
                    == AVAuthorizationStatus::Authorized
            })
        },
        unsafe { AXIsProcessTrusted() },
    ]
}

pub fn request_one(index: usize) {
    match index {
        0 => unsafe {
            if bundled_microphone_description()
                && let Some(audio) = AVMediaTypeAudio
                && AVCaptureDevice::authorizationStatusForMediaType(audio)
                    == AVAuthorizationStatus::NotDetermined
            {
                let completion = block2::RcBlock::new(|_: objc2::runtime::Bool| {});
                AVCaptureDevice::requestAccessForMediaType_completionHandler(audio, &completion);
            } else if !statuses()[0] {
                let _ = typeaside::account::open(
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
                );
            }
        },
        1 => {
            // The prompt lists TypeAside in Settings but only appears once, so also open the pane.
            prompt_accessibility();
            let _ = typeaside::account::open(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            );
        }
        _ => {}
    }
}

fn prompt_accessibility() {
    let options = CFDictionary::from_CFType_pairs(&[(
        CFString::new("AXTrustedCheckOptionPrompt"),
        CFBoolean::true_value(),
    )]);
    unsafe {
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}

/// Opens the Settings pane for the first missing permission.
pub fn open_missing() {
    if let Some(index) = statuses().into_iter().position(|granted| !granted) {
        request_one(index);
    }
}

fn bundled_microphone_description() -> bool {
    let key = CFString::new("NSMicrophoneUsageDescription");
    unsafe {
        let bundle = CFBundleGetMainBundle();
        !bundle.is_null()
            && !CFBundleGetValueForInfoDictionaryKey(bundle, key.as_concrete_TypeRef()).is_null()
    }
}

pub fn notice() -> Option<String> {
    if !bundled_microphone_description() {
        return Some(
            "Launch TypeAside.app built with scripts/bundle-macos.sh to enable microphone access."
                .into(),
        );
    }
    let accessibility = !unsafe { AXIsProcessTrusted() };
    let microphone = unsafe {
        AVMediaTypeAudio.is_none_or(|audio| {
            AVCaptureDevice::authorizationStatusForMediaType(audio)
                != AVAuthorizationStatus::Authorized
        })
    };
    let missing = [(accessibility, "Accessibility"), (microphone, "Microphone")]
        .into_iter()
        .filter_map(|(missing, name)| missing.then_some(name))
        .collect::<Vec<_>>();
    (!missing.is_empty()).then(|| {
        format!(
            "Allow {} for TypeAside in System Settings > Privacy & Security.",
            missing.join(", ")
        )
    })
}

/// Reloads the engine as grants change, so new permissions apply without a restart.
pub fn watch(state: &Entity<AppState>, cx: &mut App) {
    let mut last = statuses();
    if last.into_iter().all(|granted| granted) {
        return;
    }
    let state = state.downgrade();
    cx.spawn(async move |cx| {
        while !last.into_iter().all(|granted| granted) {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let now = statuses();
            if now == last {
                continue;
            }
            last = now;
            let updated = state.update(cx, |s, cx| {
                s.permission_notice = notice();
                s.command(Command::Reload(Box::new(s.config.clone())));
                cx.notify();
            });
            if updated.is_err() {
                break;
            }
        }
    })
    .detach();
}

/// Whether the system still claims the Fn key for emoji, dictation, or input switching.
pub fn fn_key_claimed() -> bool {
    let value = unsafe {
        CFPreferencesCopyAppValue(
            CFString::new("AppleFnUsageType").as_concrete_TypeRef(),
            CFString::new("com.apple.HIToolbox").as_concrete_TypeRef(),
        )
    };
    if value.is_null() {
        // Unset means the macOS default, which shows the emoji picker.
        return true;
    }
    let value = unsafe { CFType::wrap_under_create_rule(value) };
    value
        .downcast::<CFNumber>()
        .and_then(|number| number.to_i64())
        .is_none_or(|usage| usage != 0)
}
