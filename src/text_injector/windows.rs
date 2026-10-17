use windows::Win32::{
    Foundation::CloseHandle,
    System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    },
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};
use windows::core::PWSTR;

pub fn focused_app_id() -> Option<String> {
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = vec![0u16; 32768];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        let path = String::from_utf16(&buffer[..length as usize]).ok()?;
        std::path::Path::new(&path)
            .file_name()?
            .to_str()
            .map(str::to_owned)
    }
}

pub fn has_focused_window() -> Option<bool> {
    Some(unsafe { !GetForegroundWindow().is_invalid() })
}
