import { v } from "convex/values";
import { z } from "zod";
import { internalMutation } from "./_generated/server";
import { DAY_MS, dayStart } from "./lib";
import { pingFields } from "./schema";
import { rateLimiter } from "./rateLimits";
import { addToRollup } from "./rollups";

// Per-day ceilings; a forged ping can't skew stats beyond what one install could plausibly do.
const MAX_DICTATIONS = 2_000;
const MAX_WORDS = 50_000;
const MAX_COMMANDS = 2_000;
const MAX_LATENCY_MS = 60_000;

export const pingInput = z.strictObject({
  installId: z
    .string()
    .regex(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i)
    .transform((value) => value.toLowerCase()),
  version: z.string().trim().min(1).max(64),
  os: z.enum(["linux", "macos", "windows"]),
  source: z.enum(["hosted", "own", "local"]),
  days: z
    .array(
      z.strictObject({
        day: z
          .number()
          .int()
          .nonnegative()
          .refine(
            (day) => day % DAY_MS === 0 && day >= dayStart() - 7 * DAY_MS && day <= dayStart(),
          ),
        dictations: z.number().int().min(0).max(MAX_DICTATIONS),
        words: z.number().int().min(0).max(MAX_WORDS),
        commands: z.number().int().min(0).max(MAX_COMMANDS),
        latencyMs: z.number().int().min(0).max(MAX_LATENCY_MS),
      }),
    )
    .min(1)
    .max(7),
});

export const record = internalMutation({
  args: {
    installId: pingFields.installId,
    version: pingFields.version,
    os: pingFields.os,
    source: pingFields.source,
    days: v.array(
      v.object({
        day: pingFields.day,
        dictations: pingFields.dictations,
        words: pingFields.words,
        commands: pingFields.commands,
        latencyMs: pingFields.latencyMs,
      }),
    ),
  },
  handler: async (ctx, { days, ...install }) => {
    const status = await rateLimiter.limit(ctx, "ping", { key: install.installId });
    if (!status.ok) return status;
    for (const day of days) {
      const existing = await ctx.db
        .query("pings")
        .withIndex("by_install_day", (q) => q.eq("installId", install.installId).eq("day", day.day))
        .unique();
      const fields = { ...install, ...day };
      if (existing) {
        await ctx.db.replace(existing._id, fields);
        await addToRollup(ctx, day.day, {
          dictations: day.dictations - existing.dictations,
          words: day.words - existing.words,
        });
      } else {
        await ctx.db.insert("pings", fields);
        await addToRollup(ctx, day.day, {
          activeInstalls: 1,
          dictations: day.dictations,
          words: day.words,
        });
      }
    }
    return status;
  },
});
