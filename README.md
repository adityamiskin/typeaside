# TypeAside

Native voice dictation for macOS, Windows and Linux. Hold a key, speak, and the text lands where your cursor is. Free and open source (MIT).

- **On-device models** — Parakeet v3, Whisper, Cohere and Bark transcribe entirely on your computer, with GPU acceleration on every platform and silence gating built in.
- **Bring your own key** — or point the app at any OpenAI-compatible speech endpoint (Groq works well) and let the cloud transcribe. Your key lives in the system keychain (Keychain / Credential Manager / Secret Service).
- **AI editing** — optional cleanup removes fillers and false starts; command mode edits selected text or answers a spoken question; translation to any language.
- **Keyboard-first** — evdev hotkeys on Linux (Wayland layer-shell overlay included), CGEventTap on macOS, low-level hook on Windows. Injected text is verified against the field, falling back to the clipboard so a dictation is never lost.
- **Notes, dictionary, snippets** — quick local notes, custom vocabulary that biases recognition, text expansion, per-app styles, and corrections learned from your edits.

There is no account, no server, no telemetry. Audio and text go only where you point them: an on-device model, or the endpoint you configure.

## Screenshots

| Dictation history                          | Quick notes                           |
| ------------------------------------------ | ------------------------------------- |
| ![Dictation history](docs/assets/home.png) | ![Quick notes](docs/assets/notes.png) |

| Settings                              | Onboarding                                |
| ------------------------------------- | ----------------------------------------- |
| ![Settings](docs/assets/settings.png) | ![Onboarding](docs/assets/onboarding.png) |

## Layout

- `Cargo.toml`, `src/`, `assets/`, `vendor/`: the Rust/GPUI desktop app.
- `apps/web`: marketing site, docs and downloads (TanStack Start on Cloudflare Workers; installers are served from an R2 bucket).
- `packages/ui`: shared Base UI shadcn primitives and the Tailwind v4 theme.
- `docs/`: platform notes, on-device model catalog, history limits.
- `examples/`: runnable end-to-end checks for the pipeline and local models.

## Desktop development

```bash
cargo check          # Rust checks; building needs CMake and, on Linux x86_64, Vulkan headers
cargo test
cargo run
```

`docs/local-models.md` documents the model catalog, GPU acceleration and the streaming worker. `docs/platforms.md` covers per-OS behaviour, from Wayland input to macOS permissions.

## Website development

Use Bun and Vite+ (`vp`). The site is static content plus R2-backed downloads and needs no credentials to run.

```bash
bun install
bun run dev:web      # http://localhost:3001
bun run check        # Vite+ format, lint and type checks
bun run build
```
