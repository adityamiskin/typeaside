use core_foundation::{
    base::{CFType, CFTypeRef, TCFType},
    string::{CFString, CFStringRef},
};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
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
    if status != 0 || value.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(value) })
}

pub fn focused_app() -> Option<super::FocusedApp> {
    let app = objc2_app_kit::NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let id = app.bundleIdentifier().map(|id| id.to_string());
    let title = (|| {
        let element = unsafe { AXUIElementCreateApplication(app.processIdentifier()) };
        if element.is_null() {
            return None;
        }
        let element = unsafe { CFType::wrap_under_create_rule(element) };
        let window = attribute(&element, "AXFocusedWindow")?;
        let title = attribute(&window, "AXTitle")?;
        Some(title.downcast::<CFString>()?.to_string())
    })();
    Some(super::FocusedApp {
        id,
        title,
        name: app.localizedName().map(|name| name.to_string()),
    })
}

/// macOS always has a frontmost app; a paste that fails still falls back to the clipboard.
pub fn has_focused_window() -> bool {
    true
}
