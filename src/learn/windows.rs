use windows::Win32::{
    Foundation::{E_NOINTERFACE, E_POINTER},
    System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize,
    },
    UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
        IUIAutomationTextRange, IUIAutomationValuePattern, TextPatternRangeEndpoint_End,
        TextPatternRangeEndpoint_Start, UIA_E_NOTSUPPORTED, UIA_TextPatternId, UIA_ValuePatternId,
    },
};

struct Apartment;

impl Apartment {
    fn new() -> Option<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .ok()?;
        Some(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub(super) struct FocusedElement {
    automation: IUIAutomation,
    element: IUIAutomationElement,
    // COM interfaces must be released before the apartment is uninitialized.
    _apartment: Apartment,
}

impl FocusedElement {
    pub fn capture() -> Option<Self> {
        let apartment = Apartment::new()?;
        let automation: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.ok()?;
        let element = unsafe { automation.GetFocusedElement() }.ok()?;
        Some(Self {
            automation,
            element,
            _apartment: apartment,
        })
    }

    pub fn is_focused(&self) -> bool {
        unsafe {
            self.automation
                .GetFocusedElement()
                .and_then(|focused| self.automation.CompareElements(&self.element, &focused))
                .is_ok_and(|same| same.as_bool())
        }
    }

    pub fn value(&self) -> Option<String> {
        unsafe {
            if self.element.CurrentIsPassword().ok()?.as_bool() {
                return None;
            }
            match self
                .element
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            {
                Ok(pattern) => pattern.CurrentValue().ok().map(|value| value.to_string()),
                Err(error)
                    if error.code() == E_NOINTERFACE
                        || error.code() == E_POINTER
                        || error.code().0 as u32 == UIA_E_NOTSUPPORTED =>
                {
                    self.element
                        .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                        .ok()?
                        .DocumentRange()
                        .ok()?
                        .GetText(-1)
                        .ok()
                        .map(|value| value.to_string())
                }
                Err(_) => None,
            }
        }
    }

    /// UTF-16 start and length.
    pub fn selection(&self) -> Option<(usize, usize)> {
        unsafe {
            let pattern = self
                .element
                .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                .ok()?;
            let selection = pattern.GetSelection().ok()?.GetElement(0).ok()?;
            let before = pattern.DocumentRange().ok()?;
            before
                .MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    &selection,
                    TextPatternRangeEndpoint_Start,
                )
                .ok()?;
            let length = |range: &IUIAutomationTextRange| Some(range.GetText(-1).ok()?.len());
            Some((length(&before)?, length(&selection)?))
        }
    }
}
