import { useConvexAuth } from "convex/react";
import { useEffect, useState } from "react";
import { StyleSheet, Text, View } from "react-native";
import { connectKeyboard } from "@/lib/dictate";
import { colors } from "@/ui/theme";

export default function KeyboardSessionScreen() {
  const { isAuthenticated } = useConvexAuth();
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!isAuthenticated) return;
    void connectKeyboard().catch(() => setError(true));
  }, [isAuthenticated]);

  return (
    <View style={styles.screen}>
      <Text style={styles.text}>
        {error
          ? "Could not connect the keyboard. Reopen TypeAside to try again."
          : "Swipe back to your app to dictate."}
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  screen: {
    flex: 1,
    backgroundColor: colors.background,
    justifyContent: "center",
    padding: 24,
  },
  text: { color: colors.text, fontSize: 17, textAlign: "center" },
});
