import { requireNativeModule } from "expo";

// Shared with the iOS keyboard extension and Android accessibility service,
// which dictates through the hosted API without the JS runtime.
type Credentials = { token: string; apiUrl: string; dictionary: string[] };

type KeyboardModule = {
  setCredentials(credentials: Credentials): Promise<void>;
  clearCredentials(): Promise<void>;
  isEnabled(): Promise<boolean>;
  openSettings(): Promise<void>;
};

export const keyboard = requireNativeModule<KeyboardModule>("Keyboard");
