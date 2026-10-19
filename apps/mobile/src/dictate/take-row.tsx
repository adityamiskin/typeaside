import type { AndroidSymbol, SFSymbol } from "expo-symbols";
import { SymbolView } from "expo-symbols";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { colors } from "@/ui/theme";
import type { Take } from "./takes";

export type Action = { label: string; ios: SFSymbol; android: AndroidSymbol; onPress: () => void };

export function TakeRow({
  take,
  expanded,
  onToggle,
  actions,
}: {
  take: Take;
  expanded: boolean;
  onToggle: () => void;
  actions: Action[];
}) {
  const words = take.text.split(/\s+/).filter(Boolean).length;
  const meta = [
    formatTime(take.createdAt),
    take.pending ? take.pending.error : `${words} ${words === 1 ? "word" : "words"}`,
    take.noteId && "Saved",
  ].filter(Boolean);

  return (
    <View style={[styles.row, expanded && styles.expanded]}>
      <Pressable onPress={onToggle} style={styles.body}>
        <Text
          style={[styles.text, take.pending && styles.pendingText]}
          numberOfLines={expanded ? undefined : 3}
          selectable={expanded}
        >
          {take.pending ? (take.pending.error ? "Not transcribed yet" : "Cleaning up") : take.text}
        </Text>
        <Text style={[styles.meta, take.pending?.error && { color: colors.danger }]}>
          {meta.join(" · ")}
        </Text>
      </Pressable>
      {expanded ? (
        <View style={styles.actions}>
          {actions.map((action) => (
            <Pressable
              key={action.label}
              onPress={action.onPress}
              style={({ pressed }) => [styles.action, pressed && { opacity: 0.5 }]}
            >
              <SymbolView
                name={{ ios: action.ios, android: action.android }}
                tintColor={colors.text}
                size={20}
              />
              <Text style={styles.actionLabel}>{action.label}</Text>
            </Pressable>
          ))}
        </View>
      ) : null}
    </View>
  );
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

const styles = StyleSheet.create({
  row: { borderBottomWidth: StyleSheet.hairlineWidth, borderBottomColor: colors.separator },
  expanded: { backgroundColor: colors.elevated },
  body: { paddingHorizontal: 16, paddingVertical: 12, gap: 4 },
  text: { color: colors.text, fontSize: 16, lineHeight: 22 },
  pendingText: { color: colors.secondary },
  meta: { color: colors.tertiary, fontSize: 12 },
  actions: {
    flexDirection: "row",
    borderTopWidth: StyleSheet.hairlineWidth,
    borderTopColor: colors.separator,
  },
  action: { flex: 1, alignItems: "center", gap: 4, paddingVertical: 10 },
  actionLabel: { color: colors.text, fontSize: 11 },
});
