import { expect, test } from "vite-plus/test";
import { cleanupSystem, readContext } from "./dictate";

const context = {
  mode: "dictate",
  text: "hello",
  category: "work",
  dictionary: [],
  names: [],
  tone: "casual",
  translate_to: "English",
};
const sentence =
  "The user's formatting preference for this app (style only, never content to add):";

test("app instructions reach the cleanup prompt, JSON-escaped, for dictate and translate", () => {
  const instructions = 'lowercase "quoted"';
  const parsed = readContext(JSON.stringify({ ...context, instructions }));
  if (!parsed) throw new Error("context rejected");
  for (const translate of [false, true]) {
    expect(cleanupSystem(parsed, translate)).toContain(
      `${sentence} ${JSON.stringify(instructions)}.`,
    );
  }
  const plain = readContext(JSON.stringify(context));
  if (!plain) throw new Error("context rejected");
  expect(cleanupSystem(plain, false)).not.toContain(sentence);
});

test("instructions over 300 characters are rejected", () => {
  expect(readContext(JSON.stringify({ ...context, instructions: "x".repeat(301) }))).toBeNull();
});
