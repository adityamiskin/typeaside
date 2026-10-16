use core_foundation::{
    base::TCFType,
    boolean::CFBoolean,
    bundle::{CFBundleGetMainBundle, CFBundleGetValueForInfoDictionaryKey},
    dictionary::{CFDictionary, CFDictionaryRef},
    string::CFString,
};
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

pub fn request() {
    let granted = statuses();
    for index in [1, 2] {
        if !granted[index] {
            request_one(index);
        }
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

pub fn statuses() -> [bool; 3] {
    [
        unsafe {
            AVMediaTypeAudio.is_some_and(|audio| {
                AVCaptureDevice::authorizationStatusForMediaType(audio)
                    == AVAuthorizationStatus::Authorized
            })
        },
        unsafe { AXIsProcessTrusted() },
        unsafe { CGPreflightListenEventAccess() },
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
            let options = CFDictionary::from_CFType_pairs(&[(
                CFString::new("AXTrustedCheckOptionPrompt"),
                CFBoolean::true_value(),
            )]);
            unsafe {
                AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
            }
        }
        2 => unsafe {
            CGRequestListenEventAccess();
        },
        _ => {}
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
    let input_monitoring = !unsafe { CGPreflightListenEventAccess() };
    let missing = [
        (accessibility, "Accessibility"),
        (microphone, "Microphone"),
        (input_monitoring, "Input Monitoring"),
    ]
    .into_iter()
    .filter_map(|(missing, name)| missing.then_some(name))
    .collect::<Vec<_>>();
    (!missing.is_empty()).then(|| {
        format!(
            "Allow {} for TypeAside in System Settings > Privacy & Security, then restart TypeAside.",
            missing.join(", ")
        )
    })
}
