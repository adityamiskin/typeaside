import {
  AssistChip,
  Column,
  Host,
  Icon,
  IconButton,
  LazyColumn,
  LinearProgressIndicator,
  ListItem,
  LoadingIndicator,
  Row,
  Spacer,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  background,
  clickable,
  clip,
  fillMaxSize,
  padding,
  Shapes,
  weight,
  width,
} from "@expo/ui/jetpack-compose/modifiers";
import { Stack } from "expo-router";
import type { ReactNode } from "react";
import { StyleSheet, View } from "react-native";
import { type Account, usageProgress } from "@/account/account";
import { icons } from "@/ui/icons";
import { seedColor } from "@/ui/theme";
import { type Action, groupByDay, takeMeta, takeText, usageText } from "./format";
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
  usage: Account["usage"] | undefined;
  recordBar: ReactNode;
};

export function DictateView({ recordBar, usage, ...list }: Props) {
  const material = useMaterialColors({ seedColor });
  return (
    <View style={[styles.screen, { backgroundColor: material.surface }]}>
      <Stack.Screen options={{ headerRight: () => usage && <Usage usage={usage} /> }} />
      {list.takes.length > 0 && (
        <Host style={styles.screen} seedColor={seedColor}>
          <Takes {...list} />
        </Host>
      )}
      <View style={StyleSheet.absoluteFill} pointerEvents="box-none">
        {recordBar}
      </View>
    </View>
  );
}

function Usage({ usage }: { usage: Account["usage"] }) {
  const material = useMaterialColors({ seedColor });
  const progress = usageProgress(usage);
  return (
    <Host matchContents seedColor={seedColor}>
      <Column horizontalAlignment="end" verticalArrangement={{ spacedBy: 6 }}>
        <Text
          color={progress === 1 ? material.error : material.onSurfaceVariant}
          style={{ typography: "labelLarge" }}
        >
          {usageText(usage)}
        </Text>
        {progress !== null && (
          <LinearProgressIndicator
            progress={progress}
            color={progress === 1 ? material.error : material.primary}
            modifiers={[width(112)]}
          />
        )}
      </Column>
    </Host>
  );
}

function Takes({ takes, expandedId, onToggle, actions }: Omit<Props, "recordBar" | "usage">) {
  const material = useMaterialColors();
  return (
    <LazyColumn modifiers={[fillMaxSize()]} contentPadding={{ bottom: 112 }}>
      {groupByDay(takes).flatMap((group) => [
        <Text
          key={group.label}
          color={material.primary}
          style={{ typography: "titleSmall" }}
          modifiers={[padding(16, 16, 16, 8)]}
        >
          {group.label}
        </Text>,
        ...group.takes.map((take) => (
          <TakeItem
            key={take.id}
            take={take}
            expanded={take.id === expandedId}
            onToggle={() => onToggle(take)}
            actions={actions(take)}
          />
        )),
      ])}
    </LazyColumn>
  );
}

function TakeItem({
  take,
  expanded,
  onToggle,
  actions,
}: {
  take: Take;
  expanded: boolean;
  onToggle: () => void;
  actions: Action[];
}) {
  const material = useMaterialColors();
  const container = expanded ? material.surfaceContainerHigh : material.surface;
  const chips = actions.filter((action) => action.icon !== "delete");
  const discard = actions.find((action) => action.icon === "delete");
  return (
    <Column
      modifiers={[padding(8, 0, 8, 0), clip(Shapes.RoundedCorner(20)), background(container)]}
    >
      <ListItem
        colors={{
          containerColor: container,
          contentColor: take.pending ? material.onSurfaceVariant : material.onSurface,
          supportingContentColor: take.pending?.error ? material.error : material.onSurfaceVariant,
        }}
        modifiers={[clickable(onToggle)]}
      >
        {take.pending && !take.pending.error && (
          <ListItem.LeadingContent>
            <LoadingIndicator modifiers={[width(32)]} />
          </ListItem.LeadingContent>
        )}
        <ListItem.HeadlineContent>
          <Text
            style={{ typography: "bodyLarge" }}
            maxLines={expanded ? undefined : 3}
            overflow="ellipsis"
          >
            {takeText(take)}
          </Text>
        </ListItem.HeadlineContent>
        <ListItem.SupportingContent>
          <Text style={{ typography: "bodyMedium" }}>{takeMeta(take)}</Text>
        </ListItem.SupportingContent>
      </ListItem>
      {expanded && actions.length > 0 && (
        <Row
          verticalAlignment="center"
          horizontalArrangement={{ spacedBy: 8 }}
          modifiers={[padding(16, 0, 4, 8)]}
        >
          {chips.map((action) => (
            <AssistChip key={action.label} onClick={action.onPress}>
              <AssistChip.LeadingIcon>
                <Icon source={actionIcons[action.icon]} size={18} />
              </AssistChip.LeadingIcon>
              <AssistChip.Label>
                <Text>{action.label}</Text>
              </AssistChip.Label>
            </AssistChip>
          ))}
          <Spacer modifiers={[weight(1)]} />
          {discard && (
            <IconButton onClick={discard.onPress}>
              <Icon source={icons.delete} size={24} tint={material.onSurfaceVariant} />
            </IconButton>
          )}
        </Row>
      )}
    </Column>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1 },
});
