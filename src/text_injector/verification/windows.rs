use super::{Editability, Focus};
use windows::{
    Win32::{
        System::{
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
                CoUninitialize, SAFEARRAY,
            },
            Ole::{SafeArrayDestroy, SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound},
        },
        UI::Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationValuePattern,
            UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId, UIA_MenuItemControlTypeId,
            UIA_RadioButtonControlTypeId, UIA_ScrollBarControlTypeId, UIA_SliderControlTypeId,
            UIA_ValuePatternId,
        },
    },
    core::Interface,
};

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub(super) fn capture() -> Option<Focus> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok()?;
        let _apartment = Apartment;
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let timeouts = automation.cast::<IUIAutomation2>().ok()?;
        timeouts.SetConnectionTimeout(10).ok()?;
        timeouts.SetTransactionTimeout(10).ok()?;
        let element = automation.GetFocusedElement().ok()?;
        let pattern = element
            .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            .ok()
            .filter(|pattern| {
                pattern
                    .CurrentIsReadOnly()
                    .is_ok_and(|value| !value.as_bool())
            });
        let Some(pattern) = pattern else {
            let role = element.CurrentControlType().ok()?;
            let editability = if [
                UIA_ButtonControlTypeId,
                UIA_CheckBoxControlTypeId,
                UIA_RadioButtonControlTypeId,
                UIA_MenuItemControlTypeId,
                UIA_SliderControlTypeId,
                UIA_ScrollBarControlTypeId,
            ]
            .contains(&role)
            {
                Editability::NotEditable
            } else {
                Editability::Unknown
            };
            return Some(Focus {
                editability,
                ..Focus::default()
            });
        };
        let mut focus = Focus {
            editability: Editability::Editable,
            ..Focus::default()
        };
        if let Ok(id) = element.GetRuntimeId() {
            focus.identity = runtime_id(id).unwrap_or_default();
        }
        if element
            .CurrentIsPassword()
            .is_ok_and(|value| !value.as_bool())
        {
            focus.value = pattern.CurrentValue().ok().map(|value| value.to_string());
        }
        Some(focus)
    }
}

unsafe fn runtime_id(id: *mut SAFEARRAY) -> Option<String> {
    let values = unsafe {
        (|| {
            let start = SafeArrayGetLBound(id, 1).ok()?;
            let end = SafeArrayGetUBound(id, 1).ok()?;
            let mut values = Vec::new();
            for index in start..=end {
                let mut value = 0i32;
                SafeArrayGetElement(id, &index, (&raw mut value).cast()).ok()?;
                values.push(value);
            }
            Some(values)
        })()
    };
    let _ = unsafe { SafeArrayDestroy(id) };
    values.map(|values| format!("{values:?}"))
}
