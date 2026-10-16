# Desktop platforms

Linux keeps evdev hotkeys, uinput paste, Wayland clipboard/primary selection,
Secret Service credentials, StatusNotifierItem tray, layer-shell overlay,
XDG autostart and wpctl ducking.

macOS uses a listen-only CGEventTap, Cmd+C/Cmd+V, NSWorkspace bundle IDs,
Keychain, a main-thread menu-bar tray, nonactivating GPUI PopUp overlay,
a private application-directory Unix socket and a LaunchAgent.
Windows uses a WH_KEYBOARD_LL hook with its own message loop, Ctrl+C/Ctrl+V,
foreground process executable names, Credential Manager, a main-thread tray,
GPUI PopUp overlay, a per-user named pipe and the current-user Run registry key.
The recording/transcription engine and configuration storage continue to use
CPAL, reqwest and directories on all three platforms. Media ducking is a no-op
on macOS and Windows; start/stop sounds still play.

## macOS installation

Run `scripts/bundle-macos.sh` on macOS, then move
`target/release/TypeAside.app` to Applications and launch that copy.
Set `TYPEASIDE_SIGNING_IDENTITY` to a signing identity for a distributed build;
local builds use an ad-hoc signature. Build and launch from a stable location
so macOS can retain the app's permission identity.

The bundle includes `NSMicrophoneUsageDescription`. Real-mode startup checks
Accessibility and requests it using `AXIsProcessTrustedWithOptions`, and
requests microphone access through AVFoundation and Input Monitoring through
CoreGraphics. Missing permissions are also
shown above the main content, including onboarding. Enable TypeAside under System
Settings > Privacy & Security > Accessibility and Microphone, and Input
Monitoring, then restart TypeAside. `--demo` never requests
these permissions or starts recording, global hotkeys, or text injection.

## Shortcuts and clipboard

Existing Linux key names, including `KEY_` prefixes, remain valid. Generic
`CTRL`, `ALT`, `SHIFT` and `META` accept either side; `LEFTCTRL`, `RIGHTALT`, etc.
retain their side. META means Command on macOS and Windows key on Windows.
The macOS/Windows settings recorder saves generic modifiers, including bare
modifier combinations. Native key tables cover letters, digits, punctuation,
navigation, numpad and F1-F20; unsupported keys produce a startup error rather
than silently creating a binding that cannot fire. On macOS physical letter
positions use the standard US virtual-key table, like Linux evdev codes.

macOS/Windows preserve plain text or image clipboard contents around paste and
selection copy. Other clipboard formats (for example file lists and rich text
metadata) are not preserved. Copy waits up to 300 ms for selected text; an app
that does not support Copy returns no selection. Paste allows 300 ms for the
target app to consume the clipboard, and leaves newer clipboard text untouched.
Windows temporarily releases held hotkey modifiers for injected shortcuts and
restores only those still physically held. Self-injected events are excluded
from hotkey detection. Windows input injection follows the OS integrity boundary;
an unelevated TypeAside cannot paste into an elevated application.

## Verification

`.github/workflows/ci.yml` builds, tests and runs Clippy with warnings denied on
Ubuntu, macOS and Windows. A separate smoke matrix downloads each native build,
launches only `--demo` with `TYPEASIDE_TRACE=1`, requires `typeaside first main frame:`
within 30 seconds, takes a screenshot where supported, and terminates the demo.
Linux uses Weston nested in Xvfb and Mesa software Vulkan; macOS launches the
bundled executable. Logs and available screenshots are uploaded even on failure.

A successful demo smoke verifies UI initialization and a painted main frame.
It does not verify microphone capture, hotkeys, external transcription, paste,
selection copy, keyring access, autostart registration, or foreground-app
integration. Those need real sessions on each OS. No real-mode input or clipboard
automation should be run against a daily-driver desktop as a smoke test.
