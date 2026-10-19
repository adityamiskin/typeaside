import { api } from "@typeaside/backend/convex/_generated/api";
import { useAction, useConvexAuth } from "convex/react";
import { NativeTabs } from "expo-router/native-tabs";
import { useEffect } from "react";
import { connectKeyboard } from "@/lib/dictate";

export default function TabsLayout() {
  const { isAuthenticated } = useConvexAuth();
  const sync = useAction(api.viewer.sync);
  useEffect(() => {
    if (!isAuthenticated) return;
    void sync()
      .then(connectKeyboard)
      .catch(() => undefined);
  }, [isAuthenticated, sync]);

  return (
    <NativeTabs>
      <NativeTabs.Trigger name="dictate">
        <NativeTabs.Trigger.Label>Dictate</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf="mic" md="mic" />
      </NativeTabs.Trigger>
      <NativeTabs.Trigger name="notes">
        <NativeTabs.Trigger.Label>Notes</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf="note.text" md="sticky_note_2" />
      </NativeTabs.Trigger>
      <NativeTabs.Trigger name="account">
        <NativeTabs.Trigger.Label>Account</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf="person.crop.circle" md="account_circle" />
      </NativeTabs.Trigger>
    </NativeTabs>
  );
}
