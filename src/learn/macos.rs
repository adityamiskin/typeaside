use core_foundation::{
    base::{CFRange, CFType, CFTypeRef, TCFType},
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
    fn AXValueGetValue(value: CFTypeRef, kind: u32, result: *mut std::ffi::c_void) -> bool;
}

const CF_RANGE: u32 = 4;

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
                if subrole.downcast::<CFString>()? == "AXSecureTextField" {
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

    /// UTF-16 start and length.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let value = attribute(&self.element, "AXSelectedTextRange").ok()?;
        let mut range = CFRange::init(0, 0);
        unsafe { AXValueGetValue(value.as_CFTypeRef(), CF_RANGE, (&raw mut range).cast()) }
            .then_some(())?;
        Some((
            range.location.try_into().ok()?,
            range.length.try_into().ok()?,
        ))
    }
}
