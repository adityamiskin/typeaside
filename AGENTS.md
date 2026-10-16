# AGENTS.md

## Stack

Bun monorepo:

- TanStack Start in `apps/web`, deployed to Cloudflare Workers through Wrangler (`apps/web/wrangler.jsonc`)
- Tailwind CSS v4 and shadcn/ui primitives in `packages/ui`
- Vite+ (`vp`) for dev, build, lint, format, type checks, and task running. Lint and format config lives in the root `vite.config.ts`

- Convex backend in `packages/backend/convex`
- WorkOS AuthKit for auth

## Required Rules

- Use Bun commands: `bun install`, `bun run dev`, `bun run build`, `bun run check`.
- Do not introduce another backend, auth provider, deployment target, formatter, or linter.
- Use shadcn primitives from `@typeaside/ui/components/*` before adding custom UI.
- This shadcn setup uses Base UI. Use exported class helpers like `buttonVariants` for link-styled buttons instead of Radix `asChild`.
- Keep deployment on Cloudflare through Wrangler.
- Worker bindings are declared in `apps/web/wrangler.jsonc`. Read them with `import { env } from "cloudflare:workers"` inside request handlers, not at module scope.
- Do not put secrets in public `VITE_` variables.

- Keep Convex functions in `packages/backend/convex` and import APIs from `@typeaside/backend/convex/_generated/api`.
- Run `bun run codegen` after adding or renaming Convex functions.

## Environment

Local env lives in `apps/web/.env` (examples in `apps/web/.env.example`). The Cloudflare Vite plugin loads it for local development. Configure deployed bindings and secrets in the Worker settings.

Convex deployment env is set with `bunx convex env set` from `packages/backend`; see `packages/backend/.env.example`.

## Auth Notes

The active provider implements `AuthIntegration` (`apps/web/src/integrations/auth/types.ts`). Use `auth` from `apps/web/src/integrations/auth` instead of provider SDKs in shared code. Its middleware is registered in `apps/web/src/start.ts`, and must stay in Start request middleware.

Protected pages use `context.viewer` and explicit Convex auth checks. `/account` shows sign-in when signed out; `/desktop/connect` redirects to `auth.signInHref` preserving its validated search parameters.

WorkOS routes: `/api/auth/sign-in`, `/api/auth/sign-up`, `/api/auth/callback`, `/sign-out`.

Convex validates auth JWTs in `packages/backend/convex/auth.config.ts`. Use `ctx.auth.getUserIdentity()` in Convex functions and prefer explicit auth checks over client state.

## Verification

Before handing off meaningful changes, run:

```bash
bun run codegen
bun run check
```

Run `bun run build` when touching routing, auth, deployment, or Vite config.

## Desktop

The Rust desktop app lives at the repository root (Cargo.toml and src/).

# AGENTS Notes

Durable repo-specific facts that are easy to miss.

## Linux Credential Storage

- The app explicitly uses `secret-service` on Linux in `src/credentials.rs`.
- A local vendored copy of `keyring` lives under `vendor/keyring`.
- That vendored patch intentionally disables `keyring`'s SQLite/db-keystore backend because it drags in `turso -> aegis`, which broke Linux desktop builds through Clang/AVX512 compilation.
- Do not remove the `[patch.crates-io] keyring = { path = ... }` entry from `Cargo.toml` unless the upstream dependency chain is verified to build cleanly again on Linux.
