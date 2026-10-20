import { Image } from "expo-image";
import * as WebBrowser from "expo-web-browser";
import { useState } from "react";
import { Platform, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { signIn } from "@/lib/session";
import { PrimaryButton } from "@/ui/primary-button";
import { useColors } from "@/ui/theme";

const openLegal = (page: "terms" | "privacy") =>
  void WebBrowser.openBrowserAsync(`https://typeaside.app/${page}`);

export default function SignInScreen() {
  const colors = useColors();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function continueSignIn() {
    setBusy(true);
    setError(undefined);
    await signIn()
      .catch((cause: Error) => setError(cause.message))
      .finally(() => setBusy(false));
  }

  return (
    <SafeAreaView style={[styles.screen, { backgroundColor: colors.background }]}>
      <View style={styles.hero}>
        <Image
          source={require("../../assets/splash.png")}
          style={styles.logo}
          tintColor={colors.accent}
        />
        <Text style={[styles.title, { color: colors.text }]}>TypeAside</Text>
        <Text style={[styles.tagline, { color: colors.secondary }]}>
          Talk, get clean text. Notes sync with your desktop.
        </Text>
      </View>
      {error ? <Text style={[styles.error, { color: colors.danger }]}>{error}</Text> : null}
      <PrimaryButton label="Continue" disabled={busy} onPress={() => void continueSignIn()} />
      <Text style={[styles.legal, { color: colors.secondary }]}>
        By continuing, you agree to the{" "}
        <Text style={{ color: colors.accent }} onPress={() => openLegal("terms")}>
          Terms
        </Text>{" "}
        and{" "}
        <Text style={{ color: colors.accent }} onPress={() => openLegal("privacy")}>
          Privacy Policy
        </Text>
        .
      </Text>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1, paddingHorizontal: 24, paddingBottom: 16 },
  hero: { flex: 1, paddingTop: 96, gap: 16 },
  logo: { width: 64, height: 64, marginBottom: 8 },
  title: Platform.select({
    ios: { fontSize: 34, fontWeight: "700", letterSpacing: 0.4 },
    default: { fontSize: 36, lineHeight: 44, fontWeight: "400" },
  }),
  tagline: { fontSize: 16, lineHeight: 24 },
  error: { fontSize: 14, marginBottom: 12 },
  legal: { fontSize: 12, lineHeight: 16, textAlign: "center", marginTop: 16 },
});
