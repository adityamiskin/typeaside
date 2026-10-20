import { Stack } from "expo-router";
import { useStackOptions } from "@/ui/theme";

export default function AccountLayout() {
  return (
    <Stack screenOptions={useStackOptions()}>
      <Stack.Screen name="index" options={{ title: "Account" }} />
    </Stack>
  );
}
