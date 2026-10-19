import { v } from "convex/values";
import { z } from "zod";

export const noteColors = ["none", "sky", "lilac", "sage", "sand", "rose"] as const;
export const noteColorInput = z.enum(noteColors);
export const noteColor = v.union(...noteColors.map((color) => v.literal(color)));
export type NoteColor = z.infer<typeof noteColorInput>;
