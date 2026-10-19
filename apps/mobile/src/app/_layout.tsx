import { ConvexProviderWithAuth } from "convex/react";
import { DarkTheme, Stack, ThemeProvider } from "expo-router";
import * as SplashScreen from "expo-splash-screen";
import { useEffect } from "react";
import { convex } from "@/lib/convex";
import { fetchAccessToken, restoreSession, useSession } from "@/lib/session";

void SplashScreen.preventAutoHideAsync();
void restoreSession();

const theme = {
  ...DarkTheme,
  colors: { ...DarkTheme.colors, background: "#000", card: "#000", text: "#fff" },
};

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
  useEffect(() => {
    if (status !== "loading") void SplashScreen.hideAsync();
  }, [status]);
  if (status === "loading") return null;
  return (
    <ConvexProviderWithAuth client={convex} useAuth={useAuth}>
      <ThemeProvider value={theme}>
        <Stack screenOptions={{ headerShown: false }}>
          <Stack.Protected guard={status === "signedIn"}>
            <Stack.Screen name="(tabs)" />
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
