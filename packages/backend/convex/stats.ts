import { v } from "convex/values";
import { internalQuery } from "./_generated/server";
import { DAY_MS, dayStart, isPro } from "./lib";

export const summary = internalQuery({
  args: { days: v.optional(v.number()) },
  handler: async (ctx, { days = 30 }) => {
    if (!Number.isInteger(days) || days < 1 || days > 366)
      throw new Error("days must be an integer from 1 to 366");
    const today = dayStart();
    const start = today - (days - 1) * DAY_MS;
    const [rollups, active] = await Promise.all([
      ctx.db
        .query("rollups")
        .withIndex("by_day", (q) => q.gte("day", start).lte("day", today))
        .collect(),
      ctx.db
        .query("subscriptions")
        .withIndex("by_status", (q) => q.eq("status", "active"))
        .collect(),
    ]);
    const byDay = new Map(rollups.map((rollup) => [rollup.day, rollup]));
    return {
      days: Array.from({ length: days }, (_, i) => {
        const day = start + i * DAY_MS;
        const rollup = byDay.get(day);
        return {
          day,
          activeInstalls: rollup?.activeInstalls ?? 0,
          dictations: rollup?.dictations ?? 0,
          words: rollup?.words ?? 0,
          newUsers: rollup?.newUsers ?? 0,
          activeUsers: rollup?.activeUsers ?? 0,
          hostedWords: rollup?.hostedWords ?? 0,
        };
      }),
      currentPro: new Set(active.filter((s) => isPro(s)).map((s) => s.userId)).size,
    };
  },
});
