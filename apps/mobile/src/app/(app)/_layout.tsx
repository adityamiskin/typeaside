import {
  Column,
  Host,
  Icon,
  LinearProgressIndicator,
  Row,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  background,
  clickable,
  clip,
  fillMaxWidth,
  height,
  padding,
  Shapes,
} from "@expo/ui/jetpack-compose/modifiers";
import { api } from "@typeaside/backend/convex/_generated/api";
import { useQuery } from "convex/react";
import { useRouter } from "expo-router";
import { Drawer } from "expo-router/drawer";
import { useWindowDimensions } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { usageProgress } from "@/account/account";
import { usageText } from "@/dictate/format";
import { icons } from "@/ui/icons";
import { TabHeader } from "@/ui/tab-header";
import { seedColor } from "@/ui/theme";

export default function AppLayout() {
  const material = useMaterialColors({ seedColor });
  const { width } = useWindowDimensions();
  return (
    <Drawer
      screenOptions={{
        headerShown: true,
        header: ({ navigation }) => <TabHeader onMenu={() => navigation.openDrawer()} />,
        drawerType: "front",
        swipeEnabled: true,
        swipeEdgeWidth: 24,
        overlayColor: "#00000066",
        drawerStyle: {
          width: Math.min(360, width - 56),
          backgroundColor: material.surfaceContainerLow,
          borderTopRightRadius: 16,
          borderBottomRightRadius: 16,
          overflow: "hidden",
        },
      }}
      drawerContent={({ navigation }) => (
        <NavigationDrawer onClose={() => navigation.closeDrawer()} />
      )}
    >
      <Drawer.Screen name="(tabs)" />
    </Drawer>
  );
}

function NavigationDrawer({ onClose }: { onClose: () => void }) {
  const material = useMaterialColors({ seedColor });
  const account = useQuery(api.viewer.current);
  const insets = useSafeAreaInsets();
  const router = useRouter();
  const progress = account && usageProgress(account.usage);
  return (
    <Host style={{ flex: 1 }} seedColor={seedColor}>
      <Column modifiers={[padding(0, insets.top, 0, insets.bottom)]}>
        <Column verticalArrangement={{ spacedBy: 8 }} modifiers={[padding(28, 24, 28, 24)]}>
          <Text style={{ typography: "titleLarge" }}>TypeAside</Text>
          {account && (
            <Text color={material.onSurfaceVariant} style={{ typography: "bodyMedium" }}>
              {account.email}
            </Text>
          )}
          {account && (
            <Text color={material.onSurfaceVariant} style={{ typography: "bodyMedium" }}>
              {usageText(account.usage)}
            </Text>
          )}
          {progress != null && (
            <LinearProgressIndicator progress={progress} modifiers={[fillMaxWidth()]} />
          )}
        </Column>
        {(
          [
            { label: "Account", icon: icons.account, href: "/account" },
            { label: "Floating mic", icon: icons.mic, href: "/floating-mic" },
          ] as const
        ).map(({ label, icon, href }) => (
          <Row
            key={label}
            verticalAlignment="center"
            horizontalArrangement={{ spacedBy: 12 }}
            modifiers={[
              padding(12, 0, 12, 0),
              fillMaxWidth(),
              height(56),
              clip(Shapes.Circle),
              background(material.surfaceContainerLow),
              clickable(() => {
                onClose();
                router.push(href);
              }),
              padding(16, 0, 16, 0),
            ]}
          >
            <Icon source={icon} size={24} tint={material.onSurfaceVariant} />
            <Text style={{ typography: "labelLarge" }}>{label}</Text>
          </Row>
        ))}
      </Column>
    </Host>
  );
}
