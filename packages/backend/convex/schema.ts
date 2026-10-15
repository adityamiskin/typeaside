import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export const subscriptionStatus = v.union(
  v.literal("active"),
  v.literal("on_hold"),
  v.literal("cancelled"),
  v.literal("expired"),
);

export default defineSchema({
  users: defineTable({
    subject: v.string(),
    email: v.optional(v.string()),
    name: v.optional(v.string()),
    dodoCustomerId: v.optional(v.string()),
  }).index("by_subject", ["subject"]),
  subscriptions: defineTable({
    userId: v.id("users"),
    subscriptionId: v.string(),
    status: subscriptionStatus,
    currentPeriodEnd: v.number(),
    eventAt: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_subscription", ["subscriptionId"]),
  desktopTokens: defineTable({
    userId: v.id("users"),
    hash: v.string(),
    name: v.string(),
    createdAt: v.number(),
    lastUsedAt: v.optional(v.number()),
  })
    .index("by_hash", ["hash"])
    .index("by_user", ["userId"]),
  usage: defineTable({ userId: v.id("users"), weekStart: v.number(), words: v.number() }).index(
    "by_user_week",
    ["userId", "weekStart"],
  ),
});
