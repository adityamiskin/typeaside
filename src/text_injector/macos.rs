pub fn focused_app_id() -> Option<String> {
    objc2_app_kit::NSWorkspace::sharedWorkspace()
        .frontmostApplication()?
        .bundleIdentifier()
        .map(|id| id.to_string())
}

pub fn has_focused_window() -> Option<bool> {
    Some(
        objc2_app_kit::NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .is_some(),
    )
}
