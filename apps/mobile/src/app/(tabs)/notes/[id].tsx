import { api } from "@typeaside/backend/convex/_generated/api";
import { useQuery } from "convex/react";
import * as Crypto from "expo-crypto";
import * as Haptics from "expo-haptics";
import { useLocalSearchParams, useRouter } from "expo-router";
import type { AndroidSymbol, SFSymbol } from "expo-symbols";
import { SymbolView } from "expo-symbols";
import { useEffect, useRef, useState } from "react";
import { Pressable, Share, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { EditorBar } from "@/notes/editor-bar";
import { type Draft, toggleChecklist } from "@/notes/model";
import { NoteBody } from "@/notes/note-body";
import { useKeyboardHeight } from "@/notes/use-keyboard-height";
import { useNoteDraft } from "@/notes/use-note-draft";
import { useColors, useNoteColors } from "@/ui/theme";

const ios = process.env.EXPO_OS === "ios";

export default function NoteScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const notes = useQuery(api.notes.list);
  const [newId] = useState(() => Crypto.randomUUID().replaceAll("-", ""));
  const colors = useColors();

  if (id === "new")
    return (
      <NoteEditor
        initial={{ id: newId, text: "", pinned: false, archived: false, color: "none" }}
        stored={false}
      />
    );
  const note = notes?.find((candidate) => candidate.id === id);
  if (!note)
    return (
      <View style={[styles.screen, { backgroundColor: colors.background }]}>
        {notes && (
          <Text style={[styles.missing, { color: colors.secondary }]}>This note was deleted.</Text>
        )}
      </View>
    );
  const { text, pinned, archived, color } = note;
  return <NoteEditor key={id} initial={{ id, text, pinned, archived, color }} stored />;
}

function NoteEditor({ initial, stored }: { initial: Draft; stored: boolean }) {
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const keyboard = useKeyboardHeight();
  const colors = useColors();
  const noteColors = useNoteColors();
  const { draft, error, update } = useNoteDraft(initial, stored);
  const [notice, setNotice] = useState<string | null>(null);
  const noticeTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(noticeTimer.current), []);
  const message = notice ?? error;

  function showNotice(text: string) {
    setNotice(text);
    clearTimeout(noticeTimer.current);
    noticeTimer.current = setTimeout(() => setNotice(null), 4000);
  }

  const togglePin = () => {
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    update({ pinned: !draft.pinned, archived: false }, { now: true });
  };
  const toggleArchive = () => {
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    update({ archived: !draft.archived, pinned: false }, { now: true });
    router.back();
  };

  const actions = (
    <>
      <IconButton
        label={draft.pinned ? "Unpin" : "Pin"}
        ios={draft.pinned ? "pin.fill" : "pin"}
        android="push_pin"
        active={draft.pinned}
        onPress={togglePin}
      />
      <IconButton
        label={draft.archived ? "Unarchive" : "Archive"}
        ios={draft.archived ? "tray.and.arrow.up" : "archivebox"}
        android={draft.archived ? "unarchive" : "archive"}
        onPress={toggleArchive}
      />
      <IconButton
        label="Share"
        ios="square.and.arrow.up"
        android="share"
        disabled={!draft.text.trim()}
        onPress={() =>
          void Share.share({ message: draft.text }).catch(() => showNotice("Could not share note."))
        }
      />
    </>
  );

  return (
    <View
      style={[
        styles.screen,
        {
          backgroundColor:
            draft.color === "none" ? colors.background : noteColors[draft.color].background,
          paddingTop: ios ? 14 : insets.top,
        },
      ]}
    >
      <View style={styles.top}>
        {ios ? (
          <>
            {actions}
            <Pressable hitSlop={10} style={styles.trailing} onPress={() => router.back()}>
              <Text style={[styles.done, { color: colors.accent }]}>Done</Text>
            </Pressable>
          </>
        ) : (
          <>
            <IconButton
              label="Back"
              ios="chevron.left"
              android="arrow_back"
              onPress={() => router.back()}
            />
            <View style={styles.trailing} />
            {actions}
          </>
        )}
      </View>
      <NoteBody
        text={draft.text}
        startEditing={!initial.text.trim()}
        onChange={(text) => update({ text })}
        onToggle={(line) => {
          void Haptics.selectionAsync();
          update((current) => ({ text: toggleChecklist(current.text, line) }));
        }}
      />
      {message && <Text style={[styles.notice, { color: colors.secondary }]}>{message}</Text>}
      <View style={{ paddingBottom: Math.max(keyboard, insets.bottom) }}>
        <EditorBar
          color={draft.color}
          onColor={(color) => {
            void Haptics.selectionAsync();
            update({ color }, { now: true });
          }}
          onDictated={(text) =>
            update(
              (current) => ({
                text: current.text.trim()
                  ? `${current.text.trimEnd()}\n${text}`.slice(0, 20_000)
                  : text.slice(0, 20_000),
              }),
              {
                now: true,
              },
            )
          }
          onNotice={showNotice}
        />
      </View>
    </View>
  );
}

function IconButton({
  label,
  ios: iosSymbol,
  android,
  active = false,
  disabled = false,
  onPress,
}: {
  label: string;
  ios: SFSymbol;
  android: AndroidSymbol;
  active?: boolean;
  disabled?: boolean;
  onPress: () => void;
}) {
  const colors = useColors();
  return (
    <Pressable
      accessibilityLabel={label}
      accessibilityState={{ selected: active, disabled }}
      hitSlop={8}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [styles.icon, (pressed || disabled) && styles.dim]}
    >
      <SymbolView
        name={{ ios: iosSymbol, android }}
        tintColor={active ? colors.accent : colors.text}
        size={22}
      />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1 },
  top: {
    flexDirection: "row",
    alignItems: "center",
    gap: 4,
    paddingHorizontal: ios ? 10 : 4,
    minHeight: 48,
  },
  icon: { width: 44, height: 44, alignItems: "center", justifyContent: "center" },
  dim: { opacity: 0.4 },
  trailing: { marginLeft: "auto" },
  done: { fontSize: 17, fontWeight: "600", paddingHorizontal: 8 },
  missing: { fontSize: 15, padding: 20 },
  notice: { fontSize: 13, paddingHorizontal: 20, paddingVertical: 6 },
});
