import { internalMutation, type MutationCtx } from "./_generated/server";
import type { Doc } from "./_generated/dataModel";
import { dayStart } from "./lib";

type Counters = Omit<Doc<"rollups">, "_id" | "_creationTime" | "day">;

export async function addToRollup(ctx: MutationCtx, day: number, delta: Partial<Counters>) {
  const existing = await ctx.db
    .query("rollups")
    .withIndex("by_day", (q) => q.eq("day", day))
    .unique();
  const next: Counters = {
    activeInstalls: (existing?.activeInstalls ?? 0) + (delta.activeInstalls ?? 0),
    dictations: (existing?.dictations ?? 0) + (delta.dictations ?? 0),
    words: (existing?.words ?? 0) + (delta.words ?? 0),
    newUsers: (existing?.newUsers ?? 0) + (delta.newUsers ?? 0),
    activeUsers: (existing?.activeUsers ?? 0) + (delta.activeUsers ?? 0),
    hostedWords: (existing?.hostedWords ?? 0) + (delta.hostedWords ?? 0),
  };
  if (existing) await ctx.db.patch(existing._id, next);
  else await ctx.db.insert("rollups", { day, ...next });
}

// Recomputes every day from source rows, so it is safe to rerun after drift or for history before rollups existed.
export const rebuild = internalMutation({
  args: {},
  handler: async (ctx) => {
    const [existing, pings, daily, users] = await Promise.all([
      ctx.db.query("rollups").collect(),
      ctx.db.query("pings").collect(),
      ctx.db.query("daily").collect(),
      ctx.db.query("users").collect(),
    ]);
    const days = new Map<number, Counters>();
    const add = (day: number, delta: Partial<Counters>) => {
      const row = days.get(day) ?? {
        activeInstalls: 0,
        dictations: 0,
        words: 0,
        newUsers: 0,
        activeUsers: 0,
        hostedWords: 0,
      };
      for (const [key, value] of Object.entries(delta) as [keyof Counters, number][])
        row[key] += value;
      days.set(day, row);
    };
    for (const ping of pings)
      add(ping.day, { activeInstalls: 1, dictations: ping.dictations, words: ping.words });
    for (const usage of daily) add(usage.day, { activeUsers: 1, hostedWords: usage.words });
    for (const user of users) add(dayStart(user._creationTime), { newUsers: 1 });
    await Promise.all(existing.map((row) => ctx.db.delete(row._id)));
    await Promise.all(
      [...days].map(([day, counters]) => ctx.db.insert("rollups", { day, ...counters })),
    );
    return days.size;
  },
});
