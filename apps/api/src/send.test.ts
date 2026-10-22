import { expect, test } from "vite-plus/test";
import { trailingSend } from "./send";

test.each([
  ["See you soon. Send.", "See you soon."],
  ["hello SEND!?", "hello"],
  ["hello send…  ", "hello"],
  ["hello.send.", "hello."],
  ["send.", ""],
  ["send", ""],
  [" Send! ", ""],
  ["resend", null],
  ["please resend.", null],
  ["send it", null],
  ["send this tomorrow", null],
  ["hello send friend", null],
  ["", null],
  ["发送send", null],
])("trailing send in %j", (raw, expected) => {
  expect(trailingSend(raw)).toBe(expected);
});
