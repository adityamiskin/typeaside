import { Stack } from "expo-router";
import { useStackOptions } from "@/ui/theme";

export default function DictionaryLayout() {
  return (
    <Stack screenOptions={useStackOptions()}>
      <Stack.Screen name="index" options={{ title: "Dictionary", headerLargeTitle: true }} />
    </Stack>
  );
}
