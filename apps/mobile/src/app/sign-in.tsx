import { Image } from "expo-image";
import { useState } from "react";
import { StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { signIn } from "@/lib/session";
import { PrimaryButton } from "@/ui/primary-button";
import { useColors } from "@/ui/theme";

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
          tintColor={colors.text}
        />
        <Text style={[styles.title, { color: colors.text }]}>TypeAside</Text>
        <Text style={[styles.tagline, { color: colors.secondary }]}>
          Dictation and notes, synced with your desktop.
        </Text>
      </View>
      {error ? <Text style={[styles.error, { color: colors.danger }]}>{error}</Text> : null}
      <View style={styles.button}>
        <PrimaryButton label="Continue" disabled={busy} onPress={() => void continueSignIn()} />
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1, paddingHorizontal: 24 },
  hero: { flex: 1, justifyContent: "center", gap: 12 },
  logo: { width: 56, height: 56 },
  title: { fontSize: 32, fontWeight: "700", letterSpacing: -0.3 },
  tagline: { fontSize: 16 },
  error: { fontSize: 14, marginBottom: 12 },
  button: { marginBottom: 16 },
});
