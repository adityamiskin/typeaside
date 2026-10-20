import { api } from "@typeaside/backend/convex/_generated/api";
import { useConvexAuth, useQuery } from "convex/react";
import { useEffect } from "react";
import { fetch } from "expo/fetch";
import { File } from "expo-file-system";
import { z } from "zod";
import { keyboard } from "../../modules/keyboard";
import { deviceToken, forgetDeviceToken } from "./convex";
import { env } from "./env";

const result = z.object({ raw: z.string(), text: z.string() });
const failure = z.object({
  error: z.object({ type: z.string(), message: z.string(), resets_at: z.number().optional() }),
});
type Settings = { dictionary: string[]; snippets: { trigger: string; text: string }[] };
let settings: Settings = { dictionary: [], snippets: [] };
let credentialWrite = Promise.resolve();

export class DictationError extends Error {
  constructor(
    message: string,
    readonly type: string,
    readonly resetsAt?: number,
  ) {
    super(message);
  }
}

async function request(path: string, init: { method?: string; body?: FormData } = {}) {
  const token = await deviceToken();
  const response = await fetch(`${env.apiUrl}${path}`, {
    ...init,
    headers: { Authorization: `Bearer ${token}` },
  });
  // The device was removed from the account page; the next request mints a fresh token.
  if (response.status === 401) await forgetDeviceToken();
  return response;
}

export function useKeyboardSync() {
  const { isAuthenticated } = useConvexAuth();
  const current = useQuery(api.settings.mine, isAuthenticated ? {} : "skip");
  useEffect(() => {
    if (!isAuthenticated) {
      settings = { dictionary: [], snippets: [] };
      return;
    }
    if (current === undefined) return;
    settings = { dictionary: current?.dictionary ?? [], snippets: current?.snippets ?? [] };
    credentialWrite = credentialWrite.catch(() => undefined).then(connectKeyboard);
    void credentialWrite.catch((error: unknown) => console.warn("Keyboard settings:", error));
  }, [isAuthenticated, current]);
}

export async function dictate(audioUri: string) {
  const body = new FormData();
  body.append("audio", new File(audioUri));
  body.append(
    "context",
    JSON.stringify({
      mode: "dictate",
      category: "personal",
      app_id: "app.typeaside.mobile",
      dictionary: settings.dictionary,
      snippets: settings.snippets,
      names: [],
      tone: "casual",
      translate_to: "English",
    }),
  );
  const response = await request("/v1/dictate", { method: "POST", body });
  const json: unknown = await response.json().catch(() => null);
  if (response.ok) return result.parse(json);
  const error = failure.safeParse(json);
  throw error.success
    ? new DictationError(
        error.data.error.message,
        error.data.error.type,
        error.data.error.resets_at,
      )
    : new DictationError(`Dictation failed (${response.status})`, "network");
}

// The keyboard extension and floating mic dictate without the JS runtime.
export async function connectKeyboard() {
  await keyboard.setCredentials({
    token: await deviceToken(),
    apiUrl: env.apiUrl,
    dictionary: settings.dictionary,
    snippets: settings.snippets,
  });
}
