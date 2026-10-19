import type { MutationCtx, QueryCtx } from "./_generated/server";
import type { Doc, Id } from "./_generated/dataModel";
import { addToRollup } from "./rollups";

export const FREE_WEEKLY_WORDS = 2000;
export const DAY_MS = 24 * 60 * 60 * 1000;
export const WEEK_MS = 7 * DAY_MS;
export function dayStart(now = Date.now()) {
  return Math.floor(now / DAY_MS) * DAY_MS;
}
// Renewal webhooks can land after the period ends; the grace keeps renewing users on Pro meanwhile.
const RENEWAL_GRACE_MS = 3 * DAY_MS;
export function isPro(subscription: Doc<"subscriptions">, now = Date.now()) {
  return subscription.status === "active" && subscription.currentPeriodEnd + RENEWAL_GRACE_MS > now;
}
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
export async function ensureUser(ctx: MutationCtx, profile?: { email: string; name: string }) {
  const identity = await requireIdentity(ctx);
  const existing = await ctx.db
    .query("users")
    .withIndex("by_subject", (q) => q.eq("subject", identity.subject))
    .unique();
  const fields = {
    subject: identity.subject,
    email: profile?.email ?? identity.email,
    name: profile?.name ?? identity.name,
  };
  if (existing) {
    const updates = {
      email: fields.email ?? existing.email,
      name: existing.name?.trim() ? existing.name : fields.name,
    };
    await ctx.db.patch(existing._id, updates);
    return { ...existing, ...updates };
  }
  const id = await ctx.db.insert("users", fields);
  await addToRollup(ctx, dayStart(), { newUsers: 1 });
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
      .query("daily")
      .withIndex("by_user_day", (q) =>
        q
          .eq("userId", userId)
          .gte("day", start)
          .lt("day", start + WEEK_MS),
      )
      .collect(),
  ]);
  const active = subscriptions
    .filter((s) => isPro(s))
    .sort((a, b) => b.currentPeriodEnd - a.currentPeriodEnd)[0];
  return {
    plan: active ? ("pro" as const) : ("free" as const),
    currentPeriodEnd: active?.currentPeriodEnd ?? null,
    usage: {
      words: usage.reduce((total, day) => total + day.words, 0),
      cap: active ? null : FREE_WEEKLY_WORDS,
      resetsAt: start + WEEK_MS,
    },
  };
}
export async function hashToken(token: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(token));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}
