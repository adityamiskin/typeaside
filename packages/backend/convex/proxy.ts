import { v } from "convex/values";
import { internalQuery, internalMutation } from "./_generated/server";
import { accountState, dayStart } from "./lib";
import { addToRollup } from "./rollups";

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
    const start = dayStart(startedAt);
    const user = await ctx.db.get(token.userId);
    if (!user) return;
    const usage = await ctx.db
      .query("daily")
      .withIndex("by_user_day", (q) => q.eq("userId", token.userId).eq("day", start))
      .unique();
    await ctx.db.patch(user._id, { totalWords: (user.totalWords ?? 0) + words });
    if (usage) await ctx.db.patch(usage._id, { words: usage.words + words });
    else await ctx.db.insert("daily", { userId: token.userId, day: start, words });
    await addToRollup(ctx, start, { hostedWords: words, activeUsers: usage ? 0 : 1 });
  },
});
