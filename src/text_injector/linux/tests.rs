use super::*;

#[test]
fn hyprland_focus_distinguishes_absent_invalid_and_terminal_windows() {
    assert_eq!(parse_hyprland_window(b"{}"), Some(None));
    assert_eq!(parse_hyprland_window(br#"{"title":"stale"}"#), Some(None));
    assert_eq!(parse_hyprland_window(br#"{"address":""}"#), Some(None));
    assert_eq!(parse_hyprland_window(b"not json"), None);
    assert_eq!(parse_hyprland_window(b"[]"), None);
    let app = parse_hyprland_window(
        br#"{"address":"0x123","class":"com.mitchellh.ghostty","title":"shell"}"#,
    )
    .unwrap()
    .unwrap();
    assert_eq!(app.id.as_deref(), Some("com.mitchellh.ghostty"));
    assert_eq!(app.title.as_deref(), Some("shell"));
    assert!(is_terminal(app.id.as_deref().unwrap()));
}

#[test]
fn clipboard_mime_prefers_text_then_images_and_excludes_passwords() {
    assert_eq!(
        clipboard_mime("image/png\ntext/html\ntext/plain;charset=utf-8\n"),
        Some("text/plain;charset=utf-8")
    );
    assert_eq!(
        clipboard_mime("text/html\ntext/plain\n"),
        Some("text/plain")
    );
    assert_eq!(clipboard_mime("text/html\nimage/png\n"), Some("text/html"));
    assert_eq!(
        clipboard_mime("application/octet-stream\nimage/png\n"),
        Some("image/png")
    );
    assert_eq!(
        clipboard_mime("application/octet-stream\n"),
        Some("application/octet-stream")
    );
    assert_eq!(
        clipboard_mime("text/plain\nx-kde-passwordManagerHint\n"),
        None
    );
    assert_eq!(clipboard_mime("\n"), None);
}
