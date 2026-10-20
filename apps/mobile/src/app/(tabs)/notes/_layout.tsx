import { Stack } from "expo-router";
import { useStackOptions } from "@/ui/theme";

export const unstable_settings = { anchor: "index" };

export default function NotesLayout() {
  return (
    <Stack screenOptions={useStackOptions()}>
      <Stack.Screen
        name="index"
        // On Android the search bar is the app bar, as in Keep.
        options={{ title: "Notes", headerShown: process.env.EXPO_OS === "ios" }}
      />
    </Stack>
  );
}
