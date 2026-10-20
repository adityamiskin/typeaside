import { ConvexProviderWithAuth } from "convex/react";
import { Stack, ThemeProvider } from "expo-router";
import * as SplashScreen from "expo-splash-screen";
import { StatusBar } from "expo-status-bar";
import { useEffect } from "react";
import { WidgetSync } from "@/lib/widget-sync";
import { convex } from "@/lib/convex";
import { fetchAccessToken, restoreSession, useSession } from "@/lib/session";
import { useNavigationTheme } from "@/ui/theme";

export const unstable_settings = { anchor: "(tabs)" };

void SplashScreen.preventAutoHideAsync();
void restoreSession();

function useAuth() {
  const status = useSession((session) => session.status);
  return {
    isLoading: status === "loading",
    isAuthenticated: status === "signedIn",
    fetchAccessToken,
  };
}

export default function RootLayout() {
  const status = useSession((session) => session.status);
  const theme = useNavigationTheme();
  useEffect(() => {
    if (status !== "loading") void SplashScreen.hideAsync();
  }, [status]);
  if (status === "loading") return null;
  return (
    <ConvexProviderWithAuth client={convex} useAuth={useAuth}>
      <WidgetSync />
      <ThemeProvider value={theme}>
        <StatusBar style="auto" />
        <Stack screenOptions={{ headerShown: false }}>
          <Stack.Protected guard={status === "signedIn"}>
            <Stack.Screen name="(tabs)" />
            <Stack.Screen
              name="note/[id]"
              options={
                process.env.EXPO_OS === "ios"
                  ? {
                      presentation: "formSheet",
                      sheetGrabberVisible: true,
                      sheetAllowedDetents: [1],
                    }
                  : { animation: "fade_from_bottom" }
              }
            />
            <Stack.Screen name="floating-mic" />
            <Stack.Screen name="keyboard-session" />
          </Stack.Protected>
          <Stack.Protected guard={status === "signedOut"}>
            <Stack.Screen name="sign-in" />
          </Stack.Protected>
        </Stack>
      </ThemeProvider>
    </ConvexProviderWithAuth>
  );
}
