import { Button, Host } from "@expo/ui/swift-ui";
import { buttonBorderShape, buttonStyle, controlSize } from "@expo/ui/swift-ui/modifiers";
import { PlatformColor, Pressable, StyleSheet, Text, View } from "react-native";
import { formatElapsed } from "./format";
import { type RecordBarProps, useRecordBar } from "./use-record-bar";

// SwiftUI buttons have no press-release callback, so a Pressable owns the gesture and the
// button inside it only draws.
export function RecordBar(props: RecordBarProps) {
  const { dictation, press, bars, cancel } = useRecordBar(props);

  if (!dictation.recording)
    return (
      <Pressable
        {...press}
        style={({ pressed }) => [styles.idle, (pressed || props.busy) && styles.dim]}
      >
        <Host matchContents pointerEvents="none">
          <Button
            label={props.busy ? "Cleaning up" : "Dictate"}
            systemImage="mic.fill"
            modifiers={[
              buttonStyle("glassProminent"),
              buttonBorderShape("capsule"),
              controlSize("extraLarge"),
            ]}
          />
        </Host>
      </Pressable>
    );

  return (
    <Pressable {...press} style={styles.recording}>
      <Pressable hitSlop={12} onPress={cancel}>
        <Text style={styles.cancel}>Cancel</Text>
      </Pressable>
      <View style={styles.levels}>
        {bars.map((level, index) => (
          <View
            key={index}
            style={[
              styles.level,
              { height: 4 + (level ?? 0) * 28, opacity: level === undefined ? 0.3 : 1 },
            ]}
          />
        ))}
      </View>
      <Text style={styles.time}>{formatElapsed(dictation.elapsedMs)}</Text>
      <View style={styles.stop}>
        <View style={styles.stopSquare} />
      </View>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  idle: { alignSelf: "center", marginBottom: 12 },
  dim: { opacity: 0.6 },
  recording: {
    height: 52,
    flexDirection: "row",
    alignItems: "center",
    gap: 12,
    marginHorizontal: 16,
    marginBottom: 12,
  },
  cancel: { color: PlatformColor("systemBlue"), fontSize: 17 },
  levels: { flex: 1, height: 32, flexDirection: "row", alignItems: "center", gap: 2 },
  level: { flex: 1, borderRadius: 1.5, backgroundColor: PlatformColor("label") },
  time: { color: PlatformColor("label"), fontSize: 17, fontVariant: ["tabular-nums"] },
  stop: {
    width: 52,
    height: 52,
    borderRadius: 26,
    backgroundColor: PlatformColor("systemRed"),
    alignItems: "center",
    justifyContent: "center",
  },
  stopSquare: { width: 18, height: 18, borderRadius: 4, backgroundColor: PlatformColor("white") },
});
