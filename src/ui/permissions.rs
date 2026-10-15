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
    if !unsafe { AXIsProcessTrusted() } {
        let options = CFDictionary::from_CFType_pairs(&[(
            CFString::new("AXTrustedCheckOptionPrompt"),
            CFBoolean::true_value(),
        )]);
        unsafe {
            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
        }
    }
    if !unsafe { CGPreflightListenEventAccess() } {
        unsafe {
            CGRequestListenEventAccess();
        }
    }
    unsafe {
        if bundled_microphone_description()
            && let Some(audio) = AVMediaTypeAudio
            && AVCaptureDevice::authorizationStatusForMediaType(audio)
                == AVAuthorizationStatus::NotDetermined
        {
            let completion = block2::RcBlock::new(|granted: objc2::runtime::Bool| {
                if !granted.as_bool() {
                    eprintln!(
                        "hermes: microphone permission denied; enable Hermes in System Settings > Privacy & Security > Microphone"
                    );
                }
            });
            AVCaptureDevice::requestAccessForMediaType_completionHandler(audio, &completion);
        }
    }
    if let Some(message) = notice() {
        eprintln!("hermes: {message}");
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
            "Launch Hermes.app built with scripts/bundle-macos.sh to enable microphone access."
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
            "Allow {} for Hermes in System Settings > Privacy & Security, then restart Hermes.",
            missing.join(", ")
        )
    })
}
