import { createFileRoute } from "@tanstack/react-router";
import { releaseFiles } from "../../lib/downloads";

// Versioned keys (v1.2.3/file) give package managers stable URLs with fixed checksums.
const releaseKey = /^(?:v\d+\.\d+\.\d+\/)?([^/]+?)(\.minisig)?$/;

export const Route = createFileRoute("/releases/$")({
  server: {
    handlers: {
      GET: async ({ params }) => {
        const key = params._splat ?? "";
        const [, file, signature] = releaseKey.exec(key) ?? [];
        if (!file || !releaseFiles.has(file)) return new Response("Not found", { status: 404 });
        const { env } = await import("cloudflare:workers");
        const object = await env.RELEASES.get(key);
        if (!object) return new Response("Not found", { status: 404 });
        return new Response(object.body, {
          headers: {
            "content-type": signature
              ? "text/plain"
              : file === "latest.json"
                ? "application/json"
                : "application/octet-stream",
            "content-length": String(object.size),
            ...(file === "latest.json" || signature
              ? {}
              : { "content-disposition": `attachment; filename="${file}"` }),
            etag: object.httpEtag,
            "cache-control": "public, max-age=300",
          },
        });
      },
    },
  },
});
