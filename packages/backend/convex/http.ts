import { createDodoWebhookHandler, type Subscription } from "@dodopayments/convex";
import { httpRouter } from "convex/server";
import { internal } from "./_generated/api";
import { httpAction, type ActionCtx } from "./_generated/server";
import { hashToken } from "./lib";
import { pingInput } from "./pings";

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
const MAX_CHAT_BODY_BYTES = 64 * 1024;
const MAX_CHAT_TOKENS = 2048;
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
const transcription = httpAction(async (ctx, request) => {
  const startedAt = Date.now();
  const account = await authenticate(ctx, request);
  if (!account) return error(401, "authentication", "Invalid desktop token");
  const blocked = limit(account);
  if (blocked) return blocked;
  const key = process.env.GROQ_API_KEY;
  if (!key) return error(503, "configuration", "Transcription is unavailable");
  let incoming: FormData;
  try {
    incoming = await request.formData();
  } catch {
    return error(400, "request", "Expected multipart form data");
  }
  const file = incoming.get("file");
  if (!(file instanceof Blob)) return error(400, "request", "An audio file is required");
  const body = new FormData();
  body.set("file", file, file instanceof File ? file.name : "audio.wav");
  body.set("model", "whisper-large-v3-turbo");
  body.set("response_format", "json");
  for (const field of ["language", "prompt"]) {
    const value = incoming.get(field);
    if (typeof value === "string") body.set(field, value);
  }
  let upstream: Response;
  try {
    upstream = await fetch("https://api.groq.com/openai/v1/audio/transcriptions", {
      method: "POST",
      headers: { Authorization: `Bearer ${key}` },
      body,
    });
  } catch {
    return error(502, "provider", "Transcription provider is unavailable");
  }
  const raw = await upstream.text();
  if (upstream.ok) {
    let text: string;
    try {
      const result: unknown = JSON.parse(raw);
      if (
        typeof result !== "object" ||
        result === null ||
        !("text" in result) ||
        typeof result.text !== "string"
      ) {
        return error(502, "provider", "Invalid transcription response");
      }
      text = result.text;
    } catch {
      return error(502, "provider", "Invalid transcription response");
    }
    await ctx.scheduler.runAfter(0, internal.proxy.record, {
      tokenId: account.tokenId,
      startedAt,
      words: text.trim() ? text.trim().split(/\s+/u).length : 0,
    });
  }
  return new Response(raw, {
    status: upstream.status,
    headers: { "Content-Type": upstream.headers.get("Content-Type") ?? "application/json" },
  });
});
const chat = httpAction(async (ctx, request) => {
  if (Number(request.headers.get("Content-Length")) > MAX_CHAT_BODY_BYTES) {
    return error(413, "request", "Chat request exceeds 64 KB");
  }
  const startedAt = Date.now();
  const account = await authenticate(ctx, request);
  if (!account) return error(401, "authentication", "Invalid desktop token");
  const blocked = limit(account);
  if (blocked) return blocked;
  let input: unknown;
  try {
    const raw = await request.text();
    if (
      raw.length > MAX_CHAT_BODY_BYTES ||
      new TextEncoder().encode(raw).byteLength > MAX_CHAT_BODY_BYTES
    ) {
      return error(413, "request", "Chat request exceeds 64 KB");
    }
    input = JSON.parse(raw);
  } catch {
    return error(400, "request", "Expected JSON");
  }
  if (
    typeof input !== "object" ||
    input === null ||
    !("messages" in input) ||
    !Array.isArray(input.messages)
  )
    return error(400, "request", "Messages are required");
  if ("stream" in input && input.stream) return error(400, "request", "Streaming is not supported");
  const maxTokens: unknown = Reflect.get(input, "max_tokens");
  if (
    maxTokens !== undefined &&
    (typeof maxTokens !== "number" || !Number.isInteger(maxTokens) || maxTokens < 1)
  ) {
    return error(400, "request", "max_tokens must be a positive integer");
  }
  const body: Record<string, unknown> = {
    model: "qwen/qwen3.8-27b",
    messages: input.messages,
    stream: false,
    max_tokens:
      typeof maxTokens === "number" ? Math.min(maxTokens, MAX_CHAT_TOKENS) : MAX_CHAT_TOKENS,
  };
  for (const field of ["temperature", "reasoning_effort"] as const) {
    const value: unknown = Reflect.get(input, field);
    if (value !== undefined) body[field] = value;
  }
  const key = process.env.GROQ_API_KEY;
  if (!key) return error(503, "configuration", "Text cleanup is unavailable");
  let response: Response;
  try {
    response = await fetch("https://api.groq.com/openai/v1/chat/completions", {
      method: "POST",
      headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
  } catch {
    return error(502, "provider", "Text cleanup provider is unavailable");
  }
  if (response.ok)
    await ctx.scheduler.runAfter(0, internal.proxy.record, {
      tokenId: account.tokenId,
      startedAt,
      words: 0,
    });
  return new Response(response.body, {
    status: response.status,
    headers: { "Content-Type": response.headers.get("Content-Type") ?? "application/json" },
  });
});
http.route({ path: "/openai/v1/audio/transcriptions", method: "POST", handler: transcription });
http.route({ path: "/openai/v1/chat/completions", method: "POST", handler: chat });
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
