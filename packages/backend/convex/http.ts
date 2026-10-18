import { createDodoWebhookHandler, type Subscription } from "@dodopayments/convex";
import { httpRouter } from "convex/server";
import { internal } from "./_generated/api";
import { httpAction, type ActionCtx } from "./_generated/server";
import { hashToken } from "./lib";
import { settingsInput } from "./settings";
import { pingInput } from "./pings";
import { dictate, readInput } from "./dictate";

const http = httpRouter();
http.route({
  path: "/v1/ping",
  method: "POST",
  handler: httpAction(async (ctx, request) => {
    const maxBytes = 8 * 1024;
    if (Number(request.headers.get("Content-Length")) > maxBytes)
      return error(413, "request", "Ping exceeds 8 KB");
    const body = await request.text();
    if (new TextEncoder().encode(body).byteLength > maxBytes)
      return error(413, "request", "Ping exceeds 8 KB");
    let input: unknown;
    try {
      input = JSON.parse(body);
    } catch {
      return error(400, "request", "Expected JSON");
    }
    const parsed = pingInput.safeParse(input);
    if (!parsed.success) return error(400, "request", "Invalid ping");
    const status = await ctx.runMutation(internal.pings.record, parsed.data);
    if (!status.ok)
      return Response.json(
        { error: { type: "rate_limit", message: "Too many pings" } },
        {
          status: 429,
          headers: { "Retry-After": String(Math.ceil(status.retryAfter / 1000)) },
        },
      );
    return new Response(null, { status: 204 });
  }),
});
function error(status: number, type: string, message: string, extra = {}) {
  return Response.json({ error: { type, message, ...extra } }, { status });
}
async function authenticate(ctx: ActionCtx, request: Request) {
  const token = request.headers.get("Authorization")?.match(/^Bearer (hm_[A-Za-z0-9_-]{43})$/)?.[1];
  if (!token) return null;
  return ctx.runQuery(internal.proxy.lookup, { hash: await hashToken(token) });
}
function limit(account: NonNullable<Awaited<ReturnType<typeof authenticate>>>) {
  return account.usage.cap !== null && account.usage.words >= account.usage.cap
    ? error(402, "limit", "Weekly free limit reached", { resets_at: account.usage.resetsAt })
    : null;
}
http.route({
  path: "/v1/dictate",
  method: "POST",
  handler: httpAction(async (ctx, request) => {
    const startedAt = Date.now();
    const account = await authenticate(ctx, request);
    if (!account) return error(401, "authentication", "Invalid desktop token");
    const blocked = limit(account);
    if (blocked) return blocked;
    const input = await readInput(request);
    if (!input) return error(400, "request", "Invalid dictation input");
    const key = process.env.GROQ_API_KEY;
    if (!key) return error(503, "configuration", "Dictation is unavailable");
    let result: Awaited<ReturnType<typeof dictate>>;
    try {
      result = await dictate(key, input);
    } catch {
      return error(502, "provider", "Dictation provider is unavailable");
    }
    await ctx.scheduler.runAfter(0, internal.proxy.record, {
      tokenId: account.tokenId,
      startedAt,
      words: result.text.trim() ? result.text.trim().split(/\s+/u).length : 0,
    });
    return Response.json(result);
  }),
});
http.route({
  path: "/v1/me",
  method: "GET",
  handler: httpAction(async (ctx, request) => {
    const account = await authenticate(ctx, request);
    if (!account) return error(401, "authentication", "Invalid desktop token");
    await ctx.scheduler.runAfter(0, internal.proxy.record, {
      tokenId: account.tokenId,
      startedAt: Date.now(),
      words: 0,
    });
    return Response.json({ email: account.email, plan: account.plan, usage: account.usage });
  }),
});
for (const method of ["GET", "PUT"] as const) {
  http.route({
    path: "/v1/settings",
    method,
    handler: httpAction(async (ctx, request) => {
      const account = await authenticate(ctx, request);
      if (!account) return error(401, "authentication", "Invalid desktop token");
      if (method === "GET")
        return Response.json({
          settings: await ctx.runQuery(internal.settings.get, { userId: account.userId }),
        });
      const maxBytes = 256 * 1024;
      if (Number(request.headers.get("Content-Length")) > maxBytes)
        return error(400, "request", "Settings exceed 256 KB");
      let input: unknown;
      try {
        const body = await request.text();
        if (new TextEncoder().encode(body).byteLength > maxBytes)
          return error(400, "request", "Settings exceed 256 KB");
        input = JSON.parse(body);
      } catch {
        return error(400, "request", "Expected JSON");
      }
      const parsed = settingsInput.safeParse(input);
      if (!parsed.success) return error(400, "request", "Invalid settings");
      const result = await ctx.runMutation(internal.settings.put, {
        userId: account.userId,
        ...parsed.data,
      });
      if (!result.ok) return error(429, "rate_limit", "Too many settings updates");
      return Response.json({ settings: result.settings });
    }),
  });
}
async function saveSubscription(
  ctx: Pick<ActionCtx, "runMutation">,
  payload: { data: Subscription; timestamp: Date },
) {
  const data = payload.data;
  const status =
    data.status === "active"
      ? "active"
      : data.status === "cancelled"
        ? "cancelled"
        : data.status === "expired"
          ? "expired"
          : "on_hold";
  const subject: unknown = data.metadata.typeaside_subject;
  await ctx.runMutation(internal.billing.subscription, {
    subject: typeof subject === "string" ? subject : undefined,
    customerId: data.customer.customer_id,
    subscriptionId: data.subscription_id,
    status,
    currentPeriodEnd: data.next_billing_date.getTime(),
    eventAt: payload.timestamp.getTime(),
  });
}
http.route({
  path: "/dodopayments-webhook",
  method: "POST",
  handler: createDodoWebhookHandler({
    onSubscriptionActive: saveSubscription,
    onSubscriptionOnHold: saveSubscription,
    onSubscriptionCancelled: saveSubscription,
    onSubscriptionExpired: saveSubscription,
    onSubscriptionRenewed: saveSubscription,
    onSubscriptionUpdated: saveSubscription,
    onSubscriptionPlanChanged: saveSubscription,
    onSubscriptionFailed: saveSubscription,
    onSubscriptionPaused: saveSubscription,
    onSubscriptionUnpaused: saveSubscription,
  }),
});
export default http;
