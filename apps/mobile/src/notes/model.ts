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

// The first non-blank line is the title unless it is a checklist item.
export function titleLine(lines: string[]) {
  const index = lines.findIndex((line) => line.trim());
  return index >= 0 && !checklistItem(lines[index] ?? "") ? index : -1;
}

export function summarize(text: string) {
  const lines = text.split("\n");
  const title = titleLine(lines);
  const body = lines.filter((_, index) => index !== title);
  const items = body.flatMap((line) => checklistItem(line) ?? []);
  return {
    title: title >= 0 ? (lines[title] ?? "").trim() : "",
    items,
    preview: body
      .filter((line) => line.trim() && !checklistItem(line))
      .map((line) => line.trim())
      .join(" "),
  };
}

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
