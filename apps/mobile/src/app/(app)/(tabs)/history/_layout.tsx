import { Stack } from "expo-router";
import { useStackOptions } from "@/ui/theme";

export default function HistoryLayout() {
  return (
    <Stack screenOptions={useStackOptions()}>
      <Stack.Screen
        name="index"
        options={{ title: "History", headerShown: process.env.EXPO_OS === "ios" }}
      />
    </Stack>
  );
}
