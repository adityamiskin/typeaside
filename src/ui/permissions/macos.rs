use super::super::AppState;
use core_foundation::{
    base::TCFType,
    boolean::CFBoolean,
    bundle::{CFBundleGetMainBundle, CFBundleGetValueForInfoDictionaryKey},
    dictionary::{CFDictionary, CFDictionaryRef},
    string::CFString,
};
use gpui::{App, Entity};
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};
use std::time::Duration;

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

/// Opens the Settings pane for a permission, then offers the ad-hoc signing fix if it never lands.
pub fn allow(index: usize, state: &Entity<AppState>, cx: &mut App) {
    request_one(index);
    let state = state.downgrade();
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(Duration::from_secs(10))
            .await;
        let _ = state.update(cx, |s, cx| {
            s.permission_help = true;
            cx.notify();
        });
    })
    .detach();
}

/// Ad-hoc signed builds keep a stale grant from an older build, which only re-adding clears.
pub fn help(state: &AppState) -> Option<&'static str> {
    (state.permission_help && !statuses()[1]).then_some(
        "Still not working? In Accessibility, remove TypeAside with −, then add it again with +.",
    )
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
