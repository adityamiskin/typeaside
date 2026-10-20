import { Redirect } from "expo-router";
import { useSession } from "@/lib/session";

export default function IndexScreen() {
  const status = useSession((session) => session.status);
  return <Redirect href={status === "signedIn" ? "/(app)/(tabs)/notes" : "/sign-in"} />;
}
