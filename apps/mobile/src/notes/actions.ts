import { api } from "@typeaside/backend/convex/_generated/api";
import type { NoteColor } from "@typeaside/backend/convex/noteColors";
import { useMutation } from "convex/react";
import { ConvexError } from "convex/values";
import * as Haptics from "expo-haptics";
import { Alert } from "react-native";
import type { Draft, Note } from "./model";

export function failureMessage(error: unknown) {
  return error instanceof ConvexError && typeof error.data === "string"
    ? error.data
    : "Check your connection and try again.";
}

export function useNoteActions() {
  const save = useMutation(api.notes.save);
  const remove = useMutation(api.notes.remove);

  function update(notes: Note[], changes: Partial<Draft>) {
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    Promise.all(
      notes.map((note) => {
        const { id, text, pinned, archived, color } = { ...note, ...changes };
        return save({ id, text, pinned, archived, color });
      }),
    ).catch((error: unknown) => Alert.alert("Could not update note", failureMessage(error)));
  }

  return {
    pin: (notes: Note[], pinned: boolean) => update(notes, { pinned, archived: false }),
    archive: (notes: Note[], archived: boolean) => update(notes, { archived, pinned: false }),
    color: (notes: Note[], color: NoteColor) => update(notes, { color }),
    confirmDelete: (ids: string[], onDeleted?: () => void) =>
      Alert.alert(
        ids.length > 1 ? `Delete ${ids.length} notes?` : "Delete this note?",
        "It will be removed from all your devices.",
        [
          { text: "Cancel", style: "cancel" },
          {
            text: "Delete",
            style: "destructive",
            onPress: () => {
              void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Warning);
              onDeleted?.();
              Promise.all(ids.map((id) => remove({ id }))).catch((error: unknown) =>
                Alert.alert("Could not delete note", failureMessage(error)),
              );
            },
          },
        ],
      ),
  };
}

export type NoteActions = ReturnType<typeof useNoteActions>;
