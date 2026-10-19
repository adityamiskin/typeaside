import * as Crypto from "expo-crypto";
import * as Linking from "expo-linking";
import * as SecureStore from "expo-secure-store";
import * as WebBrowser from "expo-web-browser";
import { Platform } from "react-native";
import { z } from "zod";
import { create } from "zustand";
import { env } from "./env";

const WORKOS = "https://api.workos.com/user_management";
const REDIRECT_URI = "app.typeaside://callback";
const SIGNED_OUT_URI = "app.typeaside://signed-out";
const STORED_KEY = "workos.session";

const user = z.object({ email: z.string(), name: z.string() });
export type User = z.infer<typeof user>;
const stored = z.object({ refreshToken: z.string(), user });
const authResponse = z.object({
  access_token: z.string(),
  refresh_token: z.string(),
  user: z.object({
    email: z.string(),
    first_name: z.string().nullable(),
    last_name: z.string().nullable(),
  }),
});
const tokenClaims = z.object({ exp: z.number(), sid: z.string() });

export const useSession = create<
  { status: "loading" } | { status: "signedOut" } | { status: "signedIn"; user: User }
>(() => ({ status: "loading" }));

class Rejected extends Error {}

let access: { token: string; expiresAt: number; sessionId: string } | null = null;
let refreshing: Promise<string | null> | null = null;

const base64url = (base64: string) =>
  base64.replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
const randomString = (bytes: number) =>
  base64url(btoa(String.fromCharCode(...Crypto.getRandomBytes(bytes))));
const query = (params: Record<string, string>) =>
  Object.entries(params)
    .map(([key, value]) => `${key}=${encodeURIComponent(value)}`)
    .join("&");

async function authenticate(body: Record<string, string>) {
  const response = await fetch(`${WORKOS}/authenticate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ client_id: env.workosClientId, ...body }),
  });
  if (response.status >= 400 && response.status < 500)
    throw new Rejected(`Sign-in was rejected (${response.status})`);
  if (!response.ok) throw new Error(`Sign-in failed (${response.status})`);
  const data = authResponse.parse(await response.json());
  const claims = tokenClaims.parse(JSON.parse(atob(data.access_token.split(".")[1] ?? "")));
  const signedIn = {
    email: data.user.email,
    name: [data.user.first_name, data.user.last_name].filter(Boolean).join(" "),
  };
  await SecureStore.setItemAsync(
    STORED_KEY,
    JSON.stringify({ refreshToken: data.refresh_token, user: signedIn }),
  );
  access = { token: data.access_token, expiresAt: claims.exp * 1000, sessionId: claims.sid };
  useSession.setState({ status: "signedIn", user: signedIn });
  return data.access_token;
}

async function readStored() {
  const raw = await SecureStore.getItemAsync(STORED_KEY);
  const parsed = raw ? stored.safeParse(JSON.parse(raw)) : null;
  return parsed?.success ? parsed.data : null;
}

async function refresh() {
  const session = await readStored();
  if (!session) {
    useSession.setState({ status: "signedOut" });
    return null;
  }
  try {
    return await authenticate({ grant_type: "refresh_token", refresh_token: session.refreshToken });
  } catch (error) {
    if (error instanceof Rejected) await clear();
    return null;
  }
}

async function clear() {
  access = null;
  await SecureStore.deleteItemAsync(STORED_KEY);
  useSession.setState({ status: "signedOut" });
}

// Show the cached account immediately; an offline launch should not look signed out.
export async function restoreSession() {
  const session = await readStored();
  useSession.setState(
    session ? { status: "signedIn", user: session.user } : { status: "signedOut" },
  );
}

export function fetchAccessToken({ forceRefreshToken }: { forceRefreshToken: boolean }) {
  if (!forceRefreshToken && access && access.expiresAt - 60_000 > Date.now())
    return Promise.resolve(access.token);
  refreshing ??= refresh().finally(() => {
    refreshing = null;
  });
  return refreshing;
}

export async function signIn() {
  const verifier = randomString(32);
  const challenge = base64url(
    await Crypto.digestStringAsync(Crypto.CryptoDigestAlgorithm.SHA256, verifier, {
      encoding: Crypto.CryptoEncoding.BASE64,
    }),
  );
  const state = randomString(16);
  const url = `${WORKOS}/authorize?${query({
    client_id: env.workosClientId,
    provider: "authkit",
    response_type: "code",
    redirect_uri: REDIRECT_URI,
    code_challenge: challenge,
    code_challenge_method: "S256",
    state,
  })}`;
  // Ephemeral on iOS keeps a signed-out user from being silently signed back in.
  const result = await WebBrowser.openAuthSessionAsync(url, REDIRECT_URI, {
    preferEphemeralSession: true,
  });
  if (result.type !== "success") return;
  const params = Linking.parse(result.url).queryParams ?? {};
  if (params.state !== state) throw new Error("Sign-in expired. Try again.");
  if (typeof params.code !== "string")
    throw new Error(
      typeof params.error_description === "string" ? params.error_description : "Sign-in failed",
    );
  await authenticate({
    grant_type: "authorization_code",
    code: params.code,
    code_verifier: verifier,
  });
}

export async function signOut() {
  const sessionId = access?.sessionId;
  await clear();
  // Android custom tabs share the browser's AuthKit cookie, so end that session too.
  if (sessionId && Platform.OS === "android")
    await WebBrowser.openAuthSessionAsync(
      `${WORKOS}/sessions/logout?${query({ session_id: sessionId, return_to: SIGNED_OUT_URI })}`,
      SIGNED_OUT_URI,
    );
}
