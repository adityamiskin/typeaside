import { requireNativeModule } from "expo-modules-core";

// Shared with the system keyboard (iOS extension via the App Group, Android IME in-process),
// which dictates through the hosted API without the JS runtime.
type Credentials = { token: string; apiUrl: string; dictionary: string[] };

type KeyboardModule = {
  setCredentials(credentials: Credentials): Promise<void>;
  clearCredentials(): Promise<void>;
  isEnabled(): Promise<boolean>;
  openSettings(): Promise<void>;
};

export const keyboard = requireNativeModule<KeyboardModule>("Keyboard");
