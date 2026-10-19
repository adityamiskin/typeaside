import type { MutationCtx } from "./_generated/server";
import type { Doc } from "./_generated/dataModel";

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
