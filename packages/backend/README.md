# Hermes backend

Convex stores WorkOS users, Dodo subscriptions, hashed desktop tokens and weekly word totals. HTTP actions forward audio and cleanup requests to Groq without retaining their content. See the root README for API behavior and local setup.

Use an anonymous local deployment for development:

```bash
CONVEX_AGENT_MODE=anonymous bunx convex dev --once
CONVEX_AGENT_MODE=anonymous bunx convex env set WORKOS_CLIENT_ID client_local_unconfigured
CONVEX_AGENT_MODE=anonymous bunx convex dev
```

The first command may stop requesting the WorkOS client ID; set the placeholder and rerun. Replace it with the real client ID to verify authentication. No cloud deployment is needed for codegen or type checking.

Backend environment variables, also listed in `.env.example`:

- `WORKOS_CLIENT_ID`: WorkOS JWT client ID, same as the website.
- `GROQ_API_KEY`: hosted transcription and cleanup provider secret.
- `DODO_PAYMENTS_API_KEY`: Dodo API secret.
- `DODO_PAYMENTS_ENVIRONMENT`: `test_mode` for development or `live_mode` when explicitly deploying.
- `DODO_PAYMENTS_WEBHOOK_SECRET`: signature verification secret.
- `DODO_PRO_MONTHLY_PRODUCT_ID`: $12 monthly subscription product.
- `DODO_PRO_YEARLY_PRODUCT_ID`: $96 yearly subscription product.

Set values with `bunx convex env set NAME value` against the intended development deployment. Convex does not load these values from `.env.example`. Keep all secrets on the backend.

The official `@dodopayments/convex` component serves checkout and customer portal actions. Point Dodo subscription webhooks at `<CONVEX_SITE_URL>/dodopayments-webhook`, where `CONVEX_SITE_URL` is the HTTP actions URL, not the Convex client URL. `FREE_WEEKLY_WORDS` is defined once in `convex/lib.ts`.
