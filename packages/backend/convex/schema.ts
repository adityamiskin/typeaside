import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export const subscriptionStatus = v.union(
  v.literal("active"),
  v.literal("on_hold"),
  v.literal("failed"),
  v.literal("cancelled"),
  v.literal("expired"),
);

export const operatingSystem = v.union(
  v.literal("linux"),
  v.literal("macos"),
  v.literal("windows"),
);
export const feedbackKind = v.union(v.literal("bug"), v.literal("idea"), v.literal("other"));
export const pingFields = {
  installId: v.string(),
  day: v.number(),
  version: v.string(),
  os: operatingSystem,
  source: v.union(v.literal("hosted"), v.literal("own"), v.literal("local")),
  dictations: v.number(),
  words: v.number(),
  commands: v.number(),
  latencyMs: v.number(),
};

export const rollupFields = {
  day: v.number(),
  activeInstalls: v.number(),
  dictations: v.number(),
  words: v.number(),
  newUsers: v.number(),
  activeUsers: v.number(),
  hostedWords: v.number(),
};

export const settingsFields = {
  dictionary: v.array(v.string()),
  removedWords: v.array(v.string()),
  snippets: v.array(v.object({ trigger: v.string(), text: v.string() })),
  styles: v.record(
    v.string(),
    v.union(v.literal("formal"), v.literal("casual"), v.literal("very_casual")),
  ),
  // Rows synced before app instructions existed lack the field; clients send it on every write.
  appInstructions: v.optional(v.array(v.object({ app: v.string(), text: v.string() }))),
  updatedAt: v.number(),
};

export default defineSchema({
  settings: defineTable({ userId: v.id("users"), ...settingsFields }).index("by_user", ["userId"]),
  pings: defineTable(pingFields)
    .index("by_install_day", ["installId", "day"])
    .index("by_day", ["day"]),
  feedback: defineTable({
    kind: feedbackKind,
    message: v.string(),
    email: v.optional(v.string()),
    userId: v.optional(v.id("users")),
    version: v.optional(v.string()),
    os: v.optional(operatingSystem),
    createdAt: v.number(),
  })
    .index("by_created_at", ["createdAt"])
    .index("by_user", ["userId"]),
  users: defineTable({
    subject: v.string(),
    email: v.optional(v.string()),
    name: v.optional(v.string()),
    totalWords: v.optional(v.number()),
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
    .index("by_subscription", ["subscriptionId"])
    .index("by_status", ["status"]),
  desktopTokens: defineTable({
    userId: v.id("users"),
    hash: v.string(),
    name: v.string(),
    createdAt: v.number(),
    lastUsedAt: v.optional(v.number()),
  })
    .index("by_hash", ["hash"])
    .index("by_user", ["userId"]),
  rollups: defineTable(rollupFields).index("by_day", ["day"]),
  daily: defineTable({ userId: v.id("users"), day: v.number(), words: v.number() })
    .index("by_user_day", ["userId", "day"])
    .index("by_day", ["day"]),
});
