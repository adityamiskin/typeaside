import { Stack } from "expo-router";
import { stackOptions } from "@/ui/theme";

export default function NotesLayout() {
  return (
    <Stack screenOptions={stackOptions}>
      <Stack.Screen name="index" options={{ title: "Notes" }} />
    </Stack>
  );
}
