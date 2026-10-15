import { v } from "convex/values";
import { internalQuery } from "./_generated/server";
import { DAY_MS, dayStart } from "./lib";

export const summary = internalQuery({
  args: { days: v.optional(v.number()) },
  handler: async (ctx, { days = 30 }) => {
    if (!Number.isInteger(days) || days < 1 || days > 366)
      throw new Error("days must be an integer from 1 to 366");
    const today = dayStart();
    const start = today - (days - 1) * DAY_MS;
    const end = today + DAY_MS;
    const [pings, users, daily, subscriptions] = await Promise.all([
      ctx.db
        .query("pings")
        .withIndex("by_day", (q) => q.gte("day", start).lt("day", end))
        .collect(),
      ctx.db
        .query("users")
        .withIndex("by_creation_time", (q) =>
          q.gte("_creationTime", start).lt("_creationTime", end),
        )
        .collect(),
      ctx.db
        .query("daily")
        .withIndex("by_day", (q) => q.gte("day", start).lt("day", end))
        .collect(),
      ctx.db.query("subscriptions").collect(),
    ]);
    const perDay = Array.from({ length: days }, (_, i) => ({
      day: start + i * DAY_MS,
      activeInstalls: 0,
      dictations: 0,
      words: 0,
      newUsers: 0,
      activeUsers: 0,
      hostedWords: 0,
    }));
    for (const ping of pings) {
      const row = perDay[(ping.day - start) / DAY_MS]!;
      row.activeInstalls += 1;
      row.dictations += ping.dictations;
      row.words += ping.words;
    }
    for (const user of users)
      perDay[(dayStart(user._creationTime) - start) / DAY_MS]!.newUsers += 1;
    const activeUsers = new Map<number, Set<string>>();
    for (const usage of daily) {
      const row = perDay[(usage.day - start) / DAY_MS]!;
      row.hostedWords += usage.words;
      const active = activeUsers.get(usage.day) ?? new Set<string>();
      active.add(usage.userId);
      activeUsers.set(usage.day, active);
      row.activeUsers = active.size;
    }
    return {
      days: perDay,
      currentPro: new Set(subscriptions.filter((s) => s.status === "active").map((s) => s.userId))
        .size,
    };
  },
});
