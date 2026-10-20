import { useEffect, useState } from "react";
import { AppState } from "react-native";
import { keyboard } from "../../modules/keyboard";

// The user enables the keyboard in system settings, so re-check whenever they come back.
export function useKeyboardEnabled() {
  const [enabled, setEnabled] = useState<boolean>();
  useEffect(() => {
    const check = () => void keyboard.isEnabled().then(setEnabled, () => setEnabled(undefined));
    check();
    const subscription = AppState.addEventListener("change", (state) => {
      if (state === "active") check();
    });
    return () => subscription.remove();
  }, []);
  return enabled;
}

export const keyboardStatus = (enabled: boolean | undefined) =>
  enabled === undefined ? "" : enabled ? "On" : "Off";

export const openKeyboardSettings = () => void keyboard.openSettings().catch(() => undefined);
