import { Button, ContextMenu, Host, List, Section, Text, VStack } from "@expo/ui/swift-ui";
import {
  buttonStyle,
  font,
  foregroundStyle,
  frame,
  lineLimit,
  multilineTextAlignment,
  onTapGesture,
  padding,
} from "@expo/ui/swift-ui/modifiers";
import { Stack } from "expo-router";
import { useEffect, useState } from "react";
import { PlatformColor, Text as NativeText, View } from "react-native";
import { groupByDay, takeMeta, takeText, usageText } from "./format";
import type { HistoryViewProps } from "./history-view";

export function HistoryView({ takes, onCopy, actions, usage, copied }: HistoryViewProps) {
  const [notice, setNotice] = useState(false);
  useEffect(() => {
    if (!copied) return;
    setNotice(true);
    const timer = setTimeout(() => setNotice(false), 2000);
    return () => clearTimeout(timer);
  }, [copied]);
  return (
    <View style={{ flex: 1 }}>
      <Stack.Screen
        options={{
          headerRight: () =>
            usage ? (
              <NativeText style={{ color: PlatformColor("secondaryLabel"), fontSize: 15 }}>
                {usageText(usage)}
              </NativeText>
            ) : null,
        }}
      />
      <Host style={{ flex: 1 }}>
        {takes.length === 0 ? (
          <VStack
            modifiers={[
              frame({ maxWidth: Infinity, maxHeight: Infinity }),
              padding({ horizontal: 32 }),
            ]}
          >
            <Text
              modifiers={[
                font({ textStyle: "subheadline" }),
                multilineTextAlignment("center"),
                foregroundStyle({ type: "hierarchical", style: "secondary" }),
              ]}
            >
              Dictate in any app with the TypeAside keyboard. Your dictations show up here.
            </Text>
          </VStack>
        ) : (
          <List>
            {groupByDay(takes).map((group) => (
              <Section key={group.label} title={group.label}>
                <List.ForEach
                  data={group.takes}
                  keyExtractor={(take) => take.id}
                  recycling={false}
                  onDelete={(indices) =>
                    indices.forEach((index) => {
                      const take = group.takes[index];
                      if (take)
                        actions(take)
                          .find((action) => action.icon === "delete")
                          ?.onPress();
                    })
                  }
                >
                  {({ item: take }) => (
                    <ContextMenu>
                      <ContextMenu.Trigger>
                        <VStack
                          alignment="leading"
                          spacing={4}
                          modifiers={[
                            frame({ maxWidth: Infinity, alignment: "leading" }),
                            onTapGesture(() => onCopy(take)),
                          ]}
                        >
                          <Text modifiers={[font({ textStyle: "body" }), lineLimit(3)]}>
                            {takeText(take)}
                          </Text>
                          <Text
                            modifiers={[
                              font({ textStyle: "caption" }),
                              foregroundStyle({ type: "hierarchical", style: "secondary" }),
                            ]}
                          >
                            {takeMeta(take)}
                          </Text>
                        </VStack>
                      </ContextMenu.Trigger>
                      <ContextMenu.Items>
                        {actions(take).map((action) => (
                          <Button
                            key={action.label}
                            label={action.label}
                            role={action.icon === "delete" ? "destructive" : undefined}
                            onPress={action.onPress}
                            modifiers={[buttonStyle("plain")]}
                          />
                        ))}
                      </ContextMenu.Items>
                    </ContextMenu>
                  )}
                </List.ForEach>
              </Section>
            ))}
          </List>
        )}
      </Host>
      {notice && (
        <NativeText
          accessibilityRole="alert"
          style={{
            position: "absolute",
            bottom: 24,
            alignSelf: "center",
            backgroundColor: PlatformColor("secondarySystemBackground"),
            color: PlatformColor("label"),
            padding: 16,
            borderRadius: 8,
          }}
        >
          Copied
        </NativeText>
      )}
    </View>
  );
}
