#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn choose_external<T>(
    explicit_device: bool,
    lid_closed: Option<bool>,
    default_is_builtin: Option<bool>,
    candidates: impl IntoIterator<Item = (T, bool)>,
) -> Option<T> {
    if explicit_device || lid_closed != Some(true) || default_is_builtin != Some(true) {
        return None;
    }
    candidates
        .into_iter()
        .find_map(|(candidate, external)| external.then_some(candidate))
}

pub(super) fn external_input(
    host: &cpal::Host,
    default: &cpal::Device,
    explicit_device: bool,
) -> Option<cpal::Device> {
    if explicit_device {
        return None;
    }
    platform::external_input(host, default)
}

#[cfg(target_os = "linux")]
mod platform {
    use cpal::traits::{DeviceTrait, HostTrait};
    use std::{collections::HashSet, path::PathBuf, process::Command};

    fn output(program: &str, args: &[&str]) -> Option<String> {
        let output = Command::new(program).args(args).output().ok()?;
        output.status.success().then_some(())?;
        String::from_utf8(output.stdout).ok()
    }

    pub(super) fn external_input(host: &cpal::Host, _: &cpal::Device) -> Option<cpal::Device> {
        let closed = match output(
            "busctl",
            &[
                "--timeout=1s",
                "get-property",
                "org.freedesktop.login1",
                "/org/freedesktop/login1",
                "org.freedesktop.login1.Manager",
                "LidClosed",
            ],
        )?
        .trim()
        {
            "b true" => true,
            "b false" => false,
            _ => return None,
        };
        if !closed {
            return None;
        }
        let source = output("pactl", &["get-default-source"])?;
        let builtin = source.trim().starts_with("alsa_input.pci-");
        if !builtin {
            return None;
        }
        let mut candidates: Vec<_> = host
            .input_devices()
            .ok()?
            .filter_map(|device| {
                let id = device.id().ok()?.1;
                let (pcm, params) = id.split_once(':')?;
                let card = params.split(',').find_map(|p| p.strip_prefix("CARD="))?;
                if card.is_empty()
                    || !card
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                {
                    return None;
                }
                let card = if card.bytes().all(|b| b.is_ascii_digit()) {
                    format!("card{card}")
                } else {
                    card.to_owned()
                };
                let path = PathBuf::from("/proc/asound").join(card);
                if !path.join("usbid").try_exists().ok()? {
                    return None;
                }
                let rank = match pcm {
                    "sysdefault" => 0,
                    "front" => 1,
                    "plughw" => 2,
                    "hw" => 3,
                    _ => return None,
                };
                Some((rank, path.canonicalize().ok()?, device))
            })
            .collect();
        candidates.sort_by_key(|(rank, _, _)| *rank);
        let mut cards = HashSet::new();
        super::choose_external(
            false,
            Some(closed),
            Some(builtin),
            candidates
                .into_iter()
                .filter(|(_, card, _)| cards.insert(card.clone()))
                .map(|(_, _, device)| (device, true)),
        )
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use core_foundation::{
        base::{CFAllocatorRef, CFType, CFTypeRef, TCFType},
        boolean::CFBoolean,
        dictionary::{CFDictionaryRef, CFMutableDictionaryRef},
        string::{CFString, CFStringRef},
    };
    use cpal::traits::{DeviceTrait, HostTrait};
    use std::{ffi::c_void, ptr};

    #[repr(C)]
    struct PropertyAddress {
        selector: u32,
        scope: u32,
        element: u32,
    }

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const std::ffi::c_char) -> CFMutableDictionaryRef;
        fn IOServiceGetMatchingService(port: u32, matching: CFDictionaryRef) -> u32;
        fn IORegistryEntryCreateCFProperty(
            entry: u32,
            key: CFStringRef,
            allocator: CFAllocatorRef,
            options: u32,
        ) -> CFTypeRef;
        fn IOObjectRelease(object: u32) -> i32;
    }

    #[link(name = "CoreAudio", kind = "framework")]
    unsafe extern "C" {
        fn AudioObjectGetPropertyData(
            object: u32,
            address: *const PropertyAddress,
            qualifier_size: u32,
            qualifier: *const c_void,
            data_size: *mut u32,
            data: *mut c_void,
        ) -> i32;
    }

    fn lid_closed() -> Option<bool> {
        let key = CFString::new("AppleClamshellState");
        unsafe {
            let matching = IOServiceMatching(c"IOPMrootDomain".as_ptr());
            if matching.is_null() {
                return None;
            }
            // A null port selects the default; GetMatchingService consumes the dictionary.
            let service = IOServiceGetMatchingService(0, matching);
            if service == 0 {
                return None;
            }
            let value =
                IORegistryEntryCreateCFProperty(service, key.as_concrete_TypeRef(), ptr::null(), 0);
            IOObjectRelease(service);
            if value.is_null() {
                return None;
            }
            let value = CFType::wrap_under_create_rule(value);
            value.downcast_into::<CFBoolean>().map(bool::from)
        }
    }

    fn transport(device: &cpal::Device) -> Option<u32> {
        let uid = CFString::new(&device.id().ok()?.1);
        let uid_ref = uid.as_concrete_TypeRef();
        let mut object = 0u32;
        let mut size = size_of::<u32>() as u32;
        let address = PropertyAddress {
            selector: u32::from_be_bytes(*b"uidd"),
            scope: u32::from_be_bytes(*b"glob"),
            element: 0,
        };
        // UID translation takes a CFStringRef as qualifier and returns an AudioObjectID.
        let status = unsafe {
            AudioObjectGetPropertyData(
                1,
                &address,
                size_of::<CFStringRef>() as u32,
                ptr::from_ref(&uid_ref).cast(),
                &mut size,
                ptr::from_mut(&mut object).cast(),
            )
        };
        if status != 0 || object == 0 || size != size_of::<u32>() as u32 {
            return None;
        }
        let address = PropertyAddress {
            selector: u32::from_be_bytes(*b"tran"),
            ..address
        };
        let mut transport = 0u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                object,
                &address,
                0,
                ptr::null(),
                &mut size,
                ptr::from_mut(&mut transport).cast(),
            )
        };
        (status == 0 && size == size_of::<u32>() as u32).then_some(transport)
    }

    pub(super) fn external_input(
        host: &cpal::Host,
        default: &cpal::Device,
    ) -> Option<cpal::Device> {
        let closed = lid_closed()?;
        if !closed {
            return None;
        }
        let builtin = transport(default)? == u32::from_be_bytes(*b"bltn");
        if !builtin {
            return None;
        }
        super::choose_external(
            false,
            Some(closed),
            Some(builtin),
            host.input_devices().ok()?.filter_map(|device| {
                let transport = transport(&device)?;
                let external = ![
                    0,
                    u32::from_be_bytes(*b"bltn"),
                    u32::from_be_bytes(*b"grup"),
                    u32::from_be_bytes(*b"virt"),
                ]
                .contains(&transport);
                Some((device, external))
            }),
        )
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod platform {
    pub(super) fn external_input(_: &cpal::Host, _: &cpal::Device) -> Option<cpal::Device> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::choose_external;

    #[test]
    fn switches_only_from_an_implicit_builtin_with_a_closed_lid() {
        let candidates = [("built-in", false), ("USB", true), ("second USB", true)];
        for (explicit, closed, builtin, expected) in [
            (false, Some(true), Some(true), Some("USB")),
            (true, Some(true), Some(true), None),
            (false, Some(false), Some(true), None),
            (false, Some(true), Some(false), None),
            (false, None, Some(true), None),
            (false, Some(true), None, None),
        ] {
            assert_eq!(
                choose_external(explicit, closed, builtin, candidates),
                expected
            );
        }
        assert_eq!(
            choose_external(false, Some(true), Some(true), [("built-in", false)]),
            None
        );
        assert_eq!(
            choose_external::<&str>(false, Some(true), Some(true), []),
            None
        );
    }
}
