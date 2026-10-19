import { Image } from "expo-image";
import { useState } from "react";
import { Platform, Pressable, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { signIn } from "@/lib/session";
import { colors } from "@/ui/theme";

export default function SignInScreen() {
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
    <SafeAreaView style={styles.screen}>
      <View style={styles.hero}>
        <Image source={require("../../assets/splash.png")} style={styles.logo} />
        <Text style={styles.title}>TypeAside</Text>
        <Text style={styles.tagline}>Dictation and notes, synced with your desktop.</Text>
      </View>
      {error ? <Text style={styles.error}>{error}</Text> : null}
      <Pressable
        disabled={busy}
        onPress={() => void continueSignIn()}
        style={({ pressed }) => [styles.button, (pressed || busy) && { opacity: 0.7 }]}
      >
        <Text style={styles.buttonLabel}>Continue</Text>
      </Pressable>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1, backgroundColor: colors.background, paddingHorizontal: 24 },
  hero: { flex: 1, justifyContent: "center", gap: 12 },
  logo: { width: 56, height: 56 },
  title: { color: colors.text, fontSize: 32, fontWeight: "700", letterSpacing: -0.3 },
  tagline: { color: colors.secondary, fontSize: 16 },
  error: { color: colors.danger, fontSize: 14, marginBottom: 12 },
  button: {
    height: 52,
    borderRadius: Platform.OS === "ios" ? 14 : 26,
    backgroundColor: colors.text,
    justifyContent: "center",
    marginBottom: 16,
  },
  buttonLabel: { color: colors.onPrimary, fontSize: 17, fontWeight: "600", textAlign: "center" },
});
