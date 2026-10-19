import type { Id } from "./_generated/dataModel";
import { z } from "zod";
import { v } from "convex/values";
import { internal } from "./_generated/api";
import { cancelSubscriptions } from "./billing";
import { action, internalMutation, internalQuery, mutation, query } from "./_generated/server";
import {
  DAY_MS,
  dayStart,
  accountState,
  ensureUser,
  requireIdentity,
  userForIdentity,
  weekStart,
  WEEK_MS,
  FREE_WEEKLY_WORDS,
} from "./lib";

export const storeProfile = internalMutation({
  args: { email: v.string(), name: v.string() },
  handler: async (ctx, profile) => (await ensureUser(ctx, profile))._id,
});
export const sync = action({
  args: {},
  returns: v.id("users"),
  handler: async (ctx): Promise<Id<"users">> => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error("Sign in required");
    if (identity.email)
      return ctx.runMutation(internal.viewer.storeProfile, {
        email: identity.email,
        name: identity.name ?? "",
      });
    const key = process.env.WORKOS_API_KEY;
    if (!key) throw new Error("Account sync is not configured");
    const response = await fetch(
      `https://api.workos.com/user_management/users/${encodeURIComponent(identity.subject)}`,
      {
        headers: { Authorization: `Bearer ${key}` },
      },
    );
    if (!response.ok) throw new Error("Could not load your sign-in account");
    const user = z
      .object({
        email: z.string(),
        first_name: z.string().nullable(),
        last_name: z.string().nullable(),
      })
      .parse(await response.json());
    return ctx.runMutation(internal.viewer.storeProfile, {
      email: user.email,
      name: [user.first_name, user.last_name].filter(Boolean).join(" "),
    });
  },
});
export const current = query({
  args: {},
  handler: async (ctx) => {
    const identity = await requireIdentity(ctx);
    const user = await userForIdentity(ctx);
    return {
      email: user?.email ?? identity.email ?? null,
      name: user?.name ?? identity.name ?? "",
      ...(user
        ? await accountState(ctx, user._id)
        : {
            plan: "free" as const,
            currentPeriodEnd: null,
            usage: { words: 0, cap: FREE_WEEKLY_WORDS, resetsAt: weekStart() + WEEK_MS },
          }),
    };
  },
});

export const rename = mutation({
  args: { name: v.string() },
  handler: async (ctx, { name }) => {
    const trimmed = name.trim();
    if (!trimmed || trimmed.length > 80)
      throw new Error("Name must be between 1 and 80 characters");
    const user = await ensureUser(ctx);
    await ctx.db.patch(user._id, { name: trimmed });
  },
});
export const history = query({
  args: {},
  handler: async (ctx) => {
    const user = await userForIdentity(ctx);
    const today = dayStart();
    const start = today - 29 * DAY_MS;
    const rows = user
      ? await ctx.db
          .query("daily")
          .withIndex("by_user_day", (q) =>
            q.eq("userId", user._id).gte("day", start).lte("day", today),
          )
          .collect()
      : [];
    const wordsByDay = new Map(rows.map((row) => [row.day, row.words]));
    const days = Array.from({ length: 30 }, (_, index) => {
      const day = start + index * DAY_MS;
      return { day, words: wordsByDay.get(day) ?? 0 };
    });
    return {
      days,
      recentWords: rows.reduce((total, row) => total + row.words, 0),
      totalWords: user?.totalWords ?? 0,
    };
  },
});
export const removalData = internalQuery({
  args: { subject: v.string() },
  handler: async (ctx, { subject }) => {
    const user = await ctx.db
      .query("users")
      .withIndex("by_subject", (q) => q.eq("subject", subject))
      .unique();
    if (!user) return null;
    const subscriptions = await ctx.db
      .query("subscriptions")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .collect();
    return { user, subscriptions };
  },
});
export const erase = internalMutation({
  args: { userId: v.id("users") },
  handler: async (ctx, { userId }) => {
    for (const table of ["daily", "desktopTokens", "subscriptions", "settings"] as const) {
      const rows =
        table === "daily"
          ? await ctx.db
              .query(table)
              .withIndex("by_user_day", (q) => q.eq("userId", userId))
              .collect()
          : await ctx.db
              .query(table)
              .withIndex("by_user", (q) => q.eq("userId", userId))
              .collect();
      for (const row of rows) await ctx.db.delete(row._id);
    }
    const feedback = await ctx.db
      .query("feedback")
      .withIndex("by_user", (q) => q.eq("userId", userId))
      .collect();
    for (const row of feedback) await ctx.db.delete(row._id);
    await ctx.db.delete(userId);
  },
});
export const remove = action({
  args: { email: v.string() },
  returns: v.null(),
  handler: async (ctx, { email }) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error("Sign in required");
    const data = await ctx.runQuery(internal.viewer.removalData, { subject: identity.subject });
    if (!data || !data.user.email || email !== data.user.email)
      throw new Error("Email does not match your account");
    const key = process.env.WORKOS_API_KEY;
    if (!key) throw new Error("Account deletion is not configured");
    await cancelSubscriptions(data.user.dodoCustomerId, data.subscriptions);
    const response = await fetch(
      `https://api.workos.com/user_management/users/${encodeURIComponent(identity.subject)}`,
      {
        method: "DELETE",
        headers: { Authorization: `Bearer ${key}` },
      },
    );
    if (!response.ok && response.status !== 404)
      throw new Error("Could not delete your sign-in account. Please try again.");
    await ctx.runMutation(internal.viewer.erase, { userId: data.user._id });
    return null;
  },
});
