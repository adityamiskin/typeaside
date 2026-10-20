import { Stack } from "expo-router";
import { useColorScheme } from "react-native";
import { useStackOptions } from "./theme";

export function TabStack() {
  const options = useStackOptions();
  const dark = useColorScheme() === "dark";
  return (
    <Stack
      screenOptions={{
        ...options,
        headerShown: false,
        contentStyle: dark ? { backgroundColor: "#000" } : options.contentStyle,
      }}
    />
  );
}
