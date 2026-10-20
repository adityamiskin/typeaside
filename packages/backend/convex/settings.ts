import { ConvexError, v } from "convex/values";
import { z } from "zod";
import {
  internalMutation,
  internalQuery,
  mutation,
  query,
  type MutationCtx,
} from "./_generated/server";
import { ensureUser, userForIdentity } from "./lib";
import { settingsFields } from "./schema";
import { rateLimiter } from "./rateLimits";

export const settingsInput = z.strictObject({
  dictionary: z.array(z.string().max(100)).max(2000),
  removedWords: z.array(z.string().max(100)).max(2000),
  snippets: z
    .array(
      z.strictObject({
        trigger: z.string().max(100),
        text: z.string().max(4000),
      }),
    )
    .max(500),
  styles: z.record(z.string(), z.enum(["formal", "casual", "very_casual"])),
  appInstructions: z
    .array(z.strictObject({ app: z.string().max(512), text: z.string().max(300) }))
    .max(20),
  updatedAt: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
});
export const get = internalQuery({
  args: { userId: v.id("users") },
  handler: (ctx, { userId }) =>
    ctx.db
      .query("settings")
      .withIndex("by_user", (q) => q.eq("userId", userId))
      .unique(),
});
export const put = internalMutation({
  args: { userId: v.id("users"), ...settingsFields },
  handler: async (ctx, input) => {
    const status = await rateLimiter.limit(ctx, "settings", {
      key: input.userId,
    });
    if (!status.ok) return { ok: false as const };
    const stored = await ctx.db
      .query("settings")
      .withIndex("by_user", (q) => q.eq("userId", input.userId))
      .unique();
    if (stored && stored.updatedAt >= input.updatedAt)
      return { ok: true as const, settings: stored };
    if (stored) {
      await ctx.db.replace(stored._id, input);
      return { ok: true as const, settings: await ctx.db.get(stored._id) };
    }
    const id = await ctx.db.insert("settings", input);
    return { ok: true as const, settings: await ctx.db.get(id) };
  },
});

const wordInput = z.string().trim().min(1).max(100);
const snippetInput = z.strictObject({
  trigger: wordInput,
  text: z.string().trim().min(1).max(4000),
});
const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

async function editable(ctx: MutationCtx) {
  const user = await ensureUser(ctx);
  const status = await rateLimiter.limit(ctx, "settings", { key: user._id });
  if (!status.ok) throw new ConvexError("Too many settings requests");
  const stored = await ctx.db
    .query("settings")
    .withIndex("by_user", (q) => q.eq("userId", user._id))
    .unique();
  const fields = stored ?? {
    userId: user._id,
    dictionary: [],
    removedWords: [],
    snippets: [],
    styles: {},
    appInstructions: [],
    updatedAt: 0,
  };
  const save = async (
    change: Partial<Pick<typeof fields, "dictionary" | "removedWords" | "snippets">>,
  ) => {
    const updatedAt = Math.max(Date.now(), fields.updatedAt + 1);
    if (stored) await ctx.db.patch(stored._id, { ...change, updatedAt });
    else await ctx.db.insert("settings", { ...fields, ...change, updatedAt });
  };
  return { fields, save };
}

function parse<T>(schema: z.ZodType<T>, value: unknown, message: string) {
  const parsed = schema.safeParse(value);
  if (!parsed.success) throw new ConvexError(message);
  return parsed.data;
}

export const mine = query({
  args: {},
  handler: async (ctx) => {
    const user = await userForIdentity(ctx);
    return user
      ? ctx.db
          .query("settings")
          .withIndex("by_user", (q) => q.eq("userId", user._id))
          .unique()
      : null;
  },
});

export const addWord = mutation({
  args: { word: v.string() },
  handler: async (ctx, input) => {
    const word = parse(wordInput, input.word, "Invalid word");
    const { fields, save } = await editable(ctx);
    const dictionary = fields.dictionary.filter((value) => !same(value, word)).concat(word);
    if (dictionary.length > 2000) throw new ConvexError("Dictionary is full");
    await save({
      dictionary,
      removedWords: fields.removedWords.filter((value) => !same(value, word)),
    });
  },
});

export const removeWord = mutation({
  args: { word: v.string() },
  handler: async (ctx, input) => {
    const word = parse(wordInput, input.word, "Invalid word");
    const { fields, save } = await editable(ctx);
    await save({
      dictionary: fields.dictionary.filter((value) => !same(value, word)),
      // Oldest tombstones age out so removal never hits the settings cap.
      removedWords: fields.removedWords
        .filter((value) => !same(value, word))
        .concat(word)
        .slice(-2000),
    });
  },
});

export const saveSnippet = mutation({
  args: {
    trigger: v.string(),
    text: v.string(),
    originalTrigger: v.optional(v.string()),
  },
  handler: async (ctx, { originalTrigger, ...input }) => {
    const snippet = parse(snippetInput, input, "Invalid snippet");
    const previous = parse(wordInput.optional(), originalTrigger, "Invalid snippet");
    const { fields, save } = await editable(ctx);
    const other = fields.snippets.filter(
      (value) => !same(value.trigger, previous ?? snippet.trigger),
    );
    if (other.some((value) => same(value.trigger, snippet.trigger)))
      throw new ConvexError("Trigger already exists");
    const snippets = other.concat(snippet);
    if (snippets.length > 500) throw new ConvexError("Snippets are full");
    await save({ snippets });
  },
});

export const deleteSnippet = mutation({
  args: { trigger: v.string() },
  handler: async (ctx, input) => {
    const trigger = parse(wordInput, input.trigger, "Invalid trigger");
    const { fields, save } = await editable(ctx);
    await save({
      snippets: fields.snippets.filter((value) => !same(value.trigger, trigger)),
    });
  },
});
