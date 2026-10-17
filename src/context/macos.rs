use super::Text;
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, TCFType},
    string::{CFString, CFStringRef},
};
use std::collections::VecDeque;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateSystemWide() -> CFTypeRef;
    fn AXUIElementSetMessagingTimeout(element: CFTypeRef, timeout: f32) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementCopyAttributeValues(
        element: CFTypeRef,
        attribute: CFStringRef,
        index: isize,
        max_values: isize,
        values: *mut CFArrayRef,
    ) -> i32;
}

fn attribute(element: &CFType, name: &str) -> Option<CFType> {
    let name = CFString::new(name);
    let mut value = std::ptr::null();
    let status = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            &mut value,
        )
    };
    (status == 0 && !value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value) })
}

fn string(element: &CFType, name: &str) -> Option<String> {
    attribute(element, name)?
        .downcast::<CFString>()
        .map(|value| value.to_string())
}

pub(super) fn text() -> String {
    let mut text = Text::new();
    let system = unsafe { AXUIElementCreateSystemWide() };
    if system.is_null() {
        return text.value;
    }
    let system = unsafe { CFType::wrap_under_create_rule(system) };
    unsafe {
        AXUIElementSetMessagingTimeout(system.as_CFTypeRef(), 0.01);
    }
    let Some(app) = attribute(&system, "AXFocusedApplication") else {
        return text.value;
    };
    let Some(window) = attribute(&app, "AXFocusedWindow") else {
        return text.value;
    };
    let mut queue = VecDeque::from([window]);
    let mut visited = 0;
    while !text.expired() && visited < 2000 {
        let Some(element) = queue.pop_front() else {
            break;
        };
        visited += 1;
        if string(&element, "AXSubrole").as_deref() == Some("AXSecureTextField") {
            continue;
        }
        let role = string(&element, "AXRole");
        if matches!(
            role.as_deref(),
            Some("AXStaticText" | "AXTextField" | "AXTextArea")
        ) {
            for name in ["AXValue", "AXTitle", "AXDescription"] {
                if text.expired() {
                    break;
                }
                if let Some(value) = string(&element, name) {
                    text.push(&value);
                }
            }
        }
        let remaining = 2000 - visited - queue.len();
        if remaining == 0 || text.expired() {
            continue;
        }
        let name = CFString::new("AXChildren");
        let mut children = std::ptr::null();
        let status = unsafe {
            AXUIElementCopyAttributeValues(
                element.as_CFTypeRef(),
                name.as_concrete_TypeRef(),
                0,
                remaining as isize,
                &mut children,
            )
        };
        if status == 0 && !children.is_null() {
            let children = unsafe { CFArray::<CFType>::wrap_under_create_rule(children) };
            for child in children.iter().take(remaining) {
                queue.push_back((*child).clone());
            }
        }
    }
    text.value
}
