import { type NoteColor, noteColors as noteColorNames } from "@typeaside/backend/convex/noteColors";
import { SymbolView } from "expo-symbols";
import { useEffect, useEffectEvent, useRef, useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { DictationError } from "@/lib/dictate";
import { MicrophoneDenied, transcribe, useDictation } from "@/lib/recorder";
import { colors, noteColors } from "@/ui/theme";

export function EditorBar({
  color,
  onColor,
  onDictated,
  onNotice,
}: {
  color: NoteColor;
  onColor: (color: NoteColor) => void;
  onDictated: (text: string) => void;
  onNotice: (message: string) => void;
}) {
  const [dictating, setDictating] = useState(false);

  // The recorder polls its state while mounted, so it only exists during a take.
  if (dictating)
    return (
      <Dictation onDictated={onDictated} onNotice={onNotice} onEnd={() => setDictating(false)} />
    );

  return (
    <View style={styles.bar}>
      {noteColorNames.map((name) => (
        <Pressable
          key={name}
          accessibilityRole="radio"
          accessibilityLabel={name === "none" ? "No color" : name}
          accessibilityState={{ selected: color === name }}
          hitSlop={6}
          onPress={() => onColor(name)}
          style={[styles.ring, color === name && styles.ringSelected]}
        >
          <View
            style={[
              styles.swatch,
              { backgroundColor: name === "none" ? colors.background : noteColors[name].accent },
            ]}
          >
            {name === "none" && <View style={styles.slash} />}
          </View>
        </Pressable>
      ))}
      <Pressable
        accessibilityLabel="Dictate"
        hitSlop={10}
        style={styles.trailing}
        onPress={() => setDictating(true)}
      >
        <SymbolView name={{ ios: "mic", android: "mic" }} tintColor={colors.text} size={22} />
      </Pressable>
    </View>
  );
}

function Dictation({
  onDictated,
  onNotice,
  onEnd,
}: {
  onDictated: (text: string) => void;
  onNotice: (message: string) => void;
  onEnd: () => void;
}) {
  const dictation = useDictation();
  const [transcribing, setTranscribing] = useState(false);
  const phase = useRef<"starting" | "recording" | "stopping" | "ended">("starting");
  const active = useRef(true);

  const begin = useEffectEvent(async () => {
    try {
      await dictation.start();
      phase.current = "recording";
    } catch (error) {
      phase.current = "ended";
      if (active.current) {
        onNotice(error instanceof MicrophoneDenied ? error.message : "Could not start recording.");
        onEnd();
      }
    }
  });
  const abandon = useEffectEvent(() => {
    if (phase.current !== "recording") return;
    phase.current = "ended";
    void dictation.cancel().catch(() => undefined);
  });
  useEffect(() => {
    active.current = true;
    const started = begin();
    return () => {
      active.current = false;
      // Permission prompts can finish after the sheet has already closed.
      void started.then(() => abandon());
    };
  }, []);

  async function finish() {
    if (phase.current !== "recording") return;
    phase.current = "stopping";
    setTranscribing(true);
    try {
      const uri = await dictation.stop();
      if (uri) {
        const { text } = await transcribe(uri);
        if (active.current && text.trim()) onDictated(text.trim());
      }
    } catch (error) {
      if (active.current)
        onNotice(error instanceof DictationError ? error.message : "Dictation failed. Try again.");
    } finally {
      phase.current = "ended";
      if (active.current) onEnd();
    }
  }

  if (transcribing)
    return (
      <View style={styles.bar}>
        <Text style={styles.status}>Transcribing</Text>
      </View>
    );

  return (
    <View style={styles.bar}>
      <Pressable
        accessibilityLabel="Cancel dictation"
        disabled={!dictation.recording}
        hitSlop={10}
        onPress={() => {
          if (phase.current !== "recording") return;
          phase.current = "ended";
          void dictation.cancel().catch(() => undefined);
          onEnd();
        }}
      >
        <SymbolView
          name={{ ios: "xmark", android: "close" }}
          tintColor={colors.secondary}
          size={20}
        />
      </Pressable>
      <View style={styles.meter}>
        <View style={[styles.level, { width: `${Math.round(dictation.level * 100)}%` }]} />
      </View>
      <Text style={styles.time}>{formatElapsed(dictation.elapsedMs)}</Text>
      <Pressable
        accessibilityLabel="Stop and insert"
        hitSlop={10}
        disabled={!dictation.recording}
        onPress={() => void finish()}
        style={styles.stop}
      >
        <View style={styles.stopSquare} />
      </Pressable>
    </View>
  );
}

function formatElapsed(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

const styles = StyleSheet.create({
  bar: {
    minHeight: 52,
    flexDirection: "row",
    alignItems: "center",
    gap: 12,
    paddingHorizontal: 16,
    borderTopWidth: StyleSheet.hairlineWidth,
    borderTopColor: "rgba(255,255,255,0.08)",
  },
  ring: { padding: 2, borderRadius: 17, borderWidth: 2, borderColor: "transparent" },
  ringSelected: { borderColor: colors.text },
  swatch: {
    width: 24,
    height: 24,
    borderRadius: 12,
    borderWidth: 1,
    borderColor: "#2f3034",
    overflow: "hidden",
    alignItems: "center",
    justifyContent: "center",
  },
  slash: { width: 30, height: 1.5, backgroundColor: "#555", transform: [{ rotate: "-45deg" }] },
  trailing: { marginLeft: "auto" },
  status: { color: colors.secondary, fontSize: 15 },
  meter: {
    flex: 1,
    height: 4,
    borderRadius: 2,
    backgroundColor: "rgba(255,255,255,0.12)",
    overflow: "hidden",
  },
  level: { height: 4, backgroundColor: colors.text },
  time: { color: colors.text, fontSize: 15, fontVariant: ["tabular-nums"] },
  stop: {
    width: 36,
    height: 36,
    borderRadius: 18,
    backgroundColor: colors.danger,
    alignItems: "center",
    justifyContent: "center",
  },
  stopSquare: { width: 12, height: 12, borderRadius: 3, backgroundColor: colors.background },
});
