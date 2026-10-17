pub fn focused_app_id() -> Option<String> {
    objc2_app_kit::NSWorkspace::sharedWorkspace()
        .frontmostApplication()?
        .bundleIdentifier()
        .map(|id| id.to_string())
}

/// macOS always has a frontmost app; a paste that fails still falls back to the clipboard.
pub fn has_focused_window() -> bool {
    true
}
