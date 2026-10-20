import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import * as Clipboard from "expo-clipboard";
import * as Crypto from "expo-crypto";
import { File, Paths } from "expo-file-system";
import * as Haptics from "expo-haptics";
import { useState } from "react";
import { Alert, Linking, Share } from "react-native";
import { DictateView } from "@/dictate/dictate-view";
import type { Action } from "@/dictate/format";
import { RecordBar } from "@/dictate/record-bar";
import { type Take, useTakes } from "@/dictate/takes";
import { DictationError } from "@/lib/dictate";
import { MicrophoneDenied, transcribe } from "@/lib/recorder";

export default function DictateScreen() {
  const { takes, add, update, remove } = useTakes();
  const account = useQuery(api.viewer.current);
  const saveNote = useMutation(api.notes.save);
  const [expandedId, setExpandedId] = useState<string>();
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

  function recorded(recording: string) {
    // The recorder writes to the cache, which the OS may clear before a retry.
    const audio = new File(recording);
    audio.moveSync(Paths.document);
    const take = { id: Crypto.randomUUID(), createdAt: Date.now(), text: "" };
    add({ ...take, pending: { uri: audio.uri } });
    setExpandedId(take.id);
    void process(take, audio.uri);
  }

  function failed(error: Error) {
    if (error instanceof MicrophoneDenied)
      Alert.alert("Microphone is off", error.message, [
        { text: "Not now", style: "cancel" },
        { text: "Settings", onPress: () => void Linking.openSettings() },
      ]);
    else Alert.alert("Could not start recording", error.message);
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
        onPress: () => {
          void Clipboard.setStringAsync(take.text);
          void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
        },
      },
      {
        label: "Share",
        icon: "share",
        onPress: () => void Share.share({ message: take.text }),
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
    <DictateView
      takes={takes}
      expandedId={expandedId}
      onToggle={(take) => setExpandedId(take.id === expandedId ? undefined : take.id)}
      actions={actions}
      usage={account?.usage}
      recordBar={
        <RecordBar busy={busy} hero={takes.length === 0} onRecorded={recorded} onError={failed} />
      }
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
