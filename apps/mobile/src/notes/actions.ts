import { api } from "@typeaside/backend/convex/_generated/api";
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

  function update(note: Note, changes: Partial<Draft>) {
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    const { id, text, pinned, archived, color } = { ...note, ...changes };
    save({ id, text, pinned, archived, color }).catch((error: unknown) =>
      Alert.alert("Could not update note", failureMessage(error)),
    );
  }

  return {
    togglePin: (note: Note) => update(note, { pinned: !note.pinned, archived: false }),
    toggleArchive: (note: Note) => update(note, { archived: !note.archived, pinned: false }),
    confirmDelete: (note: Note) =>
      Alert.alert("Delete this note?", "It will be removed from all your devices.", [
        { text: "Cancel", style: "cancel" },
        {
          text: "Delete",
          style: "destructive",
          onPress: () => {
            void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Warning);
            remove({ id: note.id }).catch((error: unknown) =>
              Alert.alert("Could not delete note", failureMessage(error)),
            );
          },
        },
      ]),
  };
}

export type NoteActions = ReturnType<typeof useNoteActions>;
