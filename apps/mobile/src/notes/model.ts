import type { api } from "@typeaside/backend/convex/_generated/api";
import type { FunctionReturnType } from "convex/server";

export type Note = FunctionReturnType<typeof api.notes.list>[number];
export type Draft = Pick<Note, "id" | "text" | "pinned" | "archived" | "color">;

// Same checklist syntax as the desktop and web notes: `- [ ] item` / `- [x] item`.
export function checklistItem(line: string) {
  const match = /^- \[([ xX])\] (.*)$/.exec(line);
  return match ? { checked: match[1] !== " ", text: match[2] ?? "" } : null;
}

export function toggleChecklist(text: string, index: number) {
  return text
    .split("\n")
    .map((line, i) => {
      const item = checklistItem(line);
      return i === index && item ? `- [${item.checked ? " " : "x"}] ${item.text}` : line;
    })
    .join("\n");
}

// Notes store one text; a first line that is not a checklist item is the title.
export function splitTitle(text: string) {
  const [first = "", ...rest] = text.split("\n");
  if (checklistItem(first)) return { title: "", body: text };
  return { title: first.trim() ? first : "", body: rest.join("\n") };
}

export function joinTitle(title: string, body: string) {
  return title.trim() && body ? `${title}\n${body}` : title.trim() ? title : body;
}

export function summarize(text: string) {
  const { title, body } = splitTitle(text);
  const lines = body.split("\n");
  return {
    title: title.trim(),
    items: lines.flatMap((line) => checklistItem(line) ?? []),
    preview: lines
      .filter((line) => !checklistItem(line))
      .join("\n")
      .trim(),
  };
}

export const emptyMessages = {
  notes: "Notes you add appear here",
  archive: "Archived notes appear here",
  search: "No matching notes",
};

export type EmptyState = keyof typeof emptyMessages;

export function arrange(notes: Note[], archived: boolean, search: string) {
  const query = search.trim().toLowerCase();
  return notes
    .filter((note) => note.archived === archived && note.text.toLowerCase().includes(query))
    .sort((a, b) => Number(b.pinned) - Number(a.pinned) || b.updatedAt - a.updatedAt);
}

export function relativeTime(at: number) {
  const date = new Date(at);
  const days = Math.round((startOfDay(Date.now()) - startOfDay(at)) / 86_400_000);
  if (days <= 0) return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  if (days === 1) return "Yesterday";
  if (days < 7) return date.toLocaleDateString([], { weekday: "short" });
  if (date.getFullYear() === new Date().getFullYear())
    return date.toLocaleDateString([], { month: "short", day: "numeric" });
  return date.toLocaleDateString([], { year: "2-digit", month: "numeric", day: "numeric" });
}

function startOfDay(at: number) {
  return new Date(at).setHours(0, 0, 0, 0);
}
