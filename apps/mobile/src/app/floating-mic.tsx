import {
  Box,
  Column,
  Host,
  LazyColumn,
  ListItem,
  Row,
  RNHostView,
  SegmentedButton,
  SingleChoiceSegmentedButtonRow,
  Slider,
  Switch,
  Text,
  TextButton,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  background,
  clickable,
  clip,
  fillMaxSize,
  fillMaxWidth,
  height,
  padding,
  Shapes,
} from "@expo/ui/jetpack-compose/modifiers";
import { Stack } from "expo-router";
import { useEffect, useRef, useState } from "react";
import { Alert } from "react-native";
import { openKeyboardSettings, useKeyboardEnabled } from "@/account/keyboard";
import { BubblePreview, bubblePreviewPhases } from "@/ui/bubble-preview";
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
  const [preview, setPreview] = useState<(typeof bubblePreviewPhases)[number]>("idle");
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

  const heading = (label: string, value?: string) => (
    <Row
      horizontalArrangement="spaceBetween"
      verticalAlignment="center"
      modifiers={[fillMaxWidth(), padding(32, 24, 32, 8)]}
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
              height(preview === "limit" ? 304 : 240),
              clip(Shapes.RoundedCorner(28)),
              background(material.surfaceContainerLow),
            ]}
          >
            <Column horizontalAlignment="center" verticalArrangement={{ spacedBy: 4 }}>
              <RNHostView modifiers={[fillMaxWidth(), height(preview === "limit" ? 184 : 120)]}>
                <BubblePreview phase={preview} appearance={appearance} style={{ flex: 1 }} />
              </RNHostView>
              <Text color={material.onSurfaceVariant} style={{ typography: "bodySmall" }}>
                Drag the mic to move it
              </Text>
              <TextButton
                onClick={() =>
                  setPreview(
                    bubblePreviewPhases[
                      (bubblePreviewPhases.indexOf(preview) + 1) % bubblePreviewPhases.length
                    ] ?? "idle",
                  )
                }
              >
                <Text>{preview === "idle" ? "Preview recording" : `Preview: ${preview}`}</Text>
              </TextButton>
            </Column>
          </Box>
          {heading("Access")}
          <ListItem
            colors={{ containerColor: material.surfaceContainer }}
            modifiers={[
              padding(16, 0, 16, 0),
              clip(Shapes.RoundedCorner(24)),
              clickable(openKeyboardSettings),
            ]}
          >
            <ListItem.HeadlineContent>
              <Text>Show floating mic</Text>
            </ListItem.HeadlineContent>
            <ListItem.SupportingContent>
              <Text>
                {enabled ? "Appears when a text field is focused" : "Needs accessibility access"}
              </Text>
            </ListItem.SupportingContent>
            <ListItem.TrailingContent>
              <Switch value={enabled ?? false} onCheckedChange={openKeyboardSettings} />
            </ListItem.TrailingContent>
          </ListItem>
          {heading("Appearance")}
          <Column
            modifiers={[
              padding(16, 0, 16, 0),
              fillMaxWidth(),
              clip(Shapes.RoundedCorner(24)),
              background(material.surfaceContainer),
              padding(0, 0, 0, 8),
            ]}
          >
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
          </Column>
          {heading("Behavior")}
          <ListItem
            colors={{ containerColor: material.surfaceContainer }}
            modifiers={[
              padding(16, 0, 16, 0),
              clip(Shapes.RoundedCorner(24)),
              clickable(() => update({ shrinkWhenIdle: !appearance.shrinkWhenIdle })),
            ]}
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
