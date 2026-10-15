# Hermes

Native voice dictation for macOS, Windows and Linux, with a TanStack Start website and a Convex hosted transcription service.

## Layout

- `Cargo.toml`, `src/`, `assets/`, `vendor/`: Rust/GPUI desktop app, still at the repository root.
- `apps/web`: marketing, downloads, WorkOS sign-in, account and desktop connection routes.
- `apps/web/wrangler.jsonc`: Cloudflare Worker configuration.
- `packages/backend/convex`: identities, Dodo billing, desktop tokens, weekly usage and HTTP proxy.
- `packages/ui`: shared Base UI shadcn primitives and Hermes Tailwind v4 tokens.
- `packages/env`, `packages/config`: client environment validation and shared TypeScript configuration.

## Local development

Use Bun and Vite+ (`vp`). Local development runs in the Workers runtime without Cloudflare credentials.

```bash
bun install
cd packages/backend
CONVEX_AGENT_MODE=anonymous bunx convex dev --once
# Convex requires a client ID in auth.config.ts even without a real auth provider.
CONVEX_AGENT_MODE=anonymous bunx convex env set WORKOS_CLIENT_ID client_local_unconfigured
CONVEX_AGENT_MODE=anonymous bunx convex dev
```

The first `--once` can stop with a missing `WORKOS_CLIENT_ID` message. Set the local placeholder above, then rerun. This creates only an anonymous local deployment. It cannot sign in real users until you replace the placeholder with your WorkOS client ID. Keep Convex running while using account/proxy functions.

In another terminal:

```bash
cp apps/web/.env.example apps/web/.env
bun run --cwd apps/web dev
```

Open `http://localhost:3001`. Public pages render without WorkOS, Dodo or Groq credentials. `/account` shows sign-in; a valid signed-out `/desktop/connect` request redirects through the sign-in endpoint. Without WorkOS configuration that endpoint leads to the local sign-in page. No mock authentication bypass is shipped.

## Environment

Web env lives in `apps/web/.env` locally. The Cloudflare Vite plugin loads it locally. Set `VITE_CONVEX_URL` in Workers Builds and the WorkOS bindings in Worker settings. API keys and cookie passwords must be secrets:

| Variable                 | Purpose                                                       |
| ------------------------ | ------------------------------------------------------------- |
| `VITE_CONVEX_URL`        | Convex client URL, local default `http://127.0.0.1:3210`      |
| `WORKOS_CLIENT_ID`       | WorkOS application client ID                                  |
| `WORKOS_API_KEY`         | WorkOS secret API key                                         |
| `WORKOS_REDIRECT_URI`    | Callback URL, local `http://localhost:3001/api/auth/callback` |
| `WORKOS_COOKIE_PASSWORD` | At least 32 random characters for session encryption          |

Set backend env on the intended Convex deployment with `bunx convex env set NAME value` for development or `bunx convex env set --prod NAME value` for production from `packages/backend`. Convex functions do not read `.env.example` files. Never put secrets in `VITE_` variables.

| Backend variable               | Purpose                                                   |
| ------------------------------ | --------------------------------------------------------- |
| `WORKOS_CLIENT_ID`             | JWT validation, same client ID as the web app             |
| `GROQ_API_KEY`                 | Server-side hosted transcription and cleanup              |
| `DODO_PAYMENTS_API_KEY`        | Dodo secret API key                                       |
| `DODO_PAYMENTS_ENVIRONMENT`    | `test_mode` or `live_mode`, use test mode for development |
| `DODO_PAYMENTS_WEBHOOK_SECRET` | Standard Webhooks verification secret                     |
| `DODO_PRO_MONTHLY_PRODUCT_ID`  | Pro product configured for $12 monthly                    |
| `DODO_PRO_YEARLY_PRODUCT_ID`   | Pro product configured for $96 yearly                     |

WorkOS dashboard: callback `http://localhost:3001/api/auth/callback`, sign-in endpoint `http://localhost:3001/api/auth/sign-in`, CORS origin `http://localhost:3001`. The starter auth integration stays in Start request middleware. Dodo uses the official `@dodopayments/convex` component. Configure its subscription events at `<CONVEX_SITE_URL>/dodopayments-webhook`. Checkout metadata binds the verified event to the WorkOS subject. Duplicate and older events are ignored per subscription.

## Hosted API

Use the Convex **HTTP actions** URL (local `http://127.0.0.1:3211`, hosted `.convex.site`) and `Authorization: Bearer hm_...`:

- `POST /openai/v1/audio/transcriptions`: OpenAI multipart `file`, optional `language`, `prompt`. Forces Groq `whisper-large-v3-turbo` with JSON output. Returns the provider body unchanged and counts returned transcript words.
- `POST /openai/v1/chat/completions`: JSON `messages`, optional `temperature`, `max_tokens`, `reasoning_effort`. Forces Groq `qwen/qwen3.8-27b`. Non-streaming only, with a 64 KB request limit and at most 2,048 output tokens (also the default). No additional word counting.
- `GET /v1/me`: email, Free/Pro plan, words, cap and UTC reset timestamp.
- `POST /v1/ping`: unauthenticated, opt-in anonymous daily usage counts (8 KB, 1–7 UTC days), upserted per install/day and rate limited to 30 requests/hour per install.
- Convex mutation `feedback.send` (used by `/feedback`): bugs, ideas and other notes, with optional reply email and app version/OS; rate limited to 30/hour per signed-in user or globally for anonymous notes.

Founder queries: `bunx convex run stats:summary '{"days":30}'` returns UTC daily install counts, dictations, words, new/active hosted users and hosted words, plus the current Pro count. `bunx convex run feedback:list '{"limit":50}'` reads the newest notes. Run these from `packages/backend` against the intended deployment.

For an OpenAI-compatible desktop client, set base URL to `<CONVEX_SITE_URL>/openai/v1` and use a desktop token as the bearer key. No Rust integration is changed here. BYOK continues to call providers directly without an account.

Free includes `FREE_WEEKLY_WORDS = 2000`, exported once in `convex/lib.ts`. The week starts Monday at 00:00 UTC. Pro requires an active subscription. A request that starts below the free cap can finish above it; parallel requests already in flight are not reserved. Later requests return 402 with `error.type = "limit"` and `resets_at`. The lookup reads token, subscription and usage in one Convex query; a successful request schedules a usage/last-used mutation without waiting for that mutation to finish. The lean proxy counts transcription output before cleanup, so provider cleanup can change the final typed word total slightly. It does not retain request contents.

Desktop connect accepts only integer ports 1024–65535. After sign-in and explicit Connect, it creates a 32-byte token, stores only its SHA-256 hash, and navigates to `http://127.0.0.1:<port>/callback` with encoded state and token. Revoking a device invalidates its token immediately.

## Checks

```bash
CONVEX_AGENT_MODE=anonymous bun run codegen
bun run check
bun run build
PATH=$HOME/.cargo/bin:$PATH cargo check
```

`bun run check` runs Vite+ format, lint and type checks. Rust/vendor files are excluded from the web formatter. `bun run build` builds locally and does not deploy. Deployment uses `bun run deploy` through Wrangler. Workers Builds runs `bun run build:prod` then `bun run deploy` from `apps/web`, on pushes to `main` only. `build:prod` deploys Convex production first (using the `CONVEX_DEPLOY_KEY` build secret), then builds the site, so the backend is never behind the website. Preview builds and preview URLs are disabled. The deployed site uses Convex production, WorkOS Production and Dodo Live mode.

Real WorkOS credentials are needed to verify OAuth, signed-in account UI and browser-to-desktop token handoff. Dodo test credentials, product IDs and a reachable signed webhook endpoint are needed to verify checkout, portal and subscription lifecycle. Groq credentials are needed to verify actual transcription, cleanup and provider responses. Installers are served from the `hermes-releases` R2 bucket through `/releases/<file>`.

## Desktop releases

Bump the version in `Cargo.toml` (and update `Cargo.lock` with `cargo check`) and push to `main`. Then run the **Release** workflow manually from the Actions tab (`gh workflow run release.yml --ref main`). It refuses any ref other than `main`. It builds a universal macOS 13+ DMG, a per-user Windows 10/11 installer, and Linux x86_64 Wayland packages on Ubuntu 22.04.

Configure repository Actions secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. The token needs **Workers R2 Storage: Edit** on that account. Releases upload `Hermes.dmg`, `Hermes-setup.exe`, `Hermes.AppImage`, and `hermes.deb` to `hermes-releases`, plus copies under `v<version>/`. No GitHub Release is created.

Every run publishes to R2 and also keeps the installers as Actions artifacts. CI runs only on pull requests and pushes to `main`, and smoke-tests debug builds; only the Release workflow builds the optimized binaries.

macOS bundles use ad-hoc signing by default. `scripts/bundle-macos.sh` accepts `HERMES_SIGNING_IDENTITY` when a signing identity is installed; notarization is not configured. Windows installers are unsigned. Linux requires a Wayland session and access to the system Secret Service; `.deb` declares its runtime libraries, while the AppImage bundles application libraries and uses the host graphics drivers.
