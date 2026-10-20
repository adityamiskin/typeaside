import {
  Box,
  Host,
  Icon,
  IconButton,
  Row,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import { align, fillMaxSize, padding } from "@expo/ui/jetpack-compose/modifiers";
import { useColorScheme, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { icons } from "./icons";
import { seedColor } from "./theme";

export function TabHeader({ onMenu }: { onMenu: () => void }) {
  const material = useMaterialColors({ seedColor });
  const insets = useSafeAreaInsets();
  const dark = useColorScheme() === "dark";
  return (
    <View
      style={{
        height: insets.top + 64,
        paddingTop: insets.top,
        backgroundColor: dark ? "#000" : material.surface,
      }}
    >
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <Box contentAlignment="center" modifiers={[fillMaxSize(), padding(4, 0, 4, 0)]}>
          <IconButton onClick={onMenu} modifiers={[align("centerStart")]}>
            <Icon source={icons.menu} size={24} contentDescription="Open menu" />
          </IconButton>
          <Row verticalAlignment="center" horizontalArrangement={{ spacedBy: 8 }}>
            <Icon source={icons.typeAside} size={24} tint={material.onSurface} />
            <Text style={{ typography: "titleLarge" }}>TypeAside</Text>
          </Row>
        </Box>
      </Host>
    </View>
  );
}
