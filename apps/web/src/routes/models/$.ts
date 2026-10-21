import { createFileRoute } from "@tanstack/react-router";

// On-device model files the desktop app downloads, pinned by source revision so they never change.
const models = new Set([
  "underdog-bark-3.0@85db2ad1",
  "underdog-bark-3.0-plus@d4fab2f8",
  "underdog-bark-3.0-clean@6d34ed86",
]);
const files = new Set([
  "encoder.int8.onnx",
  "encoder.int8.data0",
  "encoder.int8.data1",
  "encoder.int8.data2",
  "decoder.int8.onnx",
  "joiner.int8.onnx",
  "tokens.txt",
]);

export const Route = createFileRoute("/models/$")({
  server: {
    handlers: {
      GET: async ({ params }) => {
        const key = params._splat ?? "";
        const [model, file, ...rest] = key.split("/");
        if (rest.length || !models.has(model ?? "") || !files.has(file ?? ""))
          return new Response("Not found", { status: 404 });
        const { env } = await import("cloudflare:workers");
        const object = await env.RELEASES.get(`models/${key}`);
        if (!object) return new Response("Not found", { status: 404 });
        return new Response(object.body, {
          headers: {
            "content-type": "application/octet-stream",
            "content-length": String(object.size),
            etag: object.httpEtag,
            "cache-control": "public, max-age=31536000, immutable",
          },
        });
      },
    },
  },
});
