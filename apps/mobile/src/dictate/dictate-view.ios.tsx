import { Stack } from "expo-router";
import { type SFSymbol, SymbolView } from "expo-symbols";
import type { ReactNode } from "react";
import { FlatList, PlatformColor, Pressable, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { useColors } from "@/ui/theme";
import { type Action, takeMeta, takeText } from "./format";
import type { Take } from "./takes";

const actionSymbols = {
  copy: "doc.on.doc",
  share: "square.and.arrow.up",
  save: "note.text.badge.plus",
  saved: "checkmark",
  delete: "trash",
  retry: "arrow.clockwise",
} satisfies Record<Action["icon"], SFSymbol>;

type Props = {
  takes: Take[];
  expandedId: string | undefined;
  onToggle: (take: Take) => void;
  actions: (take: Take) => Action[];
  usage: string | undefined;
  recordBar: ReactNode;
};

export function DictateView({ takes, expandedId, onToggle, actions, usage, recordBar }: Props) {
  const colors = useColors();
  return (
    <View style={[styles.screen, { backgroundColor: colors.background }]}>
      <Stack.Screen
        options={{
          headerRight: () => (usage ? <Text style={styles.usage}>{usage}</Text> : null),
        }}
      />
      <FlatList
        data={takes}
        keyExtractor={(take) => take.id}
        contentInsetAdjustmentBehavior="automatic"
        contentContainerStyle={styles.list}
        renderItem={({ item, index }) => (
          <TakeRow
            take={item}
            first={index === 0}
            last={index === takes.length - 1}
            expanded={item.id === expandedId}
            onToggle={() => onToggle(item)}
            actions={actions(item)}
          />
        )}
        ListEmptyComponent={<Text style={styles.empty}>Your dictations appear here.</Text>}
      />
      <SafeAreaView edges={["bottom"]}>{recordBar}</SafeAreaView>
    </View>
  );
}

function TakeRow({
  take,
  first,
  last,
  expanded,
  onToggle,
  actions,
}: {
  take: Take;
  first: boolean;
  last: boolean;
  expanded: boolean;
  onToggle: () => void;
  actions: Action[];
}) {
  return (
    <View style={[styles.row, first && styles.first, last && styles.last]}>
      <Pressable
        onPress={onToggle}
        style={({ pressed }) => [styles.body, pressed && styles.pressed]}
      >
        <Text
          style={[styles.text, take.pending && styles.pendingText]}
          numberOfLines={expanded ? undefined : 3}
          selectable={expanded}
        >
          {takeText(take)}
        </Text>
        <Text style={[styles.meta, take.pending?.error && styles.error]}>{takeMeta(take)}</Text>
      </Pressable>
      {expanded && (
        <View style={styles.actions}>
          {actions.map((action) => (
            <Pressable
              key={action.label}
              onPress={action.onPress}
              style={({ pressed }) => [styles.action, pressed && styles.pressed]}
            >
              <SymbolView
                name={actionSymbols[action.icon]}
                tintColor={PlatformColor("systemBlue")}
                size={20}
              />
              <Text style={styles.actionLabel}>{action.label}</Text>
            </Pressable>
          ))}
        </View>
      )}
      {!last && <View style={styles.separator} />}
    </View>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1 },
  list: { paddingHorizontal: 16, paddingTop: 8, paddingBottom: 16 },
  usage: { color: PlatformColor("secondaryLabel"), fontSize: 15, fontVariant: ["tabular-nums"] },
  empty: { color: PlatformColor("secondaryLabel"), fontSize: 15, padding: 16 },
  row: { backgroundColor: PlatformColor("secondarySystemGroupedBackground") },
  first: { borderTopLeftRadius: 10, borderTopRightRadius: 10 },
  last: { borderBottomLeftRadius: 10, borderBottomRightRadius: 10 },
  body: { paddingHorizontal: 16, paddingVertical: 11, gap: 3 },
  pressed: { opacity: 0.5 },
  text: { color: PlatformColor("label"), fontSize: 17, lineHeight: 22 },
  pendingText: { color: PlatformColor("secondaryLabel") },
  meta: { color: PlatformColor("secondaryLabel"), fontSize: 13 },
  error: { color: PlatformColor("systemRed") },
  actions: { flexDirection: "row", paddingBottom: 8 },
  action: { flex: 1, alignItems: "center", gap: 4, paddingVertical: 6 },
  actionLabel: { color: PlatformColor("systemBlue"), fontSize: 11 },
  separator: {
    marginLeft: 16,
    height: StyleSheet.hairlineWidth,
    backgroundColor: PlatformColor("separator"),
  },
});
