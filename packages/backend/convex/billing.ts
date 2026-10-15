import { DodoPayments } from "@dodopayments/convex";
import { v } from "convex/values";
import { components, internal } from "./_generated/api";
import { action, internalMutation, internalQuery } from "./_generated/server";
import { ensureUser } from "./lib";
import { subscriptionStatus } from "./schema";

export const customer = internalQuery({
  args: { subject: v.string() },
  handler: (ctx, { subject }) =>
    ctx.db
      .query("users")
      .withIndex("by_subject", (q) => q.eq("subject", subject))
      .unique(),
});
export const prepare = internalMutation({ args: {}, handler: (ctx) => ensureUser(ctx) });
function payments() {
  const apiKey = process.env.DODO_PAYMENTS_API_KEY;
  const environment = process.env.DODO_PAYMENTS_ENVIRONMENT;
  if (!apiKey || (environment !== "test_mode" && environment !== "live_mode"))
    throw new Error("Billing is not configured");
  return new DodoPayments(components.dodopayments, {
    apiKey,
    environment,
    identify: async (ctx) => {
      const identity = await ctx.auth.getUserIdentity();
      if (!identity) return null;
      const user = await ctx.runQuery(internal.billing.customer, { subject: identity.subject });
      return user?.dodoCustomerId ? { dodoCustomerId: user.dodoCustomerId } : null;
    },
  }).api();
}
export const checkout = action({
  args: { interval: v.union(v.literal("monthly"), v.literal("yearly")) },
  handler: async (ctx, { interval }): Promise<{ checkout_url: string }> => {
    const user = await ctx.runMutation(internal.billing.prepare, {});
    if (!user.email) throw new Error("An email address is required for billing");
    const product =
      interval === "monthly"
        ? process.env.DODO_PRO_MONTHLY_PRODUCT_ID
        : process.env.DODO_PRO_YEARLY_PRODUCT_ID;
    if (!product) throw new Error("This billing interval is not configured");
    return payments().checkout(ctx, {
      payload: {
        product_cart: [{ product_id: product, quantity: 1 }],
        customer: user.dodoCustomerId
          ? { customer_id: user.dodoCustomerId }
          : { email: user.email, name: user.name ?? user.email },
        metadata: { hermes_subject: user.subject },
      },
    });
  },
});
export const portal = action({
  args: {},
  handler: async (ctx): Promise<{ portal_url: string }> => {
    await ctx.runMutation(internal.billing.prepare, {});
    return payments().customerPortal(ctx, { send_email: false });
  },
});
export const subscription = internalMutation({
  args: {
    subject: v.optional(v.string()),
    customerId: v.string(),
    subscriptionId: v.string(),
    status: subscriptionStatus,
    currentPeriodEnd: v.number(),
    eventAt: v.number(),
  },
  handler: async (ctx, args) => {
    const existing = await ctx.db
      .query("subscriptions")
      .withIndex("by_subscription", (q) => q.eq("subscriptionId", args.subscriptionId))
      .unique();
    if (existing && existing.eventAt >= args.eventAt) return;
    const subject = args.subject;
    const user = subject
      ? await ctx.db
          .query("users")
          .withIndex("by_subject", (q) => q.eq("subject", subject))
          .unique()
      : existing
        ? await ctx.db.get(existing.userId)
        : null;
    if (!user) throw new Error("Subscription does not match a Hermes account");
    if (user.dodoCustomerId !== args.customerId)
      await ctx.db.patch(user._id, { dodoCustomerId: args.customerId });
    const fields = {
      userId: user._id,
      subscriptionId: args.subscriptionId,
      status: args.status,
      currentPeriodEnd: args.currentPeriodEnd,
      eventAt: args.eventAt,
    };
    if (existing) await ctx.db.patch(existing._id, fields);
    else await ctx.db.insert("subscriptions", fields);
  },
});
