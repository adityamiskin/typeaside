import { v } from "convex/values";
import { mutation, query } from "./_generated/server";
import { ensureUser, hashToken, userForIdentity } from "./lib";

export const createToken = mutation({
  args: { name: v.string() },
  handler: async (ctx, { name }) => {
    const user = await ensureUser(ctx);
    if (!name.trim() || name.length > 100)
      throw new Error("Device name must be 1 to 100 characters");
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    const token = `hm_${btoa(String.fromCharCode(...bytes))
      .replaceAll("+", "-")
      .replaceAll("/", "_")
      .replace(/=+$/, "")}`;
    await ctx.db.insert("desktopTokens", {
      userId: user._id,
      hash: await hashToken(token),
      name: name.trim(),
      createdAt: Date.now(),
    });
    return token;
  },
});
export const list = query({
  args: {},
  handler: async (ctx) => {
    const user = await userForIdentity(ctx);
    if (!user) return [];
    const tokens = await ctx.db
      .query("desktopTokens")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .collect();
    return tokens.map(({ _id, name, createdAt, lastUsedAt }) => ({
      _id,
      name,
      createdAt,
      lastUsedAt,
    }));
  },
});
export const revoke = mutation({
  args: { id: v.id("desktopTokens") },
  handler: async (ctx, { id }) => {
    const user = await ensureUser(ctx);
    const token = await ctx.db.get(id);
    if (!token || token.userId !== user._id) throw new Error("Device not found");
    await ctx.db.delete(id);
  },
});
