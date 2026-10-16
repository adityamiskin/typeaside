import { DodoPayments } from "@dodopayments/convex";
import { z } from "zod";
import type { ActionCtx } from "./_generated/server";
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
        metadata: { typeaside_subject: user.subject },
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
    if (!user) throw new Error("Subscription does not match a TypeAside account");
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

async function dodoRequest(path: string, init: RequestInit = {}) {
  const key = process.env.DODO_PAYMENTS_API_KEY;
  const environment = process.env.DODO_PAYMENTS_ENVIRONMENT;
  if (!key || (environment !== "test_mode" && environment !== "live_mode"))
    throw new Error("Billing is not configured");
  const host = environment === "test_mode" ? "test.dodopayments.com" : "live.dodopayments.com";
  const headers = new Headers(init.headers);
  headers.set("Authorization", `Bearer ${key}`);
  headers.set("Content-Type", "application/json");
  const response = await fetch(`https://${host}${path}`, { ...init, headers });
  if (!response.ok) throw new Error("Billing request failed. Please try again.");
  return response;
}
const payment = z.object({
  payment_id: z.string(),
  created_at: z.string(),
  total_amount: z.number(),
  currency: z.string(),
  status: z.string().nullable().optional(),
  customer: z.object({ customer_id: z.string() }),
});
async function callerCustomer(ctx: ActionCtx) {
  const identity = await ctx.auth.getUserIdentity();
  if (!identity) throw new Error("Sign in required");
  return ctx.runQuery(internal.billing.customer, { subject: identity.subject });
}
export const invoices = action({
  args: {},
  returns: v.array(
    v.object({
      paymentId: v.string(),
      date: v.string(),
      amount: v.number(),
      currency: v.string(),
      status: v.string(),
    }),
  ),
  handler: async (ctx) => {
    const user = await callerCustomer(ctx);
    if (!user?.dodoCustomerId) return [];
    const result = [];
    for (let page = 1; ; page++) {
      const response = await dodoRequest(
        `/payments?customer_id=${encodeURIComponent(user.dodoCustomerId)}&page_size=100&page_number=${page}`,
      );
      const { items } = z.object({ items: z.array(payment) }).parse(await response.json());
      for (const item of items) {
        if (item.customer.customer_id === user.dodoCustomerId)
          result.push({
            paymentId: item.payment_id,
            date: item.created_at,
            amount: item.total_amount,
            currency: item.currency,
            status: item.status ?? "processing",
          });
      }
      if (items.length < 100) break;
    }
    return result.sort((a, b) => Date.parse(b.date) - Date.parse(a.date));
  },
});
export const invoice = action({
  args: { paymentId: v.string() },
  returns: v.bytes(),
  handler: async (ctx, { paymentId }) => {
    const user = await callerCustomer(ctx);
    if (!user?.dodoCustomerId) throw new Error("Invoice not found");
    const id = encodeURIComponent(paymentId);
    const details = payment.parse(await (await dodoRequest(`/payments/${id}`)).json());
    if (details.customer.customer_id !== user.dodoCustomerId) throw new Error("Invoice not found");
    const response = await dodoRequest(`/invoices/payments/${id}`, {
      headers: { Accept: "application/pdf" },
    });
    return response.arrayBuffer();
  },
});
export async function cancelSubscriptions(
  customerId: string | undefined,
  local: { subscriptionId: string; status: string }[],
) {
  const ids = new Set(
    local
      .filter((s) => s.status === "active" || s.status === "on_hold")
      .map((s) => s.subscriptionId),
  );
  if (customerId) {
    const subscription = z.object({
      subscription_id: z.string(),
      status: z.string(),
      customer: z.object({ customer_id: z.string() }),
    });
    for (let page = 1; ; page++) {
      const response = await dodoRequest(
        `/subscriptions?customer_id=${encodeURIComponent(customerId)}&page_size=100&page_number=${page}`,
      );
      const { items } = z.object({ items: z.array(subscription) }).parse(await response.json());
      for (const item of items) {
        if (
          item.customer.customer_id === customerId &&
          item.status !== "cancelled" &&
          item.status !== "expired"
        )
          ids.add(item.subscription_id);
      }
      if (items.length < 100) break;
    }
  }
  for (const id of ids) {
    await dodoRequest(`/subscriptions/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body: JSON.stringify({ status: "cancelled", cancel_at_next_billing_date: false }),
    });
  }
}
