import { api } from "@typeaside/backend/convex/_generated/api";
import { useAction, useConvexAuth, useMutation, useQuery } from "convex/react";
import type { FunctionReturnType } from "convex/server";
import * as WebBrowser from "expo-web-browser";
import { Alert } from "react-native";
import { deviceName, forgetDeviceToken } from "@/lib/convex";
import { signOut } from "@/lib/session";
import { keyboard } from "../../modules/keyboard";

export type Account = FunctionReturnType<typeof api.viewer.current>;
export type Device = FunctionReturnType<typeof api.desktop.list>[number];

const ACCOUNT_URL = "https://typeaside.app/account";

async function signOutCompletely() {
  await forgetDeviceToken().catch(() => undefined);
  await keyboard.clearCredentials().catch(() => undefined);
  await signOut();
}

export function useAccount() {
  const { isAuthenticated } = useConvexAuth();
  const account = useQuery(api.viewer.current, isAuthenticated ? {} : "skip");
  const devices = useQuery(api.desktop.list, isAuthenticated ? {} : "skip");
  const revoke = useMutation(api.desktop.revoke);
  const remove = useAction(api.viewer.remove);

  const removeDevice = async (device: Device) => {
    try {
      await revoke({ id: device._id });
      if (isThisPhone(device)) await signOutCompletely();
    } catch {
      Alert.alert("Device could not be removed. Please try again.");
    }
  };

  return {
    account,
    devices,
    signOut: () => void signOutCompletely(),
    openBilling: () => void WebBrowser.openBrowserAsync(ACCOUNT_URL),
    confirmRemoveDevice: (device: Device) =>
      Alert.alert(
        `Remove ${device.name}?`,
        isThisPhone(device) ? "This phone will be signed out." : undefined,
        [
          { text: "Cancel", style: "cancel" },
          { text: "Remove", style: "destructive", onPress: () => void removeDevice(device) },
        ],
      ),
    deleteAccount: async (email: string) => {
      try {
        await remove({ email });
        await signOutCompletely();
      } catch {
        Alert.alert("Account could not be deleted. Please try again.");
      }
    },
  };
}

export const isThisPhone = (device: Device) => device.name === deviceName;

export const DELETE_MESSAGE =
  "This permanently deletes your account, usage and connected devices, and cancels any active subscription. Type your email to confirm.";

export const planLabel = (account: Account) => (account.plan === "pro" ? "Pro" : "Free");

export const billingAction = (account: Account) =>
  account.billingIssue ? "Fix billing" : account.plan === "pro" ? "Manage" : "Upgrade";

export const usageLabel = ({ words, cap }: Account["usage"]) =>
  cap === null
    ? `${words.toLocaleString()} words`
    : `${words.toLocaleString()} / ${cap.toLocaleString()} words`;

export const usageProgress = ({ words, cap }: Account["usage"]) =>
  cap === null ? null : Math.min(words / cap, 1);

// Weeks reset at 00:00 UTC, matching the web account page.
export const resetsLabel = ({ resetsAt }: Account["usage"]) =>
  `Resets ${new Date(resetsAt).toLocaleDateString(undefined, { weekday: "long", timeZone: "UTC" })}`;

export function lastUsed(at: number | undefined) {
  if (at === undefined) return "Never used";
  const minutes = Math.floor((Date.now() - at) / 60_000);
  if (minutes < 1) return "Now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "Yesterday" : `${days} days ago`;
}
