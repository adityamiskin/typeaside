import { SymbolView } from "expo-symbols";
import { useEffect, useRef, useState } from "react";
import { Platform, Pressable, StyleSheet, Text, View } from "react-native";
import { colors } from "@/ui/theme";
import { useDictation } from "@/lib/recorder";

const BARS = 28;

export function RecordBar({
  busy,
  onRecorded,
  onError,
}: {
  busy: boolean;
  onRecorded: (uri: string) => void;
  onError: (error: Error) => void;
}) {
  const dictation = useDictation();
  const [levels, setLevels] = useState<number[]>([]);
  const heldRef = useRef(false);

  useEffect(() => {
    if (dictation.recording) setLevels((current) => [...current, dictation.level].slice(-BARS));
    else setLevels([]);
  }, [dictation.recording, dictation.elapsedMs, dictation.level]);

  async function start() {
    await dictation.start().catch(onError);
  }
  async function stop() {
    const uri = await dictation.stop();
    if (uri) onRecorded(uri);
  }

  // One Pressable for both states so releasing a hold still reaches it after the swap.
  return (
    <Pressable
      disabled={busy}
      accessibilityLabel={dictation.recording ? "Stop dictating" : "Dictate"}
      onPress={() => void (dictation.recording ? stop() : start())}
      onLongPress={() => {
        if (dictation.recording) return;
        heldRef.current = true;
        void start();
      }}
      onPressOut={() => {
        if (!heldRef.current) return;
        heldRef.current = false;
        void stop();
      }}
      style={({ pressed }) => [
        dictation.recording ? styles.recording : styles.idle,
        !dictation.recording && (pressed || busy) && { opacity: 0.7 },
      ]}
    >
      {dictation.recording ? (
        <>
          <Pressable hitSlop={12} onPress={() => void dictation.cancel()}>
            <Text style={styles.cancel}>Cancel</Text>
          </Pressable>
          <View style={styles.levels}>
            {Array.from({ length: BARS }, (_, index) => {
              const level = levels[index - (BARS - levels.length)];
              return (
                <View
                  key={index}
                  style={[
                    styles.level,
                    { height: 4 + (level ?? 0) * 28, opacity: level === undefined ? 0.3 : 1 },
                  ]}
                />
              );
            })}
          </View>
          <Text style={styles.time}>{formatElapsed(dictation.elapsedMs)}</Text>
          <View style={styles.stop}>
            <View style={styles.stopSquare} />
          </View>
        </>
      ) : (
        <>
          <SymbolView
            name={{ ios: "mic.fill", android: "mic" }}
            tintColor={colors.onPrimary}
            size={20}
          />
          <Text style={styles.idleLabel}>{busy ? "Cleaning up" : "Tap or hold to dictate"}</Text>
        </>
      )}
    </Pressable>
  );
}

function formatElapsed(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

const styles = StyleSheet.create({
  idle: {
    height: 52,
    borderRadius: Platform.OS === "ios" ? 26 : 16,
    backgroundColor: colors.text,
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "center",
    gap: 8,
  },
  idleLabel: { color: colors.onPrimary, fontSize: 16, fontWeight: "600" },
  recording: { height: 52, flexDirection: "row", alignItems: "center", gap: 12 },
  cancel: { color: colors.secondary, fontSize: 15 },
  levels: { flex: 1, height: 32, flexDirection: "row", alignItems: "center", gap: 2 },
  level: { flex: 1, borderRadius: 1.5, backgroundColor: colors.text },
  time: { color: colors.text, fontSize: 17, fontVariant: ["tabular-nums"], fontWeight: "500" },
  stop: {
    width: 52,
    height: 52,
    borderRadius: 26,
    backgroundColor: colors.danger,
    alignItems: "center",
    justifyContent: "center",
  },
  stopSquare: { width: 18, height: 18, borderRadius: 4, backgroundColor: colors.background },
});
