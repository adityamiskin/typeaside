import { requireNativeView } from "expo";
import type { ViewProps } from "react-native";
import type { BubbleAppearance } from "../../modules/keyboard";

export const bubblePreviewPhases = [
  "idle",
  "recording",
  "sending",
  "paused",
  "copied",
  "error",
  "limit",
] as const;

export const BubblePreview = requireNativeView<
  ViewProps & { appearance: BubbleAppearance; phase: (typeof bubblePreviewPhases)[number] }
>("Keyboard", "BubblePreview");
