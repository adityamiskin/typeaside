import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation } from "convex/react";
import * as Clipboard from "expo-clipboard";
import * as Crypto from "expo-crypto";
import * as Haptics from "expo-haptics";
import { useState } from "react";
import { Alert } from "react-native";
import { HistoryView } from "@/dictate/history-view";
import type { Action } from "@/dictate/format";
import { type Take, useTakes } from "@/dictate/takes";

export default function HistoryScreen() {
  const { takes, update, remove } = useTakes();
  const saveNote = useMutation(api.notes.save);
  const [copied, setCopied] = useState(0);
  function copy(take: Take) {
    void Clipboard.setStringAsync(take.text)
      .then(() => {
        setCopied((count) => count + 1);
        void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
      })
      .catch(() => Alert.alert("Could not copy dictation."));
  }

  function actions(take: Take): Action[] {
    const discard: Action = {
      label: "Delete",
      icon: "delete",
      onPress: () => {
        remove(take.id);
      },
    };
    return [
      {
        label: "Copy",
        icon: "copy",
        onPress: () => copy(take),
      },
      {
        label: take.noteId ? "Saved" : "Save as note",
        icon: take.noteId ? "saved" : "save",
        onPress: () => {
          if (take.noteId) return;
          const id = Crypto.randomUUID().replaceAll("-", "");
          void saveNote({ id, text: take.text, pinned: false, archived: false, color: "none" })
            .then(() => {
              update(take.id, { noteId: id });
              void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
            })
            .catch((error: Error) => Alert.alert("Could not save note", error.message));
        },
      },
      discard,
    ];
  }

  return <HistoryView takes={takes} copied={copied} onCopy={copy} actions={actions} />;
}
