import type { Account } from "@/account/account";
import type { Take } from "./takes";

export type Action = {
  label: string;
  icon: "copy" | "save" | "saved" | "delete" | "retry";
  onPress: () => void;
};

export function takeText(take: Take) {
  if (!take.pending) return take.text;
  return take.pending.error ? "Not transcribed yet" : "Cleaning up";
}

export function takeMeta(take: Take) {
  const words = take.text.split(/\s+/).filter(Boolean).length;
  return [
    new Date(take.createdAt).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" }),
    take.pending ? take.pending.error : `${words} ${words === 1 ? "word" : "words"}`,
    take.noteId && "Saved",
  ]
    .filter(Boolean)
    .join(" · ");
}

export function usageText({ words, cap }: Account["usage"]) {
  if (cap === null) return `${words.toLocaleString()} words this week`;
  if (words >= cap) return "Weekly limit reached";
  return `${(cap - words).toLocaleString()} words left`;
}

// Takes arrive newest first, so each day's group is contiguous.
export function groupByDay(takes: Take[]) {
  const groups: { label: string; takes: Take[] }[] = [];
  for (const take of takes) {
    const label = dayLabel(take.createdAt);
    const last = groups.at(-1);
    if (last?.label === label) last.takes.push(take);
    else groups.push({ label, takes: [take] });
  }
  return groups;
}

function dayLabel(at: number) {
  const date = new Date(at);
  const days = Math.round((startOfDay(Date.now()) - startOfDay(at)) / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 7) return date.toLocaleDateString([], { weekday: "long" });
  return date.toLocaleDateString([], {
    month: "long",
    day: "numeric",
    year: date.getFullYear() === new Date().getFullYear() ? undefined : "numeric",
  });
}

function startOfDay(at: number) {
  return new Date(at).setHours(0, 0, 0, 0);
}
