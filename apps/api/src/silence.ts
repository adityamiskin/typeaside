const RATE = 16_000;
const FRAME = RATE / 50;
const PADDING_FRAMES = 15;
const MIN_SPEECH_FRAMES = 3;
// Matches the desktop's silence gate; the cap keeps a clip that is all speech from raising its own floor.
const MIN_THRESHOLD = 0.003;
const MAX_THRESHOLD = 0.02;

function pcm16Mono(header: DataView) {
  const tag = (offset: number) => String.fromCharCode(...new Uint8Array(header.buffer, offset, 4));
  return (
    header.byteLength === 44 &&
    tag(0) === "RIFF" &&
    tag(8) === "WAVE" &&
    tag(12) === "fmt " &&
    tag(36) === "data" &&
    header.getUint16(20, true) === 1 &&
    header.getUint16(22, true) === 1 &&
    header.getUint32(24, true) === RATE &&
    header.getUint16(34, true) === 16
  );
}

function speechFrames(samples: Int16Array) {
  const levels = Array.from({ length: Math.floor(samples.length / FRAME) }, (_, frame) => {
    let sum = 0;
    for (let i = frame * FRAME; i < (frame + 1) * FRAME; i++) sum += (samples[i]! / 32_768) ** 2;
    return Math.sqrt(sum / FRAME);
  });
  const floor = [...levels].sort((a, b) => a - b)[Math.floor(levels.length / 10)] ?? 0;
  const threshold = Math.max(MIN_THRESHOLD, Math.min(floor * 3, MAX_THRESHOLD));
  let first = -1;
  let last = -1;
  let run = 0;
  levels.forEach((level, frame) => {
    run = level >= threshold ? run + 1 : 0;
    if (run < MIN_SPEECH_FRAMES) return;
    if (first < 0) first = frame - run + 1;
    last = frame;
  });
  return first < 0 ? null : { first, last, frames: levels.length };
}

/** Trims silent edges from 16 kHz mono PCM WAV, or returns null when it holds no speech. Other audio passes through. */
export async function trimSilence(audio: Blob): Promise<Blob | null> {
  const bytes = await audio.arrayBuffer();
  if (bytes.byteLength < 44 || (bytes.byteLength - 44) % 2) return audio;
  const header = new DataView(bytes, 0, 44);
  if (!pcm16Mono(header)) return audio;
  const speech = speechFrames(new Int16Array(bytes, 44));
  if (!speech) return null;
  const start = Math.max(0, speech.first - PADDING_FRAMES) * FRAME;
  const end =
    speech.last + PADDING_FRAMES + 1 >= speech.frames
      ? (bytes.byteLength - 44) / 2
      : (speech.last + PADDING_FRAMES + 1) * FRAME;
  if (start === 0 && end === (bytes.byteLength - 44) / 2) return audio;
  const pcm = bytes.slice(44 + start * 2, 44 + end * 2);
  const trimmed = new DataView(bytes.slice(0, 44));
  trimmed.setUint32(4, 36 + pcm.byteLength, true);
  trimmed.setUint32(40, pcm.byteLength, true);
  return new File([trimmed.buffer, pcm], "audio.wav", { type: "audio/wav" });
}
