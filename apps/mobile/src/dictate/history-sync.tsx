import { useEffect } from "react";
import { AppState, Platform } from "react-native";
import { z } from "zod";
import { keyboard } from "../../modules/keyboard";
import { take, useTakes } from "./takes";

export function HistorySync() {
  useEffect(() => {
    if (Platform.OS !== "android") return;
    let pending = Promise.resolve();
    const sync = () => {
      pending = pending
        .then(async () => {
          const entries = z.array(take).parse(JSON.parse(await keyboard.readHistory()));
          if (!entries.length) return;
          useTakes.getState().merge(entries);
          await keyboard.acknowledgeHistory(entries.map((entry) => entry.id));
        })
        .catch((error: unknown) => console.warn("Dictation history:", error));
    };
    const events = keyboard.addListener("onHistoryChanged", sync);
    const foreground = AppState.addEventListener("change", (state) => {
      if (state === "active") sync();
    });
    sync();
    return () => {
      events.remove();
      foreground.remove();
    };
  }, []);
  return null;
}
