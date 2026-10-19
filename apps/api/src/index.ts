import { z } from "zod";
import { dictate, readInput, withinBodyLimit, type Result } from "./dictate";
import { stream } from "./stream";
import { error } from "./errors";

const accountInput = z.object({
  plan: z.enum(["free", "pro"]),
  usage: z.object({ words: z.number(), cap: z.number().nullable(), resetsAt: z.number() }),
});
type Account = z.infer<typeof accountInput>;

// Convex is far from most users, so auth is cached per isolate to keep it off the dictation path.
const accounts = new Map<string, { account: Account; at: number }>();
const ACCOUNT_TTL_MS = 15_000;

const streams = new Map<string, number>();
const MAX_STREAMS_PER_TOKEN = 2;

async function me(env: Env, authorization: string) {
  const response = await fetch(`${env.CONVEX_SITE_URL}/v1/me`, {
    headers: { Authorization: authorization },
  });
  const account = response.ok
    ? accountInput.safeParse(await response.clone().json()).data
    : undefined;
  if (account) accounts.set(authorization, { account, at: Date.now() });
  else accounts.delete(authorization);
  return response;
}

async function authenticate(env: Env, authorization: string) {
  const cached = accounts.get(authorization);
  if (cached && Date.now() - cached.at < ACCOUNT_TTL_MS) return cached.account;
  const response = await me(env, authorization);
  if (response.status === 401) return null;
  const account = accounts.get(authorization)?.account;
  if (!account) throw new Error("Account lookup failed");
  return account;
}

type Failure = ReturnType<typeof error>;

async function authorize(env: Env, authorization: string): Promise<Account | Failure> {
  const account = await authenticate(env, authorization).catch(() => undefined);
  if (account === undefined) return error(503, "configuration", "Dictation is unavailable");
  if (!account) return error(401, "authentication", "Invalid desktop token");
  if (account.usage.cap !== null && account.usage.words >= account.usage.cap)
    return error(402, "limit", "Weekly free limit reached", { resets_at: account.usage.resetsAt });
  return account;
}

function failed(failure: Failure) {
  return Response.json({ error: failure.error }, { status: failure.status });
}

async function finish(
  work: Promise<Result | null>,
  account: Account,
  env: Env,
  authorization: string,
  ctx: ExecutionContext,
): Promise<(NonNullable<Result> & { usage: Account["usage"] }) | Failure> {
  const result = await work;
  if (!result) return error(502, "provider", "Dictation provider is unavailable");
  const text = result.text.trim();
  const words = text ? text.split(/\s+/u).length : 0;
  account.usage.words += words;
  ctx.waitUntil(reportUsage(env, authorization, words));
  return { ...result, usage: account.usage };
}

async function reportUsage(env: Env, authorization: string, words: number) {
  let status: number | undefined;
  for (let attempt = 0; attempt < 2; attempt++) {
    const response = await fetch(`${env.CONVEX_SITE_URL}/v1/usage`, {
      method: "POST",
      headers: { Authorization: authorization, "Content-Type": "application/json" },
      body: JSON.stringify({ words }),
    }).catch(() => undefined);
    status = response?.status;
    if (status !== undefined && status < 500) break;
  }
  if (status === undefined || status >= 400)
    console.error(`Usage report failed (${status ?? "network error"}), ${words} words unmetered`);
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const authorization = request.headers.get("Authorization") ?? "";
    const streaming = url.pathname === "/v1/stream";
    const dictating = streaming || url.pathname === "/v1/dictate";

    const ip = request.headers.get("CF-Connecting-IP") ?? "";
    const limiter = dictating ? env.DICTATE_LIMIT : env.FORWARD_LIMIT;
    if (!(await limiter.limit({ key: ip })).success)
      return failed(error(429, "rate_limit", "Too many requests"));

    if (url.pathname === "/v1/me" && request.method === "GET") return me(env, authorization);
    if (!dictating)
      return fetch(new Request(`${env.CONVEX_SITE_URL}${url.pathname}${url.search}`, request));
    if (request.method !== (streaming ? "GET" : "POST"))
      return failed(error(405, "request", "Method not allowed"));

    if (streaming) {
      // Awaited before upgrading: the user is still speaking, so this is off the critical path.
      const account = await authorize(env, authorization);
      if ("error" in account) return failed(account);
      if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket")
        return failed(error(400, "request", "Expected WebSocket upgrade"));
      const active = streams.get(authorization) ?? 0;
      if (active >= MAX_STREAMS_PER_TOKEN)
        return failed(error(429, "rate_limit", "Too many active streams"));
      streams.set(authorization, active + 1);
      return stream(
        ctx,
        (input) =>
          finish(
            dictate(env, input, account.plan === "pro").catch(() => null),
            account,
            env,
            authorization,
            ctx,
          ),
        () => {
          const left = (streams.get(authorization) ?? 1) - 1;
          if (left > 0) streams.set(authorization, left);
          else streams.delete(authorization);
        },
      );
    }
    if (!withinBodyLimit(request)) return failed(error(413, "request", "Audio exceeds 20 MB"));
    // Provider calls wait for auth so invalid or capped tokens spend nothing. The body upload overlaps the lookup.
    const [account, input] = await Promise.all([authorize(env, authorization), readInput(request)]);
    if ("error" in account) return failed(account);
    if (!input) return failed(error(400, "request", "Invalid dictation input"));
    const work = dictate(env, input, account.plan === "pro").catch(() => null);
    const result = await finish(work, account, env, authorization, ctx);
    return "error" in result ? failed(result) : Response.json(result);
  },
} satisfies ExportedHandler<Env>;
