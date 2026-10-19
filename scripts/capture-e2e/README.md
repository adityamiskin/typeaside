# Linux Quick Capture E2E

From the repository root, run:

```sh
python3 scripts/e2e-capture.py --ref HEAD
```

For the originally requested product revision, use `--ref 95b21ab`. The harness
exports committed sources, adds logging only to that export, and builds offline
with `CARGO_BUILD_JOBS=6` and the shared target directory. It copies the resulting
binary into a fresh `/tmp/capture-e2e-*` artifact directory. Nothing is deployed,
pushed, or written to the checked-out product sources.

This machine's defaults are `/home/aditya/Projects/typeaside/target` and the
already extracted `/tmp/typeaside-e2e/vulkan/usr/lib/libvulkan_lvp.so`. Override
them with `--target-dir` and `--lavapipe` if necessary. A system installation of
Mesa lavapipe usually supplies `/usr/lib/libvulkan_lvp.so`. The harness does not
download or install anything. `--artifacts /tmp/unique-name` sets the destination;
use a fresh directory for each run.

## Isolation

An independent KVM kernel owns both the Shift-tap keyboard and the application's
Ctrl+Insert keyboard. A nested compositor alone cannot provide that guarantee.
The guest has its own devtmpfs, tmpfs home/config/runtime, D-Bus, udev, seatd, and
niri. It has no networking, input passthrough, host Wayland socket, or host D-Bus
connection. The host root is a read-only 9p export; only the artifact directory
is writable. Guest code verifies PID 1, QEMU DMI, mount flags, and private runtime
before creating any uinput device. Root filesystem caching is enabled because
that export is read-only and repeated binary/library reads otherwise distort
latency and udev discovery timing.

The host reads clipboard/PRIMARY hashes, focused window ID/app ID, live TypeAside
process start times, and input-device names before and after the guest. It does
not save clipboard text or window titles. These comparisons and distinct kernel
boot IDs are part of the pass gate. Human focus/clipboard changes during the run
can invalidate that comparison; inspect `live-isolation.json` before attributing
one to the test. No host input or clipboard writes are performed, and the live
TypeAside service is never stopped or restarted. No root access is needed.

## Coverage and artifacts

A GTK TextView selects a known Unicode string using GTK's native selection API.
Copying happens through the product's real uinput Ctrl+Insert shortcut, niri
clipboard events, and `wl-paste --watch`. A desktop entry resolves its reverse-DNS
application ID to `Capture Fixture`. Eight real double Left Shift taps alternate
selected and unselected text, with an unrelated multiline clipboard sentinel.
The first selected case uses a fresh application and injector. Subsequent cases
reuse both. The driver types `x` as soon as it observes compositor keyboard focus, checks that
it reaches only the note, clears drafts through the real textarea, and closes
with Escape. It records the fixture and capture key events to verify that the
real Ctrl+Insert reaches only the original app. It also checks that focus follows
selection copying, Escape retains a draft, Enter persists it, and the next
capture clears the saved draft. Startup and closed-window screenshots verify
that the persistent surface is invisible.

`results.json` and `report.txt` contain the gate result, each assertion, individual
latencies, and cold/warm timing breakdowns. `run-N.json` includes exact expected
and rendered textarea values, source header, clipboard before/after, row count,
and geometry. `capture-N.png` is a real guest niri screenshot; empty captures have
an 88 px textarea (4 × 22 px) in a 168 px panel. The host also verifies the actual
168 px panel span in every empty screenshot. Screenshotting occurs after the
clipboard comparison because niri screenshots modify the guest clipboard.
`fixture-before.png` shows the selected real application. App, fixture, niri,
udev, seatd, build, and boot logs are retained alongside the binary hash, source
revision, and uncommitted instrumentation patch.

`results.json` also records process RSS in KiB at idle startup and after each
closed capture. Compare the same harness on the base revision and changed
revision; capture windows are created on first use, so startup has no GPU cost.
No host application memory or clipboard text is sampled.

The second Shift release uses the guest kernel's evdev timestamp, bracketed by
the driver's writes. First-visible timing ends after the first successful wgpu `frame.present`
following a visible `Capture::render`, including layout and draw submission.
Keyboard-focus timing ends at GPUI's activation notification from the real
compositor keyboard-enter event, rather than the exclusive-focus request.
The cold capture is the first capture of a fresh process, which creates the
retained window, two seconds after the hotkey service is ready. Warm captures
reuse that process and injector. This is a
rendered-frame submission measurement; physical scanout is not measured. Debug
builds, lavapipe rendering, and VM filesystem/CPU overhead make these numbers
unsuitable as direct native-session performance estimates. The breakdown still
exposes synchronous lookup/copy waits and cold renderer initialization.

Requirements: this Linux machine's readable kernel/modules and mkinitcpio
BusyBox, accessible KVM and AMD render node `/dev/dri/renderD129`, QEMU with
VirGL/EGL, Mesa lavapipe, niri, seatd, udev, wl-clipboard, Python with PyGObject
GTK 4 and evdev, host Pillow, and the Rust toolchain/dependencies already cached. The VM uses
3 GiB RAM and four vCPUs. The runner disposes it after guest poweroff using
`-no-shutdown` and an intentional kill, avoiding a VirGL cleanup crash observed
with this host driver. It does not stop or modify other QEMU processes.

Exit status is zero only if every assertion and isolation comparison passes.
No unit tests or full repository E2E suite run.
