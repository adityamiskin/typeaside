import {
  Box,
  Column,
  DropdownMenu,
  DropdownMenuItem,
  Host,
  Icon,
  IconButton,
  LazyColumn,
  ListItem,
  SnackbarHost,
  type SnackbarHostRef,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  align,
  combinedClickable,
  fillMaxSize,
  fillMaxWidth,
  padding,
  weight,
} from "@expo/ui/jetpack-compose/modifiers";
import { useEffect, useRef, useState } from "react";
import { useColorScheme, View } from "react-native";
import { icons } from "@/ui/icons";
import { seedColor } from "@/ui/theme";
import { type Action, groupByDay, takeMeta } from "./format";
import type { Take } from "./takes";

export type HistoryViewProps = {
  takes: Take[];
  onCopy: (take: Take) => void;
  actions: (take: Take) => Action[];
  copied: number;
};

export function HistoryView({ takes, onCopy, actions, copied }: HistoryViewProps) {
  const material = useMaterialColors({ seedColor });
  const dark = useColorScheme() === "dark";
  const snackbar = useRef<SnackbarHostRef>(null);
  useEffect(() => {
    if (copied) void snackbar.current?.showSnackbar({ message: "Copied", duration: "short" });
  }, [copied]);
  return (
    <View style={{ flex: 1, backgroundColor: dark ? "#000" : material.surface }}>
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <Box modifiers={[fillMaxSize()]}>
          <Column modifiers={[fillMaxSize()]}>
            {takes.length === 0 ? (
              <Box
                contentAlignment="center"
                modifiers={[weight(1), fillMaxWidth(), padding(32, 32, 32, 32)]}
              >
                <Text
                  color={material.onSurfaceVariant}
                  style={{ typography: "bodyMedium", textAlign: "center" }}
                >
                  Dictate in any app with the floating mic. Your dictations show up here.
                </Text>
              </Box>
            ) : (
              <LazyColumn modifiers={[weight(1), fillMaxWidth()]} contentPadding={{ bottom: 24 }}>
                {groupByDay(takes).flatMap((group) => [
                  <Text
                    key={group.label}
                    color={material.onSurfaceVariant}
                    style={{ typography: "titleSmall" }}
                    modifiers={[padding(16, 16, 16, 8)]}
                  >
                    {group.label}
                  </Text>,
                  ...group.takes.map((take) => (
                    <TakeItem
                      key={take.id}
                      take={take}
                      onCopy={() => onCopy(take)}
                      actions={actions(take)}
                    />
                  )),
                ])}
              </LazyColumn>
            )}
          </Column>
          <SnackbarHost
            ref={snackbar}
            modifiers={[align("bottomCenter"), padding(16, 16, 16, 16)]}
          />
        </Box>
      </Host>
    </View>
  );
}

function TakeItem({
  take,
  onCopy,
  actions,
}: {
  take: Take;
  onCopy: () => void;
  actions: Action[];
}) {
  const material = useMaterialColors();
  const [menu, setMenu] = useState(false);
  const dark = useColorScheme() === "dark";
  return (
    <ListItem
      colors={{
        containerColor: dark ? "#000" : material.surface,
        contentColor: material.onSurface,
        supportingContentColor: material.onSurfaceVariant,
      }}
      modifiers={[combinedClickable({ onClick: onCopy, onLongClick: () => setMenu(true) })]}
    >
      <ListItem.HeadlineContent>
        <Text style={{ typography: "bodyLarge" }} maxLines={3} overflow="ellipsis">
          {take.text}
        </Text>
      </ListItem.HeadlineContent>
      <ListItem.SupportingContent>
        <Text style={{ typography: "bodyMedium" }}>{takeMeta(take)}</Text>
      </ListItem.SupportingContent>
      <ListItem.TrailingContent>
        <DropdownMenu expanded={menu} onDismissRequest={() => setMenu(false)}>
          <DropdownMenu.Trigger>
            <IconButton onClick={() => setMenu(true)}>
              <Icon source={icons.more} size={24} contentDescription="Dictation actions" />
            </IconButton>
          </DropdownMenu.Trigger>
          <DropdownMenu.Items>
            {actions.map((action) => (
              <DropdownMenuItem
                key={action.label}
                onClick={() => {
                  setMenu(false);
                  action.onPress();
                }}
              >
                <DropdownMenuItem.Text>
                  <Text>{action.label}</Text>
                </DropdownMenuItem.Text>
              </DropdownMenuItem>
            ))}
          </DropdownMenu.Items>
        </DropdownMenu>
      </ListItem.TrailingContent>
    </ListItem>
  );
}
