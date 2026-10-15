# Cross-platform implementation verification

Verified locally on Linux, 2026-10-15, in the `hermes-xplat` worktree on
`cross-platform`. No push, Hermes launch, live keystroke injection, clipboard
access, microphone capture, or autostart/keyring mutation was performed.

## Production manifest

All commands used `PATH="$HOME/.cargo/bin:$PATH"` and at most two Cargo jobs.

`cargo build --release` passed after the final source changes:

```text
Finished `release` profile [optimized] target(s) in 2m 59s
```

A final cached invocation confirmed the current tree:

```text
Finished `release` profile [optimized] target(s) in 0.32s
```

`cargo clippy --all-targets -- -D warnings` passed:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.35s
```

`cargo test` passed:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 3.98s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo fmt --check`, diff whitespace checks, shell syntax, Python smoke-script
syntax, workflow YAML parsing and Info.plist parsing also passed. The extracted
Linux hotkeys, text injector, tray and autostart implementations are byte-identical
to the original files. Linux instance code differs only in its relocated sibling
module reference. Linux credential store selection and error text remain the same.

`rustup target add x86_64-pc-windows-msvc aarch64-apple-darwin` succeeded; the final
invocation reported:

```text
info: component rust-std for target x86_64-pc-windows-msvc is up to date
info: component rust-std for target aarch64-apple-darwin is up to date
```

`cargo check --target x86_64-pc-windows-msvc` stopped in `ring v0.17.14`:

```text
error: failed to run custom build command for `ring v0.17.14`
error occurred in cc-rs: failed to find tool "lib.exe": No such file or directory (os error 2)
```

Retrying with the installed `clang-cl` and `llvm-lib` reached the missing Windows
SDK C headers:

```text
fatal error: 'assert.h' file not found
```

`cargo check --target aarch64-apple-darwin` also stopped in `ring v0.17.14` because
the default host C compiler does not accept Apple flags. Retrying with
`CC_aarch64_apple_darwin=clang AR_aarch64_apple_darwin=llvm-ar` reached the missing
Apple SDK header:

```text
fatal error: 'TargetConditionals.h' file not found
```

No system packages or SDKs were installed locally.

## Additional cross-target Rust checks

Temporary manifests under `target/` checked the actual source files. The full
source manifest omitted only reqwest's `rustls-tls` feature to bypass the C/SDK
blocker, and resolved its own dependency lockfile. The production manifest still
uses rustls HTTPS. This verifies Rust types, not a production executable or HTTPS
integration on those targets.

Windows full-source check:

```sh
CC_x86_64_pc_windows_msvc=clang-cl AR_x86_64_pc_windows_msvc=llvm-lib \
RC_x86_64_pc_windows_msvc=llvm-rc CARGO_BUILD_JOBS=2 \
cargo clippy --manifest-path target/ui-check/Cargo.toml \
  --target x86_64-pc-windows-msvc --target-dir target/windows-check \
  --all-targets -- -D warnings
```

```text
Checking hermes v0.1.5 (.../hermes-xplat/target/ui-check)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 37s
```

The macOS full-source check initially stopped because `gpui-pre-apple`'s build
script runs its shader generator only on Apple hosts. Its real upstream
cbindgen/runtime-shader generator was copied into a temporary host helper and
run into Cargo's temporary OUT_DIR, producing the real 66,431-byte
`stitched_shaders.metal`. Dependency sources were not edited. No Metal compiler
or renderer was executed.

```sh
CC_aarch64_apple_darwin=clang AR_aarch64_apple_darwin=llvm-ar CARGO_BUILD_JOBS=2 \
cargo clippy --manifest-path target/ui-check/Cargo.toml \
  --target aarch64-apple-darwin --target-dir target/cross-check \
  --all-targets -- -D warnings
```

```text
Checking hermes v0.1.5 (.../hermes-xplat/target/ui-check)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.56s
warning: the following packages contain code that will be rejected by a future version of Rust: block v0.1.6
```

The `block` future-compatibility notice comes from an upstream dependency; Clippy
with warnings denied exited successfully. Earlier isolated native-module Clippy
checks also passed for both targets.

Exact local command logs are retained in `target/verification/`.

## Feature coverage by OS

The feature set below means: hold/toggle hotkeys, cancel, command mode,
paste-last, recording, transcription, paste and selection copy with clipboard
restoration, focused-app detection, tray Open/Quit, transparent overlay,
single-instance handoff, keyring, autostart, shortcut capture and config paths.

- Linux: the entire feature set compiles and links with the production manifest;
  all three existing tests pass. Runtime interaction was not exercised.
- Windows: the entire feature set passes full-source cross-target Clippy with the
  temporary TLS-free manifest. Production-manifest checks and linking remain
  blocked by the SDK headers; runtime interaction was not exercised.
- macOS: the entire feature set, plus permission prompts, bundle detection and
  onboarding notice, passes full-source cross-target Clippy with the temporary
  TLS-free manifest and upstream-generated shader source. Production-manifest
  checks and linking remain blocked by the SDK headers; runtime interaction,
  permission dialogs, bundle signing and Metal compilation were not exercised.
- CI smoke: implemented for all three OSes, not executed. This branch was never
  pushed. No screenshots have been produced yet. The workflow checks for a painted
  first main frame and uploads logs and any available screenshot.

Real hotkey press/release behavior, audio, network transcription, copy/paste into
other apps, focus retention, tray interaction, single-instance handoff, credential
persistence and login startup remain runtime-unverified on every OS. Demo smoke
does not establish those behaviors.

## Decisions and dependencies

Native hotkey hooks have explicit shutdown so config reload does not accumulate
listeners. Existing Linux modules remain intact. Native trays are created and
retained on GPUI's main thread. The vendored keyring already enabled Keychain and
Windows Credential Manager; its SQLite-disabling patch remains intact. The
vendored renderer's Vulkan restriction now applies only to Linux.

New target-specific direct dependencies:

- `arboard`: native text/image clipboard save, set and restore.
- `enigo`: native Copy, Paste and Enter injection.
- `tray-icon`: macOS/Windows tray and its re-exported `muda` menus.
- `core-graphics`, `core-foundation`: macOS event tap, run loop, permission options
  and bundle metadata.
- `objc2`, `block2`, `objc2-av-foundation`, `objc2-app-kit`: microphone permission
  requests and frontmost-app bundle IDs.
- `windows`: low-level keyboard hook, physical modifier tracking and foreground
  process lookup.
- `interprocess`: Windows named-pipe instance handoff.
- `winreg`: current-user login startup registration.

Tradeoffs: ducking is intentionally disabled outside Linux. Clipboard restoration
preserves plain text or images, not all rich/native clipboard formats. macOS key
names use physical US virtual-key positions; Windows uses virtual keys, so
non-US layouts need native runtime validation. Unsupported shortcut keys produce
an error. See `platforms.md` for setup and remaining platform constraints.
