import { Stack } from "expo-router";
import { useStackOptions } from "@/ui/theme";

export default function SnippetsLayout() {
  return (
    <Stack screenOptions={useStackOptions()}>
      <Stack.Screen name="index" options={{ title: "Snippets", headerLargeTitle: true }} />
    </Stack>
  );
}
