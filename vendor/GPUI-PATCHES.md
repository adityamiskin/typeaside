GPUI 0.3.9 is vendored from crates.io. gpui-pre exposes
`Window::set_layer_shell_keyboard_interactivity` through PlatformWindow;
gpui-pre-linux applies it to the Wayland layer surface and commits immediately.
Other backends inherit a no-op. The existing input-region API is unchanged.

Quick Capture keeps a transparent, input-free surface alive between captures.
Its keyboard policy stays None until selection copying has finished, then
switches to Exclusive. Keep these two patches when upgrading GPUI.
