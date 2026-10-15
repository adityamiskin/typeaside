import type { MutationCtx, QueryCtx } from "./_generated/server";
import type { Id } from "./_generated/dataModel";

export const FREE_WEEKLY_WORDS = 2000;
export const WEEK_MS = 7 * 24 * 60 * 60 * 1000;
export function weekStart(now = Date.now()) {
  const date = new Date(now);
  date.setUTCHours(0, 0, 0, 0);
  date.setUTCDate(date.getUTCDate() - ((date.getUTCDay() + 6) % 7));
  return date.getTime();
}
export async function requireIdentity(ctx: QueryCtx | MutationCtx) {
  const identity = await ctx.auth.getUserIdentity();
  if (!identity) throw new Error("Sign in required");
  return identity;
}
export async function ensureUser(ctx: MutationCtx) {
  const identity = await requireIdentity(ctx);
  const existing = await ctx.db
    .query("users")
    .withIndex("by_subject", (q) => q.eq("subject", identity.subject))
    .unique();
  const fields = { subject: identity.subject, email: identity.email, name: identity.name };
  if (existing) {
    await ctx.db.patch(existing._id, fields);
    return { ...existing, ...fields };
  }
  const id = await ctx.db.insert("users", fields);
  return { _id: id, ...fields, dodoCustomerId: undefined };
}
export async function userForIdentity(ctx: QueryCtx) {
  const identity = await requireIdentity(ctx);
  return ctx.db
    .query("users")
    .withIndex("by_subject", (q) => q.eq("subject", identity.subject))
    .unique();
}
export async function accountState(ctx: QueryCtx, userId: Id<"users">) {
  const start = weekStart();
  const [subscriptions, usage] = await Promise.all([
    ctx.db
      .query("subscriptions")
      .withIndex("by_user", (q) => q.eq("userId", userId))
      .collect(),
    ctx.db
      .query("usage")
      .withIndex("by_user_week", (q) => q.eq("userId", userId).eq("weekStart", start))
      .unique(),
  ]);
  const active = subscriptions
    .filter((s) => s.status === "active")
    .sort((a, b) => b.currentPeriodEnd - a.currentPeriodEnd)[0];
  return {
    plan: active ? ("pro" as const) : ("free" as const),
    currentPeriodEnd: active?.currentPeriodEnd ?? null,
    usage: {
      words: usage?.words ?? 0,
      cap: active ? null : FREE_WEEKLY_WORDS,
      resetsAt: start + WEEK_MS,
    },
  };
}
export async function hashToken(token: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(token));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}
