import { ConvexError, v } from "convex/values";
import { z } from "zod";
import type { Doc, Id } from "./_generated/dataModel";
import {
  internalMutation,
  internalQuery,
  mutation,
  query,
  type MutationCtx,
  type QueryCtx,
} from "./_generated/server";
import { ensureUser, userForIdentity } from "./lib";
import { rateLimiter } from "./rateLimits";
import { noteFields } from "./schema";
import { noteColor, noteColorInput } from "./noteColors";

export const noteIdInput = z.string().regex(/^[A-Za-z0-9_-]{1,64}$/);
const editableInput = z.object({
  id: noteIdInput,
  text: z.string().trim().min(1).max(20_000),
  pinned: z.boolean(),
  archived: z.boolean(),
  color: noteColorInput,
});
const timestamp = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
export const noteInput = editableInput.extend({ createdAt: timestamp, updatedAt: timestamp });

function noteJson(note: Doc<"notes">) {
  return {
    id: note.noteId,
    text: note.text,
    pinned: note.pinned,
    archived: note.archived,
    color: note.color,
    createdAt: note.createdAt,
    updatedAt: note.updatedAt,
  };
}
async function listForUser(ctx: QueryCtx, userId: Id<"users">) {
  const notes = await ctx.db
    .query("notes")
    .withIndex("by_user", (q) => q.eq("userId", userId))
    .order("desc")
    .collect();
  return notes.map(noteJson);
}
async function findNote(ctx: QueryCtx, userId: Id<"users">, id: string) {
  return ctx.db
    .query("notes")
    .withIndex("by_user_note", (q) => q.eq("userId", userId).eq("noteId", id))
    .unique();
}
async function upsert(ctx: MutationCtx, userId: Id<"users">, note: z.infer<typeof noteInput>) {
  if (note.archived) note = { ...note, pinned: false };
  const stored = await findNote(ctx, userId, note.id);
  if (stored && stored.updatedAt >= note.updatedAt) return noteJson(stored);
  const { id, ...fields } = note;
  if (stored) await ctx.db.replace(stored._id, { userId, noteId: id, ...fields });
  else await ctx.db.insert("notes", { userId, noteId: id, ...fields });
  return note;
}
async function deleteNote(ctx: MutationCtx, userId: Id<"users">, id: string) {
  const stored = await findNote(ctx, userId, id);
  if (stored) await ctx.db.delete(stored._id);
}
async function checkRate(ctx: MutationCtx, userId: Id<"users">) {
  const status = await rateLimiter.limit(ctx, "notes", { key: userId });
  if (!status.ok) throw new ConvexError("Too many notes requests");
}
export const list = query({
  args: {},
  handler: async (ctx) => {
    const user = await userForIdentity(ctx);
    return user ? listForUser(ctx, user._id) : [];
  },
});
export const save = mutation({
  args: {
    id: v.string(),
    text: v.string(),
    pinned: v.boolean(),
    archived: v.boolean(),
    color: noteColor,
  },
  handler: async (ctx, input) => {
    const parsed = editableInput.safeParse(input);
    if (!parsed.success) throw new ConvexError("Invalid note");
    const user = await ensureUser(ctx);
    await checkRate(ctx, user._id);
    const stored = await findNote(ctx, user._id, parsed.data.id);
    const now = Date.now();
    return upsert(ctx, user._id, {
      ...parsed.data,
      createdAt: stored?.createdAt ?? now,
      updatedAt: now,
    });
  },
});
export const remove = mutation({
  args: { id: v.string() },
  handler: async (ctx, { id }) => {
    if (!noteIdInput.safeParse(id).success) throw new ConvexError("Invalid note id");
    const user = await userForIdentity(ctx);
    if (!user) return;
    await checkRate(ctx, user._id);
    await deleteNote(ctx, user._id, id);
  },
});
export const limit = internalMutation({
  args: { userId: v.id("users") },
  handler: (ctx, { userId }) => rateLimiter.limit(ctx, "notes", { key: userId }),
});
export const get = internalQuery({
  args: { userId: v.id("users") },
  handler: (ctx, { userId }) => listForUser(ctx, userId),
});
export const put = internalMutation({
  args: { userId: v.id("users"), id: v.string(), ...noteFields },
  handler: (ctx, { userId, ...input }) => upsert(ctx, userId, input),
});
export const erase = internalMutation({
  args: { userId: v.id("users"), id: v.string() },
  handler: (ctx, { userId, id }) => deleteNote(ctx, userId, id),
});
