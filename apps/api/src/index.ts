import { z } from "zod";
import { dictate, readInput } from "./dictate";

const accountInput = z.object({
  usage: z.object({ words: z.number(), cap: z.number().nullable(), resetsAt: z.number() }),
});
type Account = z.infer<typeof accountInput>;

// Convex is far from most users, so auth is cached per isolate to keep it off the dictation path.
// The desktop prewarms `/v1/me` when recording starts, which fills this cache before audio arrives.
const accounts = new Map<string, { account: Account; at: number }>();
const ACCOUNT_TTL_MS = 60_000;

function error(status: number, type: string, message: string, extra = {}) {
  return Response.json({ error: { type, message, ...extra } }, { status });
}

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

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const authorization = request.headers.get("Authorization") ?? "";
    if (url.pathname === "/v1/me" && request.method === "GET") return me(env, authorization);
    if (url.pathname !== "/v1/dictate")
      return fetch(new Request(`${env.CONVEX_SITE_URL}${url.pathname}${url.search}`, request));
    if (request.method !== "POST") return error(405, "request", "Method not allowed");

    const ip = request.headers.get("CF-Connecting-IP") ?? "";
    if (!(await env.DICTATE_LIMIT.limit({ key: ip })).success)
      return error(429, "rate_limit", "Too many requests");
    // Auth reaches Convex across an ocean, so it runs alongside the provider calls.
    // Nothing is returned until it passes; the rate limit bounds what bad tokens can spend.
    const auth = authenticate(env, authorization).catch(() => undefined);
    const input = await readInput(request);
    if (!input) return error(400, "request", "Invalid dictation input");
    const work = dictate(env.GROQ_API_KEY, input).catch(() => null);
    const account = await auth;
    if (account === undefined) return error(503, "configuration", "Dictation is unavailable");
    if (!account) return error(401, "authentication", "Invalid desktop token");
    if (account.usage.cap !== null && account.usage.words >= account.usage.cap)
      return error(402, "limit", "Weekly free limit reached", {
        resets_at: account.usage.resetsAt,
      });
    const result = await work;
    if (!result) return error(502, "provider", "Dictation provider is unavailable");
    const text = result.text.trim();
    const words = text ? text.split(/\s+/u).length : 0;
    account.usage.words += words;
    ctx.waitUntil(
      fetch(`${env.CONVEX_SITE_URL}/v1/usage`, {
        method: "POST",
        headers: { Authorization: authorization, "Content-Type": "application/json" },
        body: JSON.stringify({ words }),
      }),
    );
    return Response.json(result);
  },
} satisfies ExportedHandler<Env>;
