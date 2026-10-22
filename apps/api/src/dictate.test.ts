import { afterEach, expect, test, vi } from "vite-plus/test";
import { dictate, readContext } from "./dictate";

const env: Env = {
  CONVEX_SITE_URL: "https://unused.invalid",
  GROQ_API_KEY: "test-key",
  AZURE_SPEECH_ENDPOINT: "https://unused.invalid",
  AZURE_SPEECH_KEY: "test-key",
  DICTATE_LIMIT: { limit: async () => ({ success: true }) },
  FORWARD_LIMIT: { limit: async () => ({ success: true }) },
};

function context(text: string, sendCommand = true) {
  const parsed = readContext(
    JSON.stringify({
      mode: "dictate",
      text,
      send_command: sendCommand,
      category: "other",
      dictionary: [],
      names: [],
      tone: "casual",
      translate_to: "English",
    }),
  );
  if (!parsed) throw new Error("Invalid test context");
  return parsed;
}

afterEach(() => vi.unstubAllGlobals());

test("strips the command before cleanup and removes any reintroduced command", async () => {
  const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(
    Response.json({
      choices: [{ message: { content: "Please see you tomorrow morning. Send." } }],
    }),
  );
  vi.stubGlobal("fetch", fetch);
  const result = await dictate(
    env,
    { context: context("Please see you tomorrow morning. Send."), audio: null },
    false,
  );
  expect(fetch).toHaveBeenCalledOnce();
  expect(fetch.mock.calls[0][1]?.body).toContain(
    JSON.stringify(
      "<dictated_transcript>\nPlease see you tomorrow morning.\n</dictated_transcript>",
    ),
  );
  expect(result).toEqual({
    raw: "Please see you tomorrow morning. Send.",
    text: "Please see you tomorrow morning.",
    delivery: "submit",
  });
});

test("lone send submits without cleanup or text", async () => {
  const fetch = vi.fn<typeof globalThis.fetch>();
  vi.stubGlobal("fetch", fetch);
  expect(await dictate(env, { context: context("Send."), audio: null }, false)).toEqual({
    raw: "Send.",
    text: "",
    delivery: "submit",
  });
  expect(fetch).not.toHaveBeenCalled();
});

test("the setting can keep send as ordinary dictated text", async () => {
  const fetch = vi
    .fn<typeof globalThis.fetch>()
    .mockResolvedValue(Response.json({ choices: [{ message: { content: "Please send." } }] }));
  vi.stubGlobal("fetch", fetch);
  expect(
    await dictate(env, { context: context("Please send.", false), audio: null }, false),
  ).toEqual({
    raw: "Please send.",
    text: "Please send.",
    delivery: "insert",
  });
});
