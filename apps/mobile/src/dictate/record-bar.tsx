import {
  ExtendedFloatingActionButton,
  FilledTonalIconButton,
  Host,
  Icon,
  LargeFloatingActionButton,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import { clip, Shapes } from "@expo/ui/jetpack-compose/modifiers";
import { Pressable, StyleSheet, Text as NativeText, View } from "react-native";
import { icons } from "@/ui/icons";
import { seedColor } from "@/ui/theme";
import { formatElapsed } from "./format";
import { type RecordBarProps, useRecordBar } from "./use-record-bar";

// Compose has no press-release callback, so Pressables own the gestures and the Material
// buttons inside them only draw. The outer Pressable stays the first child in every state so a
// hold that starts on the idle button is released on the recording sheet. Host ignores
// pointerEvents, so a plain View keeps touches off the idle Hosts: a release targeted at an
// unmounted Host never reaches the Pressable.
export function RecordBar(props: RecordBarProps) {
  const { dictation, press, bars, stop, cancel } = useRecordBar(props);
  const material = useMaterialColors({ seedColor });
  const hero = props.hero && !dictation.recording && !props.busy;

  return (
    <View
      pointerEvents="box-none"
      style={[styles.root, hero ? styles.heroRoot : !dictation.recording && styles.fabRoot]}
    >
      <Pressable
        {...press}
        // Transparent rather than off while recording, so releasing a hold still clears the ripple.
        android_ripple={{
          color: dictation.recording ? "transparent" : material.onPrimaryContainer,
          foreground: true,
        }}
        style={[
          dictation.recording
            ? [styles.sheet, { backgroundColor: material.surfaceContainerHigh }]
            : hero
              ? styles.heroFab
              : styles.fab,
          props.busy && styles.busy,
        ]}
      >
        {dictation.recording ? (
          <>
            <View style={styles.handle}>
              <View style={[styles.handleBar, { backgroundColor: material.outlineVariant }]} />
            </View>
            <View style={styles.timer}>
              <View style={[styles.dot, { backgroundColor: material.error }]} />
              <NativeText style={[styles.time, { color: material.onSurface }]}>
                {formatElapsed(dictation.elapsedMs)}
              </NativeText>
            </View>
            <View style={styles.wave}>
              {bars.map((level, index) => (
                <View
                  key={index}
                  style={[
                    styles.bar,
                    {
                      height: 4 + (level ?? 0) * 60,
                      backgroundColor:
                        level === undefined ? material.outlineVariant : material.primary,
                    },
                  ]}
                />
              ))}
            </View>
            <View style={styles.controls}>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Cancel"
                onPress={cancel}
                style={styles.side}
              >
                <View pointerEvents="none" collapsable={false}>
                  <Host matchContents seedColor={seedColor}>
                    <FilledTonalIconButton>
                      <Icon source={icons.close} size={24} />
                    </FilledTonalIconButton>
                  </Host>
                </View>
              </Pressable>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Stop dictating"
                onPress={stop}
                android_ripple={{ color: material.onPrimary, foreground: true }}
                style={styles.stop}
              >
                <View pointerEvents="none" collapsable={false}>
                  <Host matchContents seedColor={seedColor}>
                    <LargeFloatingActionButton
                      containerColor={material.primary}
                      modifiers={[clip(Shapes.Circle)]}
                    >
                      <LargeFloatingActionButton.Icon>
                        <Icon source={icons.stop} size={36} tint={material.onPrimary} />
                      </LargeFloatingActionButton.Icon>
                    </LargeFloatingActionButton>
                  </Host>
                </View>
              </Pressable>
              <View style={styles.side} />
            </View>
          </>
        ) : hero ? (
          <View pointerEvents="none" collapsable={false}>
            <Host matchContents seedColor={seedColor}>
              <LargeFloatingActionButton>
                <LargeFloatingActionButton.Icon>
                  <Icon source={icons.mic} size={36} />
                </LargeFloatingActionButton.Icon>
              </LargeFloatingActionButton>
            </Host>
          </View>
        ) : (
          <View pointerEvents="none" collapsable={false}>
            <Host matchContents seedColor={seedColor}>
              <ExtendedFloatingActionButton>
                <ExtendedFloatingActionButton.Icon>
                  <Icon source={icons.mic} size={24} />
                </ExtendedFloatingActionButton.Icon>
                <ExtendedFloatingActionButton.Text>
                  <Text>{props.busy ? "Cleaning up" : "Dictate"}</Text>
                </ExtendedFloatingActionButton.Text>
              </ExtendedFloatingActionButton>
            </Host>
          </View>
        )}
      </Pressable>
      {hero && (
        <View style={styles.heroCopy} pointerEvents="none">
          <NativeText style={[styles.heroTitle, { color: material.onSurface }]}>
            Tap to dictate
          </NativeText>
          <NativeText style={[styles.heroHint, { color: material.onSurfaceVariant }]}>
            Or hold to talk, release to finish
          </NativeText>
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, justifyContent: "flex-end" },
  heroRoot: { justifyContent: "center", alignItems: "center", paddingBottom: 48 },
  fabRoot: { alignItems: "flex-end" },
  fab: { margin: 16, borderRadius: 16 },
  heroFab: { borderRadius: 28 },
  busy: { opacity: 0.6 },
  heroCopy: { alignItems: "center", gap: 4, marginTop: 24 },
  heroTitle: { fontSize: 22, lineHeight: 28 },
  heroHint: { fontSize: 14, lineHeight: 20, letterSpacing: 0.25 },
  sheet: {
    alignItems: "center",
    paddingHorizontal: 24,
    paddingBottom: 24,
    borderTopLeftRadius: 28,
    borderTopRightRadius: 28,
  },
  handle: { height: 36, justifyContent: "center" },
  handleBar: { width: 32, height: 4, borderRadius: 2 },
  timer: { flexDirection: "row", alignItems: "center", gap: 10 },
  dot: { width: 10, height: 10, borderRadius: 5 },
  time: { fontSize: 32, lineHeight: 40, fontVariant: ["tabular-nums"] },
  wave: {
    alignSelf: "stretch",
    height: 72,
    marginVertical: 16,
    flexDirection: "row",
    alignItems: "center",
    gap: 3,
  },
  bar: { flex: 1, borderRadius: 2 },
  controls: {
    alignSelf: "stretch",
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
  },
  stop: { borderRadius: 48, overflow: "hidden" },
  side: { width: 56, height: 56, alignItems: "center", justifyContent: "center" },
});
