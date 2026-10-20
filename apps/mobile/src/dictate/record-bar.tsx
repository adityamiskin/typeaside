import {
  ExtendedFloatingActionButton,
  FloatingActionButton,
  Host,
  Icon,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import { Pressable, StyleSheet, Text as NativeText, View } from "react-native";
import { icons } from "@/ui/icons";
import { seedColor } from "@/ui/theme";
import { formatElapsed } from "./format";
import { type RecordBarProps, useRecordBar } from "./use-record-bar";

// Compose has no press-release callback, so a Pressable owns the gesture and the Material
// buttons inside it only draw.
export function RecordBar(props: RecordBarProps) {
  const { dictation, press, bars, cancel } = useRecordBar(props);
  const material = useMaterialColors({ seedColor });

  if (!dictation.recording)
    return (
      <Pressable
        {...press}
        android_ripple={{ color: material.onPrimaryContainer, foreground: true }}
        style={[styles.fab, props.busy && styles.busy]}
      >
        <Host matchContents pointerEvents="none" seedColor={seedColor}>
          <ExtendedFloatingActionButton>
            <ExtendedFloatingActionButton.Icon>
              <Icon source={icons.mic} size={24} />
            </ExtendedFloatingActionButton.Icon>
            <ExtendedFloatingActionButton.Text>
              <Text>{props.busy ? "Cleaning up" : "Dictate"}</Text>
            </ExtendedFloatingActionButton.Text>
          </ExtendedFloatingActionButton>
        </Host>
      </Pressable>
    );

  return (
    <Pressable {...press} style={[styles.sheet, { backgroundColor: material.surfaceContainer }]}>
      <Pressable
        accessibilityRole="button"
        hitSlop={8}
        android_ripple={{ color: material.primary, borderless: true }}
        onPress={cancel}
        style={styles.cancel}
      >
        <NativeText style={[styles.label, { color: material.primary }]}>Cancel</NativeText>
      </Pressable>
      <View style={styles.levels}>
        {bars.map((level, index) => (
          <View
            key={index}
            style={[
              styles.level,
              {
                height: 4 + (level ?? 0) * 28,
                backgroundColor: level === undefined ? material.outlineVariant : material.primary,
              },
            ]}
          />
        ))}
      </View>
      <NativeText style={[styles.time, { color: material.onSurface }]}>
        {formatElapsed(dictation.elapsedMs)}
      </NativeText>
      <Host matchContents pointerEvents="none" seedColor={seedColor}>
        <FloatingActionButton containerColor={material.errorContainer}>
          <FloatingActionButton.Icon>
            <Icon source={icons.stop} size={24} tint={material.onErrorContainer} />
          </FloatingActionButton.Icon>
        </FloatingActionButton>
      </Host>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  fab: { alignSelf: "flex-end", margin: 16, borderRadius: 16 },
  busy: { opacity: 0.6 },
  sheet: {
    flexDirection: "row",
    alignItems: "center",
    gap: 12,
    paddingLeft: 8,
    paddingRight: 16,
    paddingVertical: 16,
    borderTopLeftRadius: 28,
    borderTopRightRadius: 28,
  },
  cancel: { paddingHorizontal: 12, height: 40, justifyContent: "center", borderRadius: 20 },
  label: { fontSize: 14, fontWeight: "500", letterSpacing: 0.1 },
  levels: { flex: 1, height: 32, flexDirection: "row", alignItems: "center", gap: 2 },
  level: { flex: 1, borderRadius: 2 },
  time: { fontSize: 16, fontWeight: "500", fontVariant: ["tabular-nums"] },
});
