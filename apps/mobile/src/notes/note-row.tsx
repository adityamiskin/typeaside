import type { AndroidSymbol, SFSymbol } from "expo-symbols";
import { SymbolView } from "expo-symbols";
import { Pressable, StyleSheet, Text, View } from "react-native";
import ReanimatedSwipeable from "react-native-gesture-handler/ReanimatedSwipeable";
import { colors, noteColors } from "@/ui/theme";
import type { NoteActions } from "./actions";
import { type Note, relativeTime, summarize } from "./model";
import { NoteLink } from "./note-link";

const PREVIEW_ITEMS = 3;

export function NoteRow({ note, actions }: { note: Note; actions: NoteActions }) {
  const { title, items, preview } = summarize(note.text);
  const done = items.filter((item) => item.checked).length;
  const meta = [items.length > 0 && `${done} of ${items.length} done`, relativeTime(note.updatedAt)]
    .filter(Boolean)
    .join(" · ");
  const tint = noteColors[note.color];

  return (
    <ReanimatedSwipeable
      friction={2}
      overshootLeft={false}
      overshootRight={false}
      renderLeftActions={
        note.archived
          ? undefined
          : (_progress, _translation, swipe) => (
              <SwipeAction
                label={note.pinned ? "Unpin" : "Pin"}
                ios={note.pinned ? "pin.slash.fill" : "pin.fill"}
                android="push_pin"
                color={noteColors.sand.accent}
                onPress={() => {
                  swipe.close();
                  actions.togglePin(note);
                }}
              />
            )
      }
      renderRightActions={(_progress, _translation, swipe) => (
        <View style={styles.actions}>
          <SwipeAction
            label={note.archived ? "Unarchive" : "Archive"}
            ios={note.archived ? "tray.and.arrow.up.fill" : "archivebox.fill"}
            android={note.archived ? "unarchive" : "archive"}
            color={noteColors.sky.accent}
            onPress={() => {
              swipe.close();
              actions.toggleArchive(note);
            }}
          />
          <SwipeAction
            label="Delete"
            ios="trash.fill"
            android="delete"
            color={colors.danger}
            onPress={() => {
              swipe.close();
              actions.confirmDelete(note);
            }}
          />
        </View>
      )}
    >
      <NoteLink
        note={note}
        actions={actions}
        style={[
          styles.row,
          { backgroundColor: note.color === "none" ? colors.background : tint.background },
        ]}
      >
        {title ? (
          <View style={styles.titleRow}>
            {note.pinned && (
              <SymbolView
                name={{ ios: "pin.fill", android: "push_pin" }}
                tintColor={tint.accent}
                size={12}
              />
            )}
            <Text style={styles.title} numberOfLines={1}>
              {title}
            </Text>
          </View>
        ) : null}
        {items.length > 0 ? (
          items.slice(0, PREVIEW_ITEMS).map((item, index) => (
            <View key={index} style={styles.item}>
              <View style={[styles.box, item.checked && styles.boxChecked]} />
              <Text style={[styles.preview, item.checked && styles.checked]} numberOfLines={1}>
                {item.text}
              </Text>
            </View>
          ))
        ) : preview ? (
          <Text style={styles.preview} numberOfLines={title ? 2 : 3}>
            {preview}
          </Text>
        ) : null}
        <Text style={styles.meta}>{meta}</Text>
      </NoteLink>
    </ReanimatedSwipeable>
  );
}

function SwipeAction({
  label,
  ios,
  android,
  color,
  onPress,
}: {
  label: string;
  ios: SFSymbol;
  android: AndroidSymbol;
  color: string;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityLabel={label}
      onPress={onPress}
      style={({ pressed }) => [
        styles.action,
        { backgroundColor: color },
        pressed && styles.pressed,
      ]}
    >
      <SymbolView name={{ ios, android }} tintColor={colors.text} size={20} />
      <Text style={styles.actionLabel}>{label}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  row: { paddingHorizontal: 16, paddingTop: 10, paddingBottom: 11, gap: 2 },
  titleRow: { flexDirection: "row", alignItems: "center", gap: 6 },
  title: { flex: 1, color: colors.text, fontSize: 15, fontWeight: "600" },
  preview: { flexShrink: 1, color: colors.secondary, fontSize: 14, lineHeight: 19 },
  item: { flexDirection: "row", alignItems: "center", gap: 7 },
  box: {
    width: 12,
    height: 12,
    borderRadius: 3,
    borderWidth: 1.5,
    borderColor: colors.tertiary,
  },
  boxChecked: { backgroundColor: colors.secondary, borderColor: colors.secondary },
  checked: { color: colors.tertiary, textDecorationLine: "line-through" },
  meta: { color: colors.tertiary, fontSize: 12, marginTop: 2 },
  actions: { flexDirection: "row" },
  action: { width: 76, alignItems: "center", justifyContent: "center", gap: 4 },
  actionLabel: { color: colors.text, fontSize: 12, fontWeight: "500" },
  pressed: { opacity: 0.7 },
});
