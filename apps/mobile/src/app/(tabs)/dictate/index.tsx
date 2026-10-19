import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import * as Clipboard from "expo-clipboard";
import * as Crypto from "expo-crypto";
import { File, Paths } from "expo-file-system";
import * as Haptics from "expo-haptics";
import { Stack } from "expo-router";
import { useState } from "react";
import { Alert, FlatList, Linking, Share, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { RecordBar } from "@/dictate/record-bar";
import { type Action, TakeRow } from "@/dictate/take-row";
import { type Take, useTakes } from "@/dictate/takes";
import { DictationError } from "@/lib/dictate";
import { MicrophoneDenied, transcribe } from "@/lib/recorder";
import { colors } from "@/ui/theme";

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
    audio.move(Paths.document);
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
      ios: "trash",
      android: "delete",
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
              ios: "arrow.clockwise",
              android: "refresh",
              onPress: () => void process(take, pending.uri),
            },
            discard,
          ];
    return [
      {
        label: "Copy",
        ios: "doc.on.doc",
        android: "content_copy",
        onPress: () => {
          void Clipboard.setStringAsync(take.text);
          void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
        },
      },
      {
        label: "Share",
        ios: "square.and.arrow.up",
        android: "share",
        onPress: () => void Share.share({ message: take.text }),
      },
      {
        label: take.noteId ? "Saved" : "Save as note",
        ios: take.noteId ? "checkmark" : "note.text.badge.plus",
        android: take.noteId ? "check" : "note_add",
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

  const usage = account?.usage;
  return (
    <View style={styles.screen}>
      <Stack.Screen
        options={{
          headerRight: () =>
            usage ? (
              <Text style={styles.usage}>
                {usage.cap === null
                  ? "Pro"
                  : `${usage.words.toLocaleString()} / ${usage.cap.toLocaleString()}`}
              </Text>
            ) : null,
        }}
      />
      <FlatList
        data={takes}
        keyExtractor={(take) => take.id}
        contentInsetAdjustmentBehavior="automatic"
        keyboardDismissMode="on-drag"
        renderItem={({ item }) => (
          <TakeRow
            take={item}
            expanded={item.id === expandedId}
            onToggle={() => setExpandedId(item.id === expandedId ? undefined : item.id)}
            actions={actions(item)}
          />
        )}
        ListEmptyComponent={<Text style={styles.empty}>Your dictations appear here.</Text>}
      />
      <SafeAreaView edges={["bottom"]} style={styles.bar}>
        <RecordBar busy={busy} onRecorded={recorded} onError={failed} />
      </SafeAreaView>
    </View>
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

const styles = StyleSheet.create({
  screen: { flex: 1, backgroundColor: colors.background },
  usage: { color: colors.secondary, fontSize: 13, fontVariant: ["tabular-nums"] },
  empty: { color: colors.tertiary, fontSize: 15, padding: 16 },
  bar: { paddingHorizontal: 12, paddingTop: 8, paddingBottom: 8 },
});
