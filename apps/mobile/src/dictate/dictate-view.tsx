import {
  AssistChip,
  Column,
  FlowRow,
  Host,
  Icon,
  LazyColumn,
  ListItem,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import { background, clickable, fillMaxSize, padding } from "@expo/ui/jetpack-compose/modifiers";
import type { ReactNode } from "react";
import { StyleSheet, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { icons } from "@/ui/icons";
import { LargeTitle } from "@/ui/large-title";
import { seedColor, useColors } from "@/ui/theme";
import { type Action, takeMeta, takeText } from "./format";
import type { Take } from "./takes";

const actionIcons = {
  copy: icons.copy,
  share: icons.share,
  save: icons.noteAdd,
  saved: icons.check,
  delete: icons.delete,
  retry: icons.refresh,
} satisfies Record<Action["icon"], unknown>;

type Props = {
  takes: Take[];
  expandedId: string | undefined;
  onToggle: (take: Take) => void;
  actions: (take: Take) => Action[];
  usage: string | undefined;
  recordBar: ReactNode;
};

export function DictateView({ recordBar, ...list }: Props) {
  const insets = useSafeAreaInsets();
  const colors = useColors();
  return (
    <View style={[styles.screen, { backgroundColor: colors.background, paddingTop: insets.top }]}>
      <Host style={styles.screen} seedColor={seedColor}>
        <Takes {...list} />
      </Host>
      <View style={styles.bottom} pointerEvents="box-none">
        {recordBar}
      </View>
    </View>
  );
}

function Takes({ takes, expandedId, onToggle, actions, usage }: Omit<Props, "recordBar">) {
  const material = useMaterialColors();
  return (
    <LazyColumn modifiers={[fillMaxSize()]} contentPadding={{ bottom: 104 }}>
      <LargeTitle title="Dictate" supporting={usage && `${usage} this week`} />
      {takes.length === 0 && (
        <Text color={material.onSurfaceVariant} modifiers={[padding(16, 8, 16, 8)]}>
          Your dictations appear here.
        </Text>
      )}
      {takes.map((take) => {
        const expanded = take.id === expandedId;
        const container = expanded ? material.surfaceContainer : material.surface;
        return (
          <Column key={take.id} modifiers={[background(container)]}>
            <ListItem
              colors={{
                containerColor: container,
                contentColor: take.pending ? material.onSurfaceVariant : material.onSurface,
                supportingContentColor: take.pending?.error
                  ? material.error
                  : material.onSurfaceVariant,
              }}
              modifiers={[clickable(() => onToggle(take))]}
            >
              <ListItem.HeadlineContent>
                <Text maxLines={expanded ? undefined : 3} overflow="ellipsis">
                  {takeText(take)}
                </Text>
              </ListItem.HeadlineContent>
              <ListItem.SupportingContent>
                <Text>{takeMeta(take)}</Text>
              </ListItem.SupportingContent>
            </ListItem>
            {expanded && (
              <FlowRow horizontalArrangement={{ spacedBy: 8 }} modifiers={[padding(16, 0, 16, 12)]}>
                {actions(take).map((action) => (
                  <AssistChip key={action.label} onClick={action.onPress}>
                    <AssistChip.LeadingIcon>
                      <Icon source={actionIcons[action.icon]} size={18} />
                    </AssistChip.LeadingIcon>
                    <AssistChip.Label>
                      <Text>{action.label}</Text>
                    </AssistChip.Label>
                  </AssistChip>
                ))}
              </FlowRow>
            )}
          </Column>
        );
      })}
    </LazyColumn>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1 },
  bottom: { position: "absolute", left: 0, right: 0, bottom: 0 },
});
