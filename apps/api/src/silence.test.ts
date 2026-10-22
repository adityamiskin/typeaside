import { expect, test } from "vite-plus/test";
import { trimSilence } from "./silence";

const RATE = 16_000;

function wav(samples: number[]) {
  const pcm = new Int16Array(samples.map((sample) => Math.round(sample * 32_767)));
  const header = new DataView(new ArrayBuffer(44));
  const label = (offset: number, value: string) => {
    for (let i = 0; i < value.length; i++) header.setUint8(offset + i, value.charCodeAt(i));
  };
  label(0, "RIFF");
  header.setUint32(4, 36 + pcm.byteLength, true);
  label(8, "WAVEfmt ");
  header.setUint32(16, 16, true);
  header.setUint16(20, 1, true);
  header.setUint16(22, 1, true);
  header.setUint32(24, RATE, true);
  header.setUint32(28, RATE * 2, true);
  header.setUint16(32, 2, true);
  header.setUint16(34, 16, true);
  label(36, "data");
  header.setUint32(40, pcm.byteLength, true);
  return new File([header.buffer, pcm.buffer], "audio.wav", { type: "audio/wav" });
}

const silence = (seconds: number, level = 0) =>
  Array.from({ length: seconds * RATE }, (_, i) => (i % 2 ? level : -level));
const tone = (seconds: number, level: number) =>
  Array.from(
    { length: seconds * RATE },
    (_, i) => level * Math.sin((2 * Math.PI * 220 * i) / RATE),
  );
const seconds = async (audio: Blob | null) =>
  ((await audio!.arrayBuffer()).byteLength - 44) / 2 / RATE;

test("drops a clip with no speech", async () => {
  expect(await trimSilence(wav(silence(3)))).toBeNull();
  expect(await trimSilence(wav(silence(3, 0.002)))).toBeNull();
});

test("trims leading and trailing silence but keeps padding and inner pauses", async () => {
  const audio = await trimSilence(
    wav([...silence(4), ...tone(1, 0.3), ...silence(2), ...tone(1, 0.3), ...silence(5)]),
  );
  expect(await seconds(audio)).toBeCloseTo(4.6, 1);
});

test("keeps quiet speech above a steady noise floor", async () => {
  const audio = await trimSilence(
    wav([...silence(2, 0.004), ...tone(1, 0.03), ...silence(2, 0.004)]),
  );
  expect(await seconds(audio)).toBeCloseTo(1.6, 1);
});

test("passes through audio it cannot read as 16 kHz mono PCM", async () => {
  const mp3 = new File([new Uint8Array(1_000)], "note.mp3", { type: "audio/mpeg" });
  expect(await trimSilence(mp3)).toBe(mp3);
});

test("keeps a clip that is speech from start to end", async () => {
  const audio = wav(tone(3, 0.3));
  expect(await trimSilence(audio)).toBe(audio);
});
