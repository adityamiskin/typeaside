pub fn focused_app_id() -> Option<String> {
    objc2_app_kit::NSWorkspace::sharedWorkspace()
        .frontmostApplication()?
        .bundleIdentifier()
        .map(|id| id.to_string())
}
