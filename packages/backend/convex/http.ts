import { createDodoWebhookHandler, type Subscription } from "@dodopayments/convex";
import { httpRouter } from "convex/server";
import { internal } from "./_generated/api";
import { httpAction, type ActionCtx } from "./_generated/server";
import { hashToken } from "./lib";

const http = httpRouter();
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
  for (const field of ["language", "prompt", "response_format"]) {
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
    const format = body.get("response_format") ?? "json";
    let text = raw;
    if (format === "json" || format === "verbose_json") {
      try {
        const result: unknown = JSON.parse(raw);
        text =
          typeof result === "object" &&
          result !== null &&
          "text" in result &&
          typeof result.text === "string"
            ? result.text
            : "";
      } catch {
        return error(502, "provider", "Invalid transcription response");
      }
    } else if (format === "srt" || format === "vtt") {
      text = raw
        .split("\n")
        .filter((line) => !/^\s*(?:\d+|WEBVTT)?\s*$/.test(line) && !line.includes("-->"))
        .join(" ");
    }
    await ctx.runMutation(internal.proxy.record, {
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
  const startedAt = Date.now();
  const account = await authenticate(ctx, request);
  if (!account) return error(401, "authentication", "Invalid desktop token");
  const blocked = limit(account);
  if (blocked) return blocked;
  const key = process.env.GROQ_API_KEY;
  if (!key) return error(503, "configuration", "Text cleanup is unavailable");
  let input: unknown;
  try {
    input = await request.json();
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
  const body: Record<string, unknown> = {
    model: "qwen/qwen3.8-27b",
    messages: input.messages,
    stream: false,
  };
  for (const field of ["temperature", "max_tokens", "reasoning_effort"] as const) {
    const value: unknown = Reflect.get(input, field);
    if (value !== undefined) body[field] = value;
  }
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
    await ctx.runMutation(internal.proxy.record, { tokenId: account.tokenId, startedAt, words: 0 });
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
    await ctx.runMutation(internal.proxy.record, {
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
  const subject: unknown = data.metadata.hermes_subject;
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
