import { api } from "@typeaside/backend/convex/_generated/api";
import { useQuery } from "convex/react";
import * as Crypto from "expo-crypto";
import * as Haptics from "expo-haptics";
import { useLocalSearchParams, useRouter } from "expo-router";
import { useEffect, useRef, useState } from "react";
import { Pressable, Share, StyleSheet, Text, TextInput, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { useNoteActions } from "@/notes/actions";
import { EditorBar, IconButton } from "@/notes/editor-bar";
import { type Draft, joinTitle, splitTitle, toggleChecklist } from "@/notes/model";
import { NoteBody } from "@/notes/note-body";
import { useKeyboardHeight } from "@/notes/use-keyboard-height";
import { useNoteDraft } from "@/notes/use-note-draft";
import { useColors, useNoteColors } from "@/ui/theme";

const ios = process.env.EXPO_OS === "ios";
const MAX_LENGTH = 20_000;

export default function NoteScreen() {
  const { id, dictate } = useLocalSearchParams<{ id: string; dictate?: string }>();
  const notes = useQuery(api.notes.list);
  const [newId] = useState(() => Crypto.randomUUID().replaceAll("-", ""));
  const colors = useColors();

  if (id === "new")
    return (
      <NoteEditor
        initial={{ id: newId, text: "", pinned: false, archived: false, color: "none" }}
        updatedAt={notes?.find((note) => note.id === newId)?.updatedAt}
        stored={false}
        autoDictate={dictate === "1"}
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
  return (
    <NoteEditor
      key={id}
      initial={{ id, text, pinned, archived, color }}
      updatedAt={note.updatedAt}
      stored
      autoDictate={false}
    />
  );
}

function NoteEditor({
  initial,
  updatedAt,
  stored,
  autoDictate,
}: {
  initial: Draft;
  updatedAt: number | undefined;
  stored: boolean;
  autoDictate: boolean;
}) {
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const keyboard = useKeyboardHeight();
  const colors = useColors();
  const noteColors = useNoteColors();
  const actions = useNoteActions();
  const { draft, error, update, discard } = useNoteDraft(initial, stored);
  const body = useRef<{ edit: () => void }>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const noticeTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(noticeTimer.current), []);
  const message = notice ?? error;
  // Title and body live apart while editing: the server trims text, so a note typed without a
  // title would otherwise have its first body line jump into the title field.
  const [parts, setParts] = useState(() => splitTitle(initial.text));
  const latestParts = useRef(parts);

  function showNotice(message: string) {
    setNotice(message);
    clearTimeout(noticeTimer.current);
    noticeTimer.current = setTimeout(() => setNotice(null), 4000);
  }

  function edit(change: (current: typeof parts) => Partial<typeof parts>) {
    latestParts.current = { ...latestParts.current, ...change(latestParts.current) };
    setParts(latestParts.current);
    const { title, body } = latestParts.current;
    update({ text: joinTitle(title, body).slice(0, MAX_LENGTH) });
  }

  const togglePin = () => {
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    update({ pinned: !draft.pinned, archived: false }, { now: true });
  };

  const pin = (
    <IconButton
      label={draft.pinned ? "Unpin" : "Pin"}
      ios={draft.pinned ? "pin.fill" : "pin"}
      android="push_pin"
      active={draft.pinned}
      onPress={togglePin}
    />
  );
  const share = (
    <IconButton
      label="Share"
      ios="square.and.arrow.up"
      android="share"
      disabled={!draft.text.trim()}
      onPress={() =>
        void Share.share({ message: draft.text.trim() }).catch(() =>
          showNotice("Could not share note."),
        )
      }
    />
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
            {pin}
            {share}
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
            {pin}
            {share}
          </>
        )}
      </View>
      <TextInput
        value={parts.title}
        multiline
        submitBehavior="submit"
        style={[styles.title, { color: colors.text }]}
        placeholder="Title"
        placeholderTextColor={colors.tertiary}
        selectionColor={colors.accent}
        onChangeText={(title) => edit(() => ({ title: title.replaceAll("\n", " ") }))}
        onSubmitEditing={() => body.current?.edit()}
      />
      <NoteBody
        ref={body}
        text={parts.body}
        startEditing={!initial.text.trim() && !autoDictate}
        onChange={(body) => edit(() => ({ body }))}
        onToggle={(line) => {
          void Haptics.selectionAsync();
          edit(({ body }) => ({ body: toggleChecklist(body, line) }));
        }}
      />
      {message && <Text style={[styles.notice, { color: colors.secondary }]}>{message}</Text>}
      <View style={{ paddingBottom: Math.max(keyboard, insets.bottom) }}>
        <EditorBar
          color={draft.color}
          editedAt={updatedAt}
          archived={draft.archived}
          autoDictate={autoDictate}
          onColor={(color) => {
            void Haptics.selectionAsync();
            update({ color }, { now: true });
          }}
          onArchive={() => {
            void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
            update({ archived: !draft.archived, pinned: false }, { now: true });
            router.back();
          }}
          onDelete={() =>
            actions.confirmDelete([draft.id], () => {
              discard();
              router.back();
            })
          }
          onDictated={(dictated) =>
            edit(({ body }) => ({
              body: body.trim() ? `${body.trimEnd()}\n${dictated}` : dictated,
            }))
          }
          onNotice={showNotice}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1 },
  top: {
    flexDirection: "row",
    alignItems: "center",
    paddingHorizontal: ios ? 10 : 4,
    minHeight: 64,
  },
  trailing: { marginLeft: "auto" },
  done: { fontSize: 17, fontWeight: "600", paddingHorizontal: 8 },
  title: {
    fontSize: 24,
    lineHeight: 32,
    fontWeight: ios ? "700" : "400",
    paddingHorizontal: 16,
    paddingTop: 4,
    paddingBottom: 8,
  },
  missing: { fontSize: 15, padding: 20 },
  notice: { fontSize: 13, paddingHorizontal: 16, paddingVertical: 6 },
});
