import { Stack } from "expo-router";
import { useStackOptions } from "@/ui/theme";

export default function DictateLayout() {
  return (
    <Stack screenOptions={useStackOptions()}>
      <Stack.Screen name="index" options={{ title: "Dictate" }} />
    </Stack>
  );
}
