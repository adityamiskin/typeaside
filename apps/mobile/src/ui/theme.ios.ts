import type { NoteColor } from "@typeaside/backend/convex/noteColors";
import { DarkTheme, DefaultTheme, type NativeStackNavigationOptions } from "expo-router";
import { DynamicColorIOS, PlatformColor, useColorScheme } from "react-native";
import type * as android from "./theme";

export const seedColor = undefined;

const colors = {
  background: PlatformColor("systemGroupedBackground"),
  surface: PlatformColor("secondarySystemGroupedBackground"),
  text: PlatformColor("label"),
  secondary: PlatformColor("secondaryLabel"),
  tertiary: PlatformColor("tertiaryLabel"),
  separator: PlatformColor("separator"),
  accent: PlatformColor("systemBlue"),
  onAccent: PlatformColor("white"),
  danger: PlatformColor("systemRed"),
} satisfies Record<keyof android.Colors, unknown>;

export function useColors() {
  return colors;
}

const tint = (light: string, dark: string, accent: string) => ({
  background: DynamicColorIOS({ light, dark }),
  text: colors.text,
  accent: PlatformColor(accent),
});

const noteColors = {
  none: { background: colors.surface, text: colors.text, accent: colors.tertiary },
  sky: tint("#e3edf8", "#1c2533", "systemBlue"),
  lilac: tint("#eee6f6", "#282133", "systemPurple"),
  sage: tint("#e2f1e7", "#1c2a22", "systemGreen"),
  sand: tint("#f6eedc", "#2d281d", "systemOrange"),
  rose: tint("#f8e4e6", "#2e1f22", "systemPink"),
} satisfies Record<NoteColor, unknown>;

export function useNoteColors() {
  return noteColors;
}

export function useNavigationTheme() {
  return useColorScheme() === "dark" ? DarkTheme : DefaultTheme;
}

const stackOptions: NativeStackNavigationOptions = {
  headerLargeTitle: true,
  headerTransparent: true,
  headerBlurEffect: "systemChromeMaterial",
  headerShadowVisible: false,
  contentStyle: { backgroundColor: colors.background },
};

export function useStackOptions() {
  return stackOptions;
}

export function useTabsAppearance() {
  return {};
}
