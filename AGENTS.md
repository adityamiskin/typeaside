# AGENTS.md

## Stack

Bun monorepo:

- TanStack Start in `apps/web`, deployed to Cloudflare Workers through Wrangler (`apps/web/wrangler.jsonc`). Workers Builds runs `bun run build:prod` then `bun run deploy` from `apps/web` on pushes to `main`
- Tailwind CSS v4 and shadcn/ui primitives in `packages/ui`
- Vite+ (`vp`) for dev, build, lint, format, and type checks. Lint and format config lives in the root `vite.config.ts`

- Rust/GPUI desktop app at the repository root (Cargo.toml and src/)

## Required Rules

- Use Bun commands: `bun install`, `bun run dev:web`, `bun run build`, `bun run check`.
- Do not introduce another deployment target, formatter, or linter.
- Use shadcn primitives from `@typeaside/ui/components/*` before adding custom UI.
- This shadcn setup uses Base UI. Use exported class helpers like `buttonVariants` for link-styled buttons instead of Radix `asChild`.
- Keep deployment on Cloudflare through Wrangler.
- Worker bindings are declared in `apps/web/wrangler.jsonc`. Read them with `import { env } from "cloudflare:workers"` inside request handlers, not at module scope.
- Do not put secrets in public `VITE_` variables. The app has no backend; never add one without asking.

- There is no account system, billing, telemetry, or hosted transcription. Do not reintroduce sign-in, metering, or a hosted API.

## Desktop

The desktop app is Rust/GPUI. Transcription is on-device (ONNX or ggml via transcribe.cpp) or BYOK through an OpenAI-compatible endpoint; keys live in the system keychain (`src/credentials.rs`, vendored `keyring` under `vendor/keyring`).

## Linux Credential Storage

- The app explicitly uses `secret-service` on Linux in `src/credentials.rs`.
- A local vendored copy of `keyring` lives under `vendor/keyring`.
- That vendored patch intentionally disables `keyring`'s SQLite/db-keystore backend because it drags in `turso -> aegis`, which broke Linux desktop builds through Clang/AVX512 compilation.
- Do not remove the `[patch.crates-io] keyring = { path = ... }` entry from `Cargo.toml` unless the upstream dependency chain is verified to build cleanly again on Linux.

## Verification

Before handing off web changes, run:

```bash
bun run check
bun run build
```

Desktop changes need:

```bash
cargo check
cargo test
```

Building the desktop app on Linux x86_64 needs CMake and the Vulkan headers (`vulkan-headers` on Arch, `libvulkan-dev glslc` on Debian/Ubuntu). For a CPU-only type-check without Vulkan installed: `CMAKE_ARGS="-DTRANSCRIBE_VULKAN=OFF" cargo check`.
