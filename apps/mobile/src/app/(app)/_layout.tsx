import {
  Box,
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
  size,
  weight,
  fillMaxSize,
  Shapes,
} from "@expo/ui/jetpack-compose/modifiers";
import * as WebBrowser from "expo-web-browser";
import { useRouter } from "expo-router";
import { Drawer } from "expo-router/drawer";
import { useWindowDimensions } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { initial, resetsLabel, useAccount, usageProgress } from "@/account/account";
import { keyboardStatus, useKeyboardEnabled } from "@/account/keyboard";
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
  const { account, openBilling } = useAccount();
  const enabled = useKeyboardEnabled();
  const insets = useSafeAreaInsets();
  const router = useRouter();
  const progress = account && usageProgress(account.usage);
  return (
    <Host style={{ flex: 1 }} seedColor={seedColor}>
      <Column modifiers={[fillMaxSize(), padding(0, insets.top, 0, insets.bottom + 12)]}>
        {account && (
          <Column horizontalAlignment="center" modifiers={[fillMaxWidth(), padding(28, 32, 28, 0)]}>
            <Box
              contentAlignment="center"
              modifiers={[size(72, 72), clip(Shapes.Circle), background(material.primaryContainer)]}
            >
              <Text color={material.onPrimaryContainer} style={{ typography: "headlineLarge" }}>
                {initial(account)}
              </Text>
            </Box>
            <Text style={{ typography: "titleLarge" }} modifiers={[padding(0, 16, 0, 4)]}>
              {account.name || "Your account"}
            </Text>
            <Text color={material.onSurfaceVariant} style={{ typography: "bodyMedium" }}>
              {account.email}
            </Text>
            <Text
              color={material.primary}
              style={{ typography: "labelLarge" }}
              modifiers={[padding(0, 12, 0, 0)]}
            >
              {account.plan === "pro" ? "Pro" : "Free plan"}
            </Text>
          </Column>
        )}
        {account && (
          <Column
            verticalArrangement={{ spacedBy: 12 }}
            modifiers={[padding(28, 36, 28, 0), fillMaxWidth()]}
          >
            <Text style={{ typography: "headlineSmall" }}>
              {account.usage.cap === null
                ? `${account.usage.words.toLocaleString()} words spoken`
                : `${Math.max(0, account.usage.cap - account.usage.words).toLocaleString()} words left`}
            </Text>
            {progress != null && (
              <LinearProgressIndicator
                progress={progress}
                modifiers={[fillMaxWidth(), height(8)]}
              />
            )}
            <Text color={material.onSurfaceVariant} style={{ typography: "bodyMedium" }}>
              {resetsLabel(account.usage)}
            </Text>
          </Column>
        )}
        <Box modifiers={[weight(1)]} />
        {[
          {
            label: "Floating mic",
            icon: icons.mic,
            trailing: keyboardStatus(enabled),
            action: () => router.push("/floating-mic"),
          },
          {
            label: "Share feedback",
            icon: icons.share,
            action: () => void WebBrowser.openBrowserAsync("https://typeaside.app/feedback"),
          },
          { label: "Account", icon: icons.account, action: () => router.push("/account") },
          ...(account?.plan === "free"
            ? [{ label: "Upgrade to Pro", icon: icons.premium, action: openBilling }]
            : []),
        ].map(({ label, icon, action, ...item }) => (
          <Row
            key={label}
            verticalAlignment="center"
            horizontalArrangement={{ spacedBy: 12 }}
            modifiers={[
              padding(12, 0, 12, 0),
              fillMaxWidth(),
              height(56),
              clip(Shapes.Circle),
              clickable(() => {
                onClose();
                action();
              }),
              padding(16, 0, 16, 0),
            ]}
          >
            <Icon source={icon} size={24} tint={material.onSurfaceVariant} />
            <Text style={{ typography: "labelLarge" }} modifiers={[weight(1)]}>
              {label}
            </Text>
            {"trailing" in item && (
              <Text color={material.onSurfaceVariant} style={{ typography: "labelMedium" }}>
                {item.trailing}
              </Text>
            )}
          </Row>
        ))}
      </Column>
    </Host>
  );
}
