import type { Take } from "./takes";

export type Action = {
  label: string;
  icon: "copy" | "share" | "save" | "saved" | "delete" | "retry";
  onPress: () => void;
};

export function takeText(take: Take) {
  if (!take.pending) return take.text;
  return take.pending.error ? "Not transcribed yet" : "Cleaning up";
}

export function takeMeta(take: Take) {
  const words = take.text.split(/\s+/).filter(Boolean).length;
  return [
    formatTime(take.createdAt),
    take.pending ? take.pending.error : `${words} ${words === 1 ? "word" : "words"}`,
    take.noteId && "Saved",
  ]
    .filter(Boolean)
    .join(" · ");
}

export function formatElapsed(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

function formatTime(at: number) {
  const date = new Date(at);
  const days = Math.floor((startOfDay(Date.now()) - startOfDay(at)) / 86_400_000);
  if (days === 0) return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  if (days === 1) return "Yesterday";
  if (days < 7) return date.toLocaleDateString([], { weekday: "short" });
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}

function startOfDay(at: number) {
  return new Date(at).setHours(0, 0, 0, 0);
}
