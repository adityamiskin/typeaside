use super::Text;
use std::collections::VecDeque;
use windows::{
    Win32::{
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        },
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationValuePattern,
                TreeScope_Element, UIA_IsPasswordPropertyId, UIA_NamePropertyId,
                UIA_ValuePatternId, UIA_ValueValuePropertyId,
            },
            WindowsAndMessaging::GetForegroundWindow,
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

pub(super) fn text() -> String {
    let mut text = Text::new();
    unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            return text.value;
        }
        let _apartment = Apartment;
        let Ok(automation): windows::core::Result<IUIAutomation> =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        else {
            return text.value;
        };
        if let Ok(timeouts) = automation.cast::<IUIAutomation2>() {
            let _ = timeouts.SetConnectionTimeout(10);
            let _ = timeouts.SetTransactionTimeout(10);
        }
        let Ok(cache) = automation.CreateCacheRequest() else {
            return text.value;
        };
        if cache.SetTreeScope(TreeScope_Element).is_err() {
            return text.value;
        }
        for property in [
            UIA_NamePropertyId,
            UIA_ValueValuePropertyId,
            UIA_IsPasswordPropertyId,
        ] {
            if cache.AddProperty(property).is_err() {
                return text.value;
            }
        }
        if cache.AddPattern(UIA_ValuePatternId).is_err() {
            return text.value;
        }
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return text.value;
        }
        let Ok(root) = automation.ElementFromHandleBuildCache(window, &cache) else {
            return text.value;
        };
        let Ok(walker) = automation.ControlViewWalker() else {
            return text.value;
        };
        let mut queue = VecDeque::from([root]);
        let mut visited = 0;
        while !text.expired() && visited < 2000 {
            let Some(element) = queue.pop_front() else {
                break;
            };
            visited += 1;
            if !element
                .CachedIsPassword()
                .is_ok_and(|password| !password.as_bool())
            {
                continue;
            }
            if let Ok(value) = element.CachedName() {
                text.push(&value.to_string());
            }
            if !text.expired()
                && let Ok(pattern) =
                    element.GetCachedPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                && let Ok(value) = pattern.CachedValue()
            {
                text.push(&value.to_string());
            }
            if text.expired() || visited + queue.len() >= 2000 {
                continue;
            }
            let mut child = walker.GetFirstChildElementBuildCache(&element, &cache);
            while !text.expired() && visited + queue.len() < 2000 {
                let Ok(element) = child else {
                    break;
                };
                queue.push_back(element.clone());
                if visited + queue.len() >= 2000 {
                    break;
                }
                child = walker.GetNextSiblingElementBuildCache(&element, &cache);
            }
        }
    }
    text.value
}
