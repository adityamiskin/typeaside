import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import * as Clipboard from "expo-clipboard";
import * as Crypto from "expo-crypto";
import { File } from "expo-file-system";
import * as Haptics from "expo-haptics";
import { useState } from "react";
import { Alert } from "react-native";
import { HistoryView } from "@/dictate/history-view";
import type { Action } from "@/dictate/format";
import { type Take, useTakes } from "@/dictate/takes";
import { DictationError } from "@/lib/dictate";
import { transcribe } from "@/lib/recorder";

export default function HistoryScreen() {
  const { takes, update, remove } = useTakes();
  const account = useQuery(api.viewer.current);
  const saveNote = useMutation(api.notes.save);
  const [copied, setCopied] = useState(0);
  const [busy, setBusy] = useState(false);

  async function process(take: Take, uri: string) {
    setBusy(true);
    update(take.id, { pending: { uri } });
    try {
      const { text } = await transcribe(uri);
      update(take.id, { text, pending: undefined });
      void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
    } catch (error) {
      update(take.id, { pending: { uri, error: describe(error) } });
      void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Error);
    } finally {
      setBusy(false);
    }
  }

  function copy(take: Take) {
    if (take.pending) return;
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
        if (take.pending) new File(take.pending.uri).delete();
        remove(take.id);
      },
    };
    const pending = take.pending;
    if (pending)
      return busy
        ? []
        : [
            {
              label: "Retry",
              icon: "retry",
              onPress: () => void process(take, pending.uri),
            },
            discard,
          ];
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

  return (
    <HistoryView
      takes={takes}
      copied={copied}
      onCopy={copy}
      actions={actions}
      usage={account?.usage}
    />
  );
}

function describe(error: unknown) {
  if (error instanceof DictationError) {
    if (error.type === "limit" && error.resetsAt)
      return `Weekly limit reached, resets ${new Date(error.resetsAt).toLocaleDateString([], { weekday: "long" })}`;
    if (error.type === "network") return "Offline";
    return error.message;
  }
  return "Offline";
}
