import { v } from "convex/values";
import { internalQuery, internalMutation } from "./_generated/server";
import { accountState, weekStart } from "./lib";

export const lookup = internalQuery({
  args: { hash: v.string() },
  handler: async (ctx, { hash }) => {
    const token = await ctx.db
      .query("desktopTokens")
      .withIndex("by_hash", (q) => q.eq("hash", hash))
      .unique();
    if (!token) return null;
    const user = await ctx.db.get(token.userId);
    if (!user) return null;
    return {
      tokenId: token._id,
      userId: user._id,
      email: user.email ?? null,
      ...(await accountState(ctx, user._id)),
    };
  },
});
export const record = internalMutation({
  args: { tokenId: v.id("desktopTokens"), words: v.number(), startedAt: v.number() },
  handler: async (ctx, { tokenId, words, startedAt }) => {
    const token = await ctx.db.get(tokenId);
    if (!token) return;
    await ctx.db.patch(tokenId, { lastUsedAt: Date.now() });
    if (words === 0) return;
    const start = weekStart(startedAt);
    const usage = await ctx.db
      .query("usage")
      .withIndex("by_user_week", (q) => q.eq("userId", token.userId).eq("weekStart", start))
      .unique();
    if (usage) await ctx.db.patch(usage._id, { words: usage.words + words });
    else await ctx.db.insert("usage", { userId: token.userId, weekStart: start, words });
  },
});
