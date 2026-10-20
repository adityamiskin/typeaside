import { api } from "@typeaside/backend/convex/_generated/api";
import { useAction, useConvexAuth } from "convex/react";
import { NativeTabs } from "expo-router/native-tabs";
import { useEffect } from "react";
import { connectKeyboard } from "@/lib/dictate";
import { useTabsAppearance } from "@/ui/theme";

export const unstable_settings = { initialRouteName: "notes" };

export default function TabsLayout() {
  const { isAuthenticated } = useConvexAuth();
  const sync = useAction(api.viewer.sync);
  const appearance = useTabsAppearance();
  useEffect(() => {
    if (!isAuthenticated) return;
    void sync()
      .then(connectKeyboard)
      .catch(() => undefined);
  }, [isAuthenticated, sync]);

  return (
    <NativeTabs {...appearance} labelVisibilityMode="labeled">
      <NativeTabs.Trigger name="notes">
        <NativeTabs.Trigger.Label>Notes</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf="note.text" md="sticky_note_2" />
      </NativeTabs.Trigger>
      <NativeTabs.Trigger name="history">
        <NativeTabs.Trigger.Label>History</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf="clock.arrow.circlepath" md="history" />
      </NativeTabs.Trigger>
      <NativeTabs.Trigger name="dictionary">
        <NativeTabs.Trigger.Label>Dictionary</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf="character.book.closed" md="book_2" />
      </NativeTabs.Trigger>
      <NativeTabs.Trigger name="snippets">
        <NativeTabs.Trigger.Label>Snippets</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf="text.quote" md="short_text" />
      </NativeTabs.Trigger>
    </NativeTabs>
  );
}
