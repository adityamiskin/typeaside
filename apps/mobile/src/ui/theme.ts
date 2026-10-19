import type { NoteColor } from "@typeaside/backend/convex/noteColors";
import type { NativeStackNavigationOptions } from "expo-router";
import { Platform } from "react-native";

export const colors = {
  background: "#000",
  text: "#fff",
  secondary: "rgba(242,242,241,0.58)",
  tertiary: "rgba(242,242,241,0.36)",
  separator: "#242528",
  fill: "#1c1c1e",
  elevated: "#121315",
  accent: "#8ab8e8",
  danger: "#e27a66",
  onPrimary: "#16171a",
} as const;

// Same dark tints and accents as the desktop notes (src/ui/theme.rs).
export const noteColors: Record<NoteColor, { background: string; accent: string }> = {
  none: { background: colors.elevated, accent: colors.tertiary },
  sky: { background: "#1c2533", accent: "#5f86b3" },
  lilac: { background: "#282133", accent: "#8a6aa3" },
  sage: { background: "#1c2a22", accent: "#5f9472" },
  sand: { background: "#2d281d", accent: "#a88d55" },
  rose: { background: "#2e1f22", accent: "#a86369" },
};

export const stackOptions: NativeStackNavigationOptions = {
  headerLargeTitle: true,
  headerTransparent: Platform.OS === "ios",
  headerBlurEffect: "systemChromeMaterialDark",
  headerLargeStyle: { backgroundColor: colors.background },
  headerStyle: { backgroundColor: Platform.OS === "ios" ? undefined : colors.background },
  headerTintColor: colors.text,
  headerShadowVisible: false,
  contentStyle: { backgroundColor: colors.background },
};
