use windows::Win32::{
    Foundation::CloseHandle,
    System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    },
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId},
};
use windows::core::PWSTR;

pub fn focused_app() -> Option<super::FocusedApp> {
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return None;
        }
        let mut title = [0u16; 256];
        let length = GetWindowTextW(window, &mut title);
        let title = (length > 0).then(|| String::from_utf16_lossy(&title[..length as usize]));
        let id = (|| {
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
        })();
        let name = id.as_deref().map(|id| {
            super::fallback_app_name(
                id.rsplit_once('.')
                    .filter(|(_, extension)| extension.eq_ignore_ascii_case("exe"))
                    .map_or(id, |(stem, _)| stem),
            )
        });
        Some(super::FocusedApp { id, title, name })
    }
}

pub fn has_focused_window() -> bool {
    unsafe { !GetForegroundWindow().is_invalid() }
}
