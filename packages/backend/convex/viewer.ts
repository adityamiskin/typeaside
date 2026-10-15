import { mutation, query } from "./_generated/server";
import {
  accountState,
  ensureUser,
  requireIdentity,
  userForIdentity,
  weekStart,
  WEEK_MS,
  FREE_WEEKLY_WORDS,
} from "./lib";

export const sync = mutation({ args: {}, handler: async (ctx) => (await ensureUser(ctx))._id });
export const current = query({
  args: {},
  handler: async (ctx) => {
    const identity = await requireIdentity(ctx);
    const user = await userForIdentity(ctx);
    return {
      email: identity.email ?? null,
      ...(user
        ? await accountState(ctx, user._id)
        : {
            plan: "free" as const,
            currentPeriodEnd: null,
            usage: { words: 0, cap: FREE_WEEKLY_WORDS, resetsAt: weekStart() + WEEK_MS },
          }),
    };
  },
});
