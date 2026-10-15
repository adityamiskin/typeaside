import { ConvexError, v } from "convex/values";
import { internalQuery, mutation } from "./_generated/server";
import { ensureUser } from "./lib";
import { feedbackKind, operatingSystem } from "./schema";
import { rateLimiter } from "./rateLimits";

export const send = mutation({
  args: {
    kind: feedbackKind,
    message: v.string(),
    email: v.optional(v.string()),
    version: v.optional(v.string()),
    os: v.optional(operatingSystem),
  },
  handler: async (ctx, args) => {
    const message = args.message.trim();
    if (!message || message.length > 4000)
      throw new ConvexError("Message must contain 1 to 4000 characters.");
    const version = args.version?.trim();
    if (version !== undefined && (!version || version.length > 64))
      throw new ConvexError("Version must contain 1 to 64 characters.");
    const identity = await ctx.auth.getUserIdentity();
    const user = identity ? await ensureUser(ctx) : undefined;
    const email = user ? user.email : args.email?.trim() || undefined;
    if (!user && email && (email.length > 254 || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)))
      throw new ConvexError("Enter a valid email address.");
    const status = user
      ? await rateLimiter.limit(ctx, "feedbackUser", { key: user._id })
      : await rateLimiter.limit(ctx, "feedbackAnonymous");
    if (!status.ok) throw new ConvexError("Too many notes. Please try again later.");
    await ctx.db.insert("feedback", {
      kind: args.kind,
      message,
      email,
      userId: user?._id,
      version,
      os: args.os,
      createdAt: Date.now(),
    });
    return null;
  },
});

export const list = internalQuery({
  args: { limit: v.optional(v.number()) },
  handler: async (ctx, { limit = 50 }) => {
    if (!Number.isInteger(limit) || limit < 1 || limit > 1000)
      throw new Error("limit must be an integer from 1 to 1000");
    return ctx.db.query("feedback").withIndex("by_created_at").order("desc").take(limit);
  },
});
