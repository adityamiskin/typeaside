use core_foundation::{
    base::{CFType, CFTypeRef, TCFType},
    string::{CFString, CFStringRef},
};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateSystemWide() -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
}

const ATTRIBUTE_UNSUPPORTED: i32 = -25205;
const NO_VALUE: i32 = -25212;

fn attribute(element: &CFType, name: &str) -> Result<CFType, i32> {
    let name = CFString::new(name);
    let mut value = std::ptr::null();
    let status = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            &mut value,
        )
    };
    if status != 0 {
        return Err(status);
    }
    if value.is_null() {
        return Err(NO_VALUE);
    }
    Ok(unsafe { CFType::wrap_under_create_rule(value) })
}

pub(super) struct FocusedElement {
    system: CFType,
    element: CFType,
}

impl FocusedElement {
    pub fn capture() -> Option<Self> {
        let system = unsafe { AXUIElementCreateSystemWide() };
        if system.is_null() {
            return None;
        }
        let system = unsafe { CFType::wrap_under_create_rule(system) };
        let element = attribute(&system, "AXFocusedUIElement").ok()?;
        Some(Self { system, element })
    }

    pub fn is_focused(&self) -> bool {
        attribute(&self.system, "AXFocusedUIElement").is_ok_and(|element| element == self.element)
    }

    pub fn value(&self) -> Option<String> {
        match attribute(&self.element, "AXSubrole") {
            Ok(subrole) => {
                if subrole.downcast::<CFString>()?.to_string() == "AXSecureTextField" {
                    return None;
                }
            }
            // Ordinary editable elements can omit this optional attribute.
            Err(ATTRIBUTE_UNSUPPORTED | NO_VALUE) => {}
            Err(_) => return None,
        }
        attribute(&self.element, "AXValue")
            .ok()?
            .downcast::<CFString>()
            .map(|value| value.to_string())
    }
}
