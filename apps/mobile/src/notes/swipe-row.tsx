import type { AndroidSymbol, SFSymbol } from "expo-symbols";
import { SymbolView } from "expo-symbols";
import { Link } from "expo-router";
import type { ReactNode } from "react";
import {
  PlatformColor,
  Pressable,
  type StyleProp,
  StyleSheet,
  Text,
  View,
  type ViewStyle,
  type ColorValue,
} from "react-native";
import ReanimatedSwipeable from "react-native-gesture-handler/ReanimatedSwipeable";
import { useNoteColors } from "@/ui/theme";
import type { NoteActions } from "./actions";
import { type Note, relativeTime, summarize } from "./model";

const PREVIEW_ITEMS = 3;

// iOS list row: swipe for pin, archive and delete; long press for the context menu.
export function SwipeRow({ note, actions }: { note: Note; actions: NoteActions }) {
  const noteColors = useNoteColors();
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
                color={PlatformColor("systemOrange")}
                onPress={() => {
                  swipe.close();
                  actions.pin([note], !note.pinned);
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
            color={PlatformColor("systemIndigo")}
            onPress={() => {
              swipe.close();
              actions.archive([note], !note.archived);
            }}
          />
          <SwipeAction
            label="Delete"
            ios="trash.fill"
            android="delete"
            color={PlatformColor("systemRed")}
            onPress={() => {
              swipe.close();
              actions.confirmDelete([note.id]);
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
          {
            backgroundColor:
              note.color === "none" ? PlatformColor("systemBackground") : tint.background,
          },
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
  color: ColorValue;
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
      <SymbolView name={{ ios, android }} tintColor={PlatformColor("white")} size={20} />
      <Text style={styles.actionLabel}>{label}</Text>
    </Pressable>
  );
}

function NoteLink({
  note,
  actions,
  style,
  children,
}: {
  note: Note;
  actions: NoteActions;
  style: StyleProp<ViewStyle>;
  children: ReactNode;
}) {
  return (
    <Link href={{ pathname: "/notes/[id]", params: { id: note.id } }} asChild>
      <Link.Trigger>
        <Pressable style={({ pressed }) => [style, pressed && { opacity: 0.7 }]}>
          {children}
        </Pressable>
      </Link.Trigger>
      <Link.Menu>
        <Link.MenuAction
          hidden={note.archived}
          title={note.pinned ? "Unpin" : "Pin"}
          icon={note.pinned ? "pin.slash" : "pin"}
          onPress={() => actions.pin([note], !note.pinned)}
        />
        <Link.MenuAction
          title={note.archived ? "Unarchive" : "Archive"}
          icon={note.archived ? "tray.and.arrow.up" : "archivebox"}
          onPress={() => actions.archive([note], !note.archived)}
        />
        <Link.MenuAction
          title="Delete"
          icon="trash"
          destructive
          onPress={() => actions.confirmDelete([note.id])}
        />
      </Link.Menu>
    </Link>
  );
}

const styles = StyleSheet.create({
  row: {
    paddingHorizontal: 16,
    paddingTop: 10,
    paddingBottom: 11,
    gap: 2,
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderBottomColor: PlatformColor("separator"),
  },
  titleRow: { flexDirection: "row", alignItems: "center", gap: 6 },
  title: { flex: 1, color: PlatformColor("label"), fontSize: 15, fontWeight: "600" },
  preview: { flexShrink: 1, color: PlatformColor("secondaryLabel"), fontSize: 14, lineHeight: 19 },
  item: { flexDirection: "row", alignItems: "center", gap: 7 },
  box: {
    width: 12,
    height: 12,
    borderRadius: 3,
    borderWidth: 1.5,
    borderColor: PlatformColor("tertiaryLabel"),
  },
  boxChecked: {
    backgroundColor: PlatformColor("secondaryLabel"),
    borderColor: PlatformColor("secondaryLabel"),
  },
  checked: { color: PlatformColor("tertiaryLabel"), textDecorationLine: "line-through" },
  meta: { color: PlatformColor("tertiaryLabel"), fontSize: 12, marginTop: 2 },
  actions: { flexDirection: "row" },
  action: { width: 76, alignItems: "center", justifyContent: "center", gap: 4 },
  actionLabel: { color: PlatformColor("white"), fontSize: 12, fontWeight: "500" },
  pressed: { opacity: 0.7 },
});
