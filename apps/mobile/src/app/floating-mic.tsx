import {
  Host,
  LazyColumn,
  ListItem,
  RadioButton,
  Text,
  TextButton,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import { clickable, fillMaxSize, padding } from "@expo/ui/jetpack-compose/modifiers";
import { Stack } from "expo-router";
import { useEffect, useState } from "react";
import { Alert } from "react-native";
import { openKeyboardSettings, useKeyboardEnabled } from "@/account/keyboard";
import { seedColor, useStackOptions } from "@/ui/theme";
import { type BubbleAppearance, keyboard } from "../../modules/keyboard";

const defaults = { shape: "pill", size: "standard", position: 0.5 } satisfies BubbleAppearance;

export default function FloatingMicScreen() {
  const material = useMaterialColors({ seedColor });
  const options = useStackOptions();
  const enabled = useKeyboardEnabled();
  const [appearance, setAppearance] = useState<BubbleAppearance>();
  useEffect(() => {
    void keyboard
      .getAppearance()
      .then(setAppearance)
      .catch(() => {
        Alert.alert("Could not load floating mic settings.");
      });
  }, []);

  function update(change: Partial<BubbleAppearance>) {
    if (!appearance) return;
    const next = { ...appearance, ...change };
    setAppearance(next);
    void keyboard.setAppearance(next).catch(() => {
      setAppearance(appearance);
      Alert.alert("Could not save floating mic settings.");
    });
  }

  const heading = (text: string) => (
    <Text
      color={material.primary}
      style={{ typography: "titleSmall" }}
      modifiers={[padding(24, 24, 24, 8)]}
    >
      {text}
    </Text>
  );
  const choice = (label: string, selected: boolean, onClick: () => void) => (
    <ListItem key={label} modifiers={[clickable(onClick)]}>
      <ListItem.HeadlineContent>
        <Text>{label}</Text>
      </ListItem.HeadlineContent>
      <ListItem.TrailingContent>
        <RadioButton selected={selected} onClick={onClick} />
      </ListItem.TrailingContent>
    </ListItem>
  );

  return (
    <>
      <Stack.Screen options={{ ...options, headerShown: true, title: "Floating mic" }} />
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <LazyColumn modifiers={[fillMaxSize(), padding(0, 0, 0, 24)]}>
          <ListItem modifiers={[clickable(openKeyboardSettings)]}>
            <ListItem.HeadlineContent>
              <Text>Accessibility access</Text>
            </ListItem.HeadlineContent>
            <ListItem.SupportingContent>
              <Text>Dictate into other apps</Text>
            </ListItem.SupportingContent>
            <ListItem.TrailingContent>
              <Text>{enabled === undefined ? "" : enabled ? "On" : "Off"}</Text>
            </ListItem.TrailingContent>
          </ListItem>
          {appearance && (
            <>
              {heading("Shape")}
              {choice("Pill", appearance.shape === "pill", () => update({ shape: "pill" }))}
              {choice("Circle", appearance.shape === "circle", () => update({ shape: "circle" }))}
              {heading("Size")}
              {choice("Small", appearance.size === "small", () => update({ size: "small" }))}
              {choice("Standard", appearance.size === "standard", () =>
                update({ size: "standard" }),
              )}
              {choice("Large", appearance.size === "large", () => update({ size: "large" }))}
              {heading("Position")}
              {choice("Left", appearance.position < 0.33, () => update({ position: 0.18 }))}
              {choice("Center", appearance.position >= 0.33 && appearance.position <= 0.67, () =>
                update({ position: 0.5 }),
              )}
              {choice("Right", appearance.position > 0.67, () => update({ position: 0.82 }))}
              <Text
                color={material.onSurfaceVariant}
                style={{ typography: "bodySmall" }}
                modifiers={[padding(24, 8, 24, 16)]}
              >
                Drag horizontally for precise placement.
              </Text>
              <TextButton onClick={() => update(defaults)}>
                <Text>Reset to default</Text>
              </TextButton>
            </>
          )}
        </LazyColumn>
      </Host>
    </>
  );
}
