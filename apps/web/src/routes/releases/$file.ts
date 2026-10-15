import { createFileRoute } from "@tanstack/react-router";
import { releaseFiles } from "../../lib/downloads";

export const Route = createFileRoute("/releases/$file")({
  server: {
    handlers: {
      GET: async ({ params }) => {
        if (!releaseFiles.has(params.file)) return new Response("Not found", { status: 404 });
        const { env } = await import("cloudflare:workers");
        const object = await env.RELEASES.get(params.file);
        if (!object) return new Response("Not found", { status: 404 });
        return new Response(object.body, {
          headers: {
            "content-type":
              params.file === "latest.json" ? "application/json" : "application/octet-stream",
            "content-length": String(object.size),
            ...(params.file === "latest.json"
              ? {}
              : { "content-disposition": `attachment; filename="${params.file}"` }),
            etag: object.httpEtag,
            "cache-control": "public, max-age=300",
          },
        });
      },
    },
  },
});
