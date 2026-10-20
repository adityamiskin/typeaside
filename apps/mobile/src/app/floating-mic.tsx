import {
  Box,
  Column,
  Host,
  Icon,
  LazyColumn,
  ListItem,
  Row,
  SegmentedButton,
  SingleChoiceSegmentedButtonRow,
  Slider,
  Switch,
  Text,
  TextButton,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  alpha,
  background,
  clickable,
  clip,
  fillMaxSize,
  fillMaxWidth,
  height,
  padding,
  Shapes,
  size,
} from "@expo/ui/jetpack-compose/modifiers";
import { Stack } from "expo-router";
import { useEffect, useRef, useState } from "react";
import { Alert } from "react-native";
import { openKeyboardSettings, useKeyboardEnabled } from "@/account/keyboard";
import { icons } from "@/ui/icons";
import { seedColor, useStackOptions } from "@/ui/theme";
import { type BubbleAppearance, keyboard } from "../../modules/keyboard";

const defaults = {
  shape: "pill",
  size: 0.5,
  opacity: 1,
  shrinkWhenIdle: false,
  position: 0.5,
} satisfies BubbleAppearance;

export default function FloatingMicScreen() {
  const material = useMaterialColors({ seedColor });
  const options = useStackOptions();
  const enabled = useKeyboardEnabled();
  const [appearance, setAppearance] = useState<BubbleAppearance>(defaults);
  const latest = useRef<BubbleAppearance>(defaults);
  useEffect(() => {
    void keyboard
      .getAppearance()
      .then((stored) => {
        latest.current = stored;
        setAppearance(stored);
      })
      .catch(() => Alert.alert("Could not load floating mic settings."));
  }, []);

  function save() {
    void keyboard
      .setAppearance(latest.current)
      .catch(() => Alert.alert("Could not save floating mic settings."));
  }
  function update(change: Partial<BubbleAppearance>, persist = true) {
    latest.current = { ...latest.current, ...change };
    setAppearance(latest.current);
    if (persist) save();
  }
  const reset = () => update({ ...defaults, position: latest.current.position });
  const pillHeight = 32 + appearance.size * 24;
  const heading = (label: string, value?: string) => (
    <Row
      horizontalArrangement="spaceBetween"
      verticalAlignment="center"
      modifiers={[fillMaxWidth(), padding(24, 24, 24, 8)]}
    >
      <Text color={material.primary} style={{ typography: "titleSmall" }}>
        {label}
      </Text>
      {value && (
        <Text color={material.onSurfaceVariant} style={{ typography: "bodyMedium" }}>
          {value}
        </Text>
      )}
    </Row>
  );

  return (
    <>
      <Stack.Screen
        options={{
          ...options,
          headerShown: true,
          title: "Floating mic",
          headerRight: () => (
            <Host matchContents seedColor={seedColor}>
              <TextButton onClick={reset}>
                <Text>Reset</Text>
              </TextButton>
            </Host>
          ),
        }}
      />
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <LazyColumn modifiers={[fillMaxSize(), padding(0, 0, 0, 24)]}>
          <Box
            contentAlignment="center"
            modifiers={[
              padding(16, 16, 16, 0),
              fillMaxWidth(),
              height(160),
              clip(Shapes.RoundedCorner(28)),
              background(material.surfaceContainerHigh),
            ]}
          >
            <Column horizontalAlignment="center" verticalArrangement={{ spacedBy: 16 }}>
              <Box
                contentAlignment="center"
                modifiers={[
                  size(appearance.shape === "circle" ? pillHeight : pillHeight + 24, pillHeight),
                  alpha(appearance.opacity),
                  clip(Shapes.Circle),
                  background(material.secondaryContainer),
                ]}
              >
                <Icon source={icons.typeAside} size={24} tint={material.onSecondaryContainer} />
              </Box>
              <Text color={material.onSurfaceVariant} style={{ typography: "bodySmall" }}>
                Drag the mic to move it
              </Text>
            </Column>
          </Box>
          <ListItem modifiers={[clickable(openKeyboardSettings)]}>
            <ListItem.HeadlineContent>
              <Text>Show floating mic</Text>
            </ListItem.HeadlineContent>
            <ListItem.SupportingContent>
              <Text>Needs accessibility access</Text>
            </ListItem.SupportingContent>
            <ListItem.TrailingContent>
              <Switch value={enabled ?? false} onCheckedChange={openKeyboardSettings} />
            </ListItem.TrailingContent>
          </ListItem>
          {heading("Size")}
          <Slider
            value={appearance.size}
            onValueChange={(value) => update({ size: value }, false)}
            onValueChangeFinished={save}
            modifiers={[fillMaxWidth(), padding(24, 0, 24, 0)]}
          />
          <Row
            horizontalArrangement="spaceBetween"
            modifiers={[fillMaxWidth(), padding(24, 0, 24, 0)]}
          >
            <Text color={material.onSurfaceVariant} style={{ typography: "bodySmall" }}>
              Small
            </Text>
            <Text color={material.onSurfaceVariant} style={{ typography: "bodySmall" }}>
              Large
            </Text>
          </Row>
          {heading("Opacity", `${Math.round(appearance.opacity * 100)}%`)}
          <Slider
            value={appearance.opacity}
            min={0.2}
            max={1}
            steps={7}
            onValueChange={(value) => update({ opacity: Math.round(value * 10) / 10 }, false)}
            onValueChangeFinished={save}
            modifiers={[fillMaxWidth(), padding(24, 0, 24, 0)]}
          />
          {heading("Shape")}
          <SingleChoiceSegmentedButtonRow modifiers={[fillMaxWidth(), padding(24, 0, 24, 16)]}>
            <SegmentedButton
              selected={appearance.shape === "pill"}
              onClick={() => update({ shape: "pill" })}
            >
              <SegmentedButton.Label>
                <Text>Pill</Text>
              </SegmentedButton.Label>
            </SegmentedButton>
            <SegmentedButton
              selected={appearance.shape === "circle"}
              onClick={() => update({ shape: "circle" })}
            >
              <SegmentedButton.Label>
                <Text>Circle</Text>
              </SegmentedButton.Label>
            </SegmentedButton>
          </SingleChoiceSegmentedButtonRow>
          <ListItem
            modifiers={[clickable(() => update({ shrinkWhenIdle: !appearance.shrinkWhenIdle }))]}
          >
            <ListItem.HeadlineContent>
              <Text>Shrink when idle</Text>
            </ListItem.HeadlineContent>
            <ListItem.SupportingContent>
              <Text>Collapse to a small dot after 5 seconds</Text>
            </ListItem.SupportingContent>
            <ListItem.TrailingContent>
              <Switch
                value={appearance.shrinkWhenIdle}
                onCheckedChange={(value) => update({ shrinkWhenIdle: value })}
              />
            </ListItem.TrailingContent>
          </ListItem>
        </LazyColumn>
      </Host>
    </>
  );
}
