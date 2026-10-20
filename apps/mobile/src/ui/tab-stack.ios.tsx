import { Stack, useRouter } from "expo-router";
import { SymbolView } from "expo-symbols";
import { Image } from "expo-image";
import { PlatformColor, Pressable, Text, useColorScheme, View } from "react-native";
import { useStackOptions } from "./theme";

export function TabStack() {
  const router = useRouter();
  const dark = useColorScheme() === "dark";
  return (
    <Stack
      screenOptions={{
        ...useStackOptions(),
        title: "",
        headerLeft: () => (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Account"
            onPress={() => router.push("/account")}
            style={{ padding: 8 }}
          >
            <SymbolView name="person.crop.circle" size={24} tintColor={dark ? "#fff" : "#000"} />
          </Pressable>
        ),
        headerLargeTitle: false,
        headerTitleAlign: "center",
        headerTitle: () => (
          <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
            <Image
              source={require("../../assets/icon.icon/Assets/mark.svg")}
              tintColor={dark ? "#fff" : "#000"}
              style={{ width: 24, height: 24 }}
            />
            <Text style={{ color: PlatformColor("label"), fontSize: 22 }}>TypeAside</Text>
          </View>
        ),
      }}
    />
  );
}
