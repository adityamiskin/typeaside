# Desktop platforms

Linux keeps evdev hotkeys, uinput paste, Wayland clipboard/primary selection,
Secret Service credentials, StatusNotifierItem tray, layer-shell overlay,
XDG autostart and media control while dictating: pause playing MPRIS players
(default) and resume only those still paused on the same track, or lower volume
through wpctl or pactl. A marker file restores the volume on the next start
after a crash.

macOS uses a listen-only CGEventTap, Cmd+C/Cmd+V, NSWorkspace bundle IDs,
Keychain, a main-thread menu-bar tray, nonactivating GPUI PopUp overlay,
a private application-directory Unix socket and a LaunchAgent.
Windows uses a WH_KEYBOARD_LL hook with its own message loop, Ctrl+C/Ctrl+V,
foreground process executable names, Credential Manager, a main-thread tray,
GPUI PopUp overlay, a per-user named pipe and the current-user Run registry key.
The recording/transcription engine and configuration storage continue to use
CPAL, reqwest and directories on all three platforms. Media control is a no-op
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

Before pasting, a 65 ms accessibility probe (macOS AX, Windows UIA, Linux AT-SPI)
classifies the focused element. A focused button, checkbox, menu item, slider or
scrollbar skips the paste and leaves the transcript on the clipboard. Anything
else, including terminals, games and apps without accessibility, pastes as
before. When a focused text field's value was readable, it is read again after
the paste window; if the same field is unchanged, the transcript stays on the
clipboard instead of being restored over. Auto-submit skips that check. On
Linux the probe only trusts a single AT-SPI active window whose title matches
the Hyprland or niri focused window, because Wayland Chromium keeps stale active
states.

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

## Command line control

With TypeAside running, use `typeaside dictation start`, `typeaside dictation stop`,
`typeaside dictation toggle`, or `typeaside dictation cancel`. Start while recording
and stop while idle do nothing, independently of the configured hold/toggle mode.
`typeaside open` shows the window. Commands exit non-zero if the instance cannot
be reached. Linux commands use the instance in the same `XDG_RUNTIME_DIR`.

For Hyprland, bind press and release separately:

```ini
bind = , F9, exec, typeaside dictation start
bindr = , F9, exec, typeaside dictation stop
```

`typeaside paste-last` pastes the latest successful transcript through the normal
text delivery path, including clipboard fallback and the auto-submit setting.
It does nothing when no transcript is available. The tray also has “Paste last
dictation”; its shortcut defaults to left Ctrl+Alt+V and can be changed or
cleared in Settings. The latest saved transcript is available after restart;
with history disabled, only transcripts from the current session are available.
