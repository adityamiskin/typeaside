import { v } from "convex/values";
import { z } from "zod";
import { internalMutation, internalQuery } from "./_generated/server";
import { settingsFields } from "./schema";
import { rateLimiter } from "./rateLimits";

export const settingsInput = z.strictObject({
  dictionary: z.array(z.string().max(100)).max(2000),
  removedWords: z.array(z.string().max(100)).max(2000),
  snippets: z
    .array(z.strictObject({ trigger: z.string().max(100), text: z.string().max(4000) }))
    .max(500),
  styles: z.record(z.string(), z.enum(["formal", "casual", "very_casual"])),
  appInstructions: z
    .array(z.strictObject({ app: z.string().max(512), text: z.string().max(300) }))
    .max(20),
  updatedAt: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
});
export const get = internalQuery({
  args: { userId: v.id("users") },
  handler: (ctx, { userId }) =>
    ctx.db
      .query("settings")
      .withIndex("by_user", (q) => q.eq("userId", userId))
      .unique(),
});
export const put = internalMutation({
  args: { userId: v.id("users"), ...settingsFields },
  handler: async (ctx, input) => {
    const status = await rateLimiter.limit(ctx, "settings", { key: input.userId });
    if (!status.ok) return { ok: false as const };
    const stored = await ctx.db
      .query("settings")
      .withIndex("by_user", (q) => q.eq("userId", input.userId))
      .unique();
    if (stored && stored.updatedAt >= input.updatedAt)
      return { ok: true as const, settings: stored };
    if (stored) {
      await ctx.db.replace(stored._id, input);
      return { ok: true as const, settings: await ctx.db.get(stored._id) };
    }
    const id = await ctx.db.insert("settings", input);
    return { ok: true as const, settings: await ctx.db.get(id) };
  },
});
