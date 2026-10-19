import { Redirect } from "expo-router";
import { useSession } from "@/lib/session";

export default function IndexScreen() {
  const status = useSession((session) => session.status);
  return <Redirect href={status === "signedIn" ? "/(tabs)/dictate" : "/sign-in"} />;
}
