import { Stack } from "expo-router";
import { stackOptions } from "@/ui/theme";

export const unstable_settings = { anchor: "index" };

export default function NotesLayout() {
  return (
    <Stack screenOptions={stackOptions}>
      <Stack.Screen name="index" options={{ title: "Notes" }} />
      <Stack.Screen
        name="[id]"
        options={
          process.env.EXPO_OS === "ios"
            ? {
                presentation: "formSheet",
                sheetGrabberVisible: true,
                sheetAllowedDetents: [1],
                headerShown: false,
              }
            : { headerShown: false, animation: "fade_from_bottom" }
        }
      />
    </Stack>
  );
}
