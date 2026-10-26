GPUI 0.3.9 is vendored from crates.io. gpui-pre exposes
`Window::set_layer_shell_keyboard_interactivity` through PlatformWindow;
gpui-pre-linux applies it to the Wayland layer surface and commits immediately.
Other backends inherit a no-op. The existing input-region API is unchanged.

Quick Capture keeps a transparent, input-free surface alive between captures.
Its keyboard policy stays None until selection copying has finished, then
switches to Exclusive. Keep these two patches when upgrading GPUI.

gpui-pre-wgpu shares bind group layouts, pipelines and the glyph atlas across
every window on a GPU device (`RenderCache` in `wgpu_renderer.rs`). Upstream
builds them per window, so each new window and every dictation pill compiled
shaders and rasterized its glyphs again. The atlas carries an epoch: clearing
it, or a device loss reported by the first window, makes the other windows
repaint instead of drawing stale tiles.

Keep these patches when upgrading GPUI.
