import {
  getMaterialColors,
  isDynamicColorAvailable,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import type { NoteColor } from "@typeaside/backend/convex/noteColors";
import { DarkTheme, DefaultTheme, type NativeStackNavigationOptions } from "expo-router";
import type { NativeTabsProps } from "expo-router/native-tabs";
import { useColorScheme } from "react-native";

// Desktop accent, used as the Material seed only where the wallpaper palette is unavailable.
export const seedColor = isDynamicColorAvailable ? undefined : "#8ab8e8";

export function useColors() {
  const material = useMaterialColors({ seedColor });
  return {
    background: material.surface,
    surface: material.surfaceContainer,
    text: material.onSurface,
    secondary: material.onSurfaceVariant,
    tertiary: material.outline,
    separator: material.outlineVariant,
    accent: material.primary,
    onAccent: material.onPrimary,
    danger: material.error,
  };
}

export type Colors = ReturnType<typeof useColors>;

// Desktop note hues seed their own tonal palettes so tints sit beside the wallpaper colors.
const noteSeeds = {
  sky: "#5f86b3",
  lilac: "#8a6aa3",
  sage: "#5f9472",
  sand: "#a88d55",
  rose: "#a86369",
} as const satisfies Record<Exclude<NoteColor, "none">, string>;

export function useNoteColors(): Record<
  NoteColor,
  { background: string; text: string; accent: string }
> {
  const scheme = useColorScheme() === "dark" ? "dark" : "light";
  const material = useMaterialColors({ seedColor });
  const tint = (seed: string) => {
    const palette = getMaterialColors({ scheme, seedColor: seed });
    return {
      background: palette.primaryContainer,
      text: palette.onPrimaryContainer,
      accent: palette.primary,
    };
  };
  return {
    none: {
      background: material.surfaceContainerHigh,
      text: material.onSurface,
      accent: material.outline,
    },
    sky: tint(noteSeeds.sky),
    lilac: tint(noteSeeds.lilac),
    sage: tint(noteSeeds.sage),
    sand: tint(noteSeeds.sand),
    rose: tint(noteSeeds.rose),
  };
}

export function useNavigationTheme() {
  const dark = useColorScheme() === "dark";
  const material = useMaterialColors({ seedColor });
  const base = dark ? DarkTheme : DefaultTheme;
  return {
    ...base,
    colors: {
      ...base.colors,
      primary: material.primary,
      background: material.surface,
      card: material.surface,
      text: material.onSurface,
      border: material.outlineVariant,
    },
  };
}

// Tab roots draw their own Material large title inside Compose.
export function useStackOptions(): NativeStackNavigationOptions {
  const { background } = useColors();
  return { headerShown: false, contentStyle: { backgroundColor: background } };
}

export function useTabsAppearance() {
  const material = useMaterialColors({ seedColor });
  return {
    backgroundColor: material.surfaceContainer,
    indicatorColor: material.secondaryContainer,
    iconColor: { default: material.onSurfaceVariant, selected: material.onSecondaryContainer },
    labelStyle: {
      default: { color: material.onSurfaceVariant },
      selected: { color: material.onSurface },
    },
  } satisfies Partial<NativeTabsProps>;
}
