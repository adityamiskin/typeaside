import { type NoteColor, noteColors as noteColorNames } from "@typeaside/backend/convex/noteColors";
import type { AndroidSymbol, SFSymbol } from "expo-symbols";
import { SymbolView } from "expo-symbols";
import { useEffect, useEffectEvent, useRef, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import { DictationError } from "@/lib/dictate";
import { MicrophoneDenied, transcribe, useDictation } from "@/lib/recorder";
import { useColors, useNoteColors } from "@/ui/theme";
import { relativeTime } from "./model";

export function EditorBar({
  color,
  editedAt,
  archived,
  autoDictate,
  onColor,
  onArchive,
  onDelete,
  onDictated,
  onNotice,
}: {
  color: NoteColor;
  editedAt: number | undefined;
  archived: boolean;
  autoDictate: boolean;
  onColor: (color: NoteColor) => void;
  onArchive: () => void;
  onDelete: () => void;
  onDictated: (text: string) => void;
  onNotice: (message: string) => void;
}) {
  const [dictating, setDictating] = useState(autoDictate);
  const [palette, setPalette] = useState(false);
  const colors = useColors();
  const noteColors = useNoteColors();

  // The recorder polls its state while mounted, so it only exists during a take.
  if (dictating)
    return (
      <Dictation onDictated={onDictated} onNotice={onNotice} onEnd={() => setDictating(false)} />
    );

  return (
    <View>
      {palette && (
        <ScrollView
          horizontal
          showsHorizontalScrollIndicator={false}
          contentContainerStyle={styles.palette}
        >
          {noteColorNames.map((name) => (
            <Pressable
              key={name}
              accessibilityRole="radio"
              accessibilityLabel={name === "none" ? "Default" : name}
              accessibilityState={{ selected: color === name }}
              onPress={() => onColor(name)}
              style={[
                styles.swatch,
                {
                  borderColor: color === name ? colors.accent : colors.separator,
                  borderWidth: color === name ? 2 : 1,
                  backgroundColor:
                    name === "none" ? colors.background : noteColors[name].background,
                },
              ]}
            >
              {color === name ? (
                <SymbolView
                  name={{ ios: "checkmark", android: "check" }}
                  tintColor={colors.accent}
                  size={20}
                />
              ) : (
                name === "none" && (
                  <SymbolView
                    name={{ ios: "drop.degreesign.slash", android: "format_color_reset" }}
                    tintColor={colors.secondary}
                    size={20}
                  />
                )
              )}
            </Pressable>
          ))}
        </ScrollView>
      )}
      <View style={styles.bar}>
        <IconButton
          label="Color"
          ios="paintpalette"
          android="palette"
          active={palette}
          onPress={() => setPalette(!palette)}
        />
        <IconButton label="Dictate" ios="mic" android="mic" onPress={() => setDictating(true)} />
        <Text style={[styles.edited, { color: colors.secondary }]} numberOfLines={1}>
          {editedAt ? `Edited ${relativeTime(editedAt)}` : ""}
        </Text>
        <IconButton
          label={archived ? "Unarchive" : "Archive"}
          ios={archived ? "tray.and.arrow.up" : "archivebox"}
          android={archived ? "unarchive" : "archive"}
          onPress={onArchive}
        />
        <IconButton label="Delete" ios="trash" android="delete" onPress={onDelete} />
      </View>
    </View>
  );
}

export function IconButton({
  label,
  ios,
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
      disabled={disabled}
      onPress={onPress}
      android_ripple={{ borderless: true, radius: 20, color: colors.separator }}
      style={({ pressed }) => [
        styles.icon,
        ((process.env.EXPO_OS === "ios" && pressed) || disabled) && styles.dim,
      ]}
    >
      <SymbolView
        name={{ ios, android }}
        tintColor={active ? colors.accent : colors.text}
        size={24}
      />
    </Pressable>
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
  const colors = useColors();
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
      <View style={styles.recording}>
        <Text style={[styles.status, { color: colors.secondary }]}>Transcribing</Text>
      </View>
    );

  return (
    <View style={styles.recording}>
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
      <View style={[styles.meter, { backgroundColor: colors.separator }]}>
        <View
          style={[
            styles.level,
            { backgroundColor: colors.accent, width: `${Math.round(dictation.level * 100)}%` },
          ]}
        />
      </View>
      <Text style={[styles.time, { color: colors.text }]}>
        {formatElapsed(dictation.elapsedMs)}
      </Text>
      <Pressable
        accessibilityLabel="Stop and insert"
        hitSlop={10}
        disabled={!dictation.recording}
        onPress={() => void finish()}
        style={[styles.stop, { backgroundColor: colors.danger }]}
      >
        <View style={[styles.stopSquare, { backgroundColor: colors.onAccent }]} />
      </Pressable>
    </View>
  );
}

function formatElapsed(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

const styles = StyleSheet.create({
  bar: { minHeight: 56, flexDirection: "row", alignItems: "center", paddingHorizontal: 4 },
  recording: {
    minHeight: 56,
    flexDirection: "row",
    alignItems: "center",
    gap: 16,
    paddingHorizontal: 16,
  },
  palette: { gap: 12, paddingHorizontal: 16, paddingVertical: 12 },
  swatch: {
    width: 44,
    height: 44,
    borderRadius: 22,
    alignItems: "center",
    justifyContent: "center",
  },
  edited: { flex: 1, fontSize: 12, textAlign: "center" },
  icon: { width: 48, height: 48, alignItems: "center", justifyContent: "center" },
  dim: { opacity: 0.4 },
  status: { fontSize: 15 },
  meter: {
    flex: 1,
    height: 4,
    borderRadius: 2,
    overflow: "hidden",
  },
  level: { height: 4 },
  time: { fontSize: 15, fontVariant: ["tabular-nums"] },
  stop: {
    width: 36,
    height: 36,
    borderRadius: 18,
    alignItems: "center",
    justifyContent: "center",
  },
  stopSquare: { width: 12, height: 12, borderRadius: 3 },
});
