import { api } from "@typeaside/backend/convex/_generated/api";
import { ConvexReactClient } from "convex/react";
import * as Device from "expo-device";
import * as SecureStore from "expo-secure-store";
import { env } from "./env";

export const convex = new ConvexReactClient(env.convexUrl, { unsavedChangesWarning: false });

const DEVICE_TOKEN_KEY = "typeaside.deviceToken";
export const deviceName = Device.deviceName ?? Device.modelName ?? "Phone";

// Dictation goes through the same hosted API as the desktop app, which accepts device tokens.
export async function deviceToken() {
  const stored = await SecureStore.getItemAsync(DEVICE_TOKEN_KEY);
  if (stored) return stored;
  const token = await convex.mutation(api.desktop.createToken, { name: deviceName });
  await SecureStore.setItemAsync(DEVICE_TOKEN_KEY, token);
  return token;
}

export async function forgetDeviceToken() {
  if (!(await SecureStore.getItemAsync(DEVICE_TOKEN_KEY))) return;
  const devices = await convex.query(api.desktop.list, {}).catch(() => []);
  const own = devices.find((device) => device.name === deviceName);
  if (own) await convex.mutation(api.desktop.revoke, { id: own._id }).catch(() => undefined);
  await SecureStore.deleteItemAsync(DEVICE_TOKEN_KEY);
}
