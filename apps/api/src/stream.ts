import { readContext, type Input, type Result } from "./dictate";
import { error } from "./errors";

const MAX_AUDIO_BYTES = 19_200_000;
const IDLE_MS = 30_000;
const DEADLINE_MS = 11 * 60_000;

function wav(chunks: ArrayBuffer[], bytes: number) {
  const header = new ArrayBuffer(44);
  const view = new DataView(header);
  const label = (offset: number, value: string) => {
    for (let i = 0; i < value.length; i++) view.setUint8(offset + i, value.charCodeAt(i));
  };
  label(0, "RIFF");
  view.setUint32(4, 36 + bytes, true);
  label(8, "WAVEfmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, 16_000, true);
  view.setUint32(28, 32_000, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  label(36, "data");
  view.setUint32(40, bytes, true);
  return new File([header, ...chunks], "audio.wav", { type: "audio/wav" });
}

export function stream(
  ctx: ExecutionContext,
  run: (input: Input) => Promise<Result | ReturnType<typeof error>>,
  onClose: () => void,
) {
  const { 0: client, 1: server } = new WebSocketPair();
  server.accept();
  server.binaryType = "arraybuffer";
  let chunks: ArrayBuffer[] = [];
  let bytes = 0;
  let released = false;
  let closed = false;
  let idle: number | null = null;
  let deadline: number | null = null;

  const clear = () => {
    chunks = [];
    bytes = 0;
  };
  const stopTimers = () => {
    clearTimeout(idle);
    clearTimeout(deadline);
  };
  const end = () => {
    if (closed) return;
    closed = true;
    stopTimers();
    clear();
    onClose();
  };
  const respond = (result: Awaited<ReturnType<typeof run>>) => {
    released = true;
    clear();
    if (closed) return;
    server.send(JSON.stringify(result));
    server.close(1000);
    end();
  };
  const expire = (message: string) => () => {
    if (!released) respond(error(408, "request", message));
  };
  const listen = () => {
    clearTimeout(idle);
    idle = setTimeout(expire("Stream idle"), IDLE_MS);
  };
  deadline = setTimeout(expire("Stream exceeded 11 minutes"), DEADLINE_MS);
  listen();
  server.addEventListener("close", end);
  server.addEventListener("error", end);
  server.addEventListener("message", (event) => {
    if (released || closed) return;
    listen();
    if (typeof event.data !== "string") {
      if (event.data.byteLength % 2 !== 0) {
        respond(error(400, "request", "Invalid dictation input"));
      } else if (bytes + event.data.byteLength > MAX_AUDIO_BYTES) {
        respond(error(413, "request", "Audio exceeds 10 minutes"));
      } else {
        bytes += event.data.byteLength;
        chunks.push(event.data);
      }
      return;
    }
    released = true;
    stopTimers();
    const context = readContext(event.data);
    if (!context || context.text !== undefined || bytes === 0) {
      respond(error(400, "request", "Invalid dictation input"));
      return;
    }
    const audio = wav(chunks, bytes);
    clear();
    // Register the handler's work so metering can finish even after the socket closes.
    ctx.waitUntil(
      run({ context, audio })
        .then(respond)
        .catch(() => respond(error(502, "provider", "Dictation provider is unavailable"))),
    );
  });
  return new Response(null, { status: 101, webSocket: client });
}
