use super::{Editability, Focus};
use core_foundation::{
    base::{CFHash, CFType, CFTypeRef, TCFType},
    string::{CFString, CFStringRef},
};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateSystemWide() -> CFTypeRef;
    fn AXUIElementSetMessagingTimeout(element: CFTypeRef, timeout: f32) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementIsAttributeSettable(
        element: CFTypeRef,
        attribute: CFStringRef,
        settable: *mut u8,
    ) -> i32;
    fn AXUIElementGetPid(element: CFTypeRef, pid: *mut i32) -> i32;
}

fn attribute(element: &CFType, name: &str) -> Option<CFType> {
    let mut value = std::ptr::null();
    let name = CFString::new(name);
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

pub(super) fn capture() -> Option<Focus> {
    let system = unsafe { AXUIElementCreateSystemWide() };
    if system.is_null() {
        return None;
    }
    let system = unsafe { CFType::wrap_under_create_rule(system) };
    unsafe {
        AXUIElementSetMessagingTimeout(system.as_CFTypeRef(), 0.01);
    }
    let element = attribute(&system, "AXFocusedUIElement")?;
    unsafe {
        AXUIElementSetMessagingTimeout(element.as_CFTypeRef(), 0.01);
    }
    let role = string(&element, "AXRole")?;
    let mut settable = 0;
    let value = CFString::new("AXValue");
    let status = unsafe {
        AXUIElementIsAttributeSettable(
            element.as_CFTypeRef(),
            value.as_concrete_TypeRef(),
            &mut settable,
        )
    };
    let editability = match role.as_str() {
        "AXTextField" | "AXTextArea" | "AXComboBox" if status == 0 && settable != 0 => {
            Editability::Editable
        }
        "AXButton" | "AXCheckBox" | "AXRadioButton" | "AXMenuItem" | "AXSlider" | "AXScrollBar" => {
            Editability::NotEditable
        }
        _ => Editability::Unknown,
    };
    if editability != Editability::Editable {
        return Some(Focus {
            editability,
            ..Focus::default()
        });
    }
    let mut pid = 0;
    unsafe {
        AXUIElementGetPid(element.as_CFTypeRef(), &mut pid);
    }
    let secure = string(&element, "AXSubrole").as_deref() == Some("AXSecureTextField");
    Some(Focus {
        editability,
        // AX elements hash equal across separate queries, unlike their pointers.
        identity: format!("{pid}:{}", unsafe { CFHash(element.as_CFTypeRef()) }),
        value: (!secure).then(|| string(&element, "AXValue")).flatten(),
        caret: None,
    })
}
