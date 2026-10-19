import { useState } from "react";
import { Pressable, Text, View } from "react-native";
import { signIn } from "@/lib/session";

export default function SignInScreen() {
  const [error, setError] = useState<string>();
  return (
    <View style={{ flex: 1, justifyContent: "center", padding: 24, backgroundColor: "#000" }}>
      <Pressable
        onPress={() => signIn().catch((cause: Error) => setError(cause.message))}
        style={{ height: 48, borderRadius: 12, backgroundColor: "#fff", justifyContent: "center" }}
      >
        <Text style={{ textAlign: "center", fontWeight: "600" }}>Continue</Text>
      </Pressable>
      {error ? <Text style={{ color: "#e27a66", marginTop: 12 }}>{error}</Text> : null}
    </View>
  );
}
