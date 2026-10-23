import { createFileRoute } from "@tanstack/react-router";
import { siteUrl } from "../lib/seo";

export const Route = createFileRoute("/sitemap.xml")({
  server: {
    handlers: {
      GET: async () => {
        const { source } = await import("../lib/docs-source");
        const paths = [
          "/",
          "/download",
          ...source.getPages().map((page) => page.url),
          "/privacy",
          "/terms",
        ];
        const urls = paths.map((path) => `<url><loc>${siteUrl}${path}</loc></url>`).join("");
        return new Response(
          `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${urls}</urlset>`,
          {
            headers: { "content-type": "application/xml", "cache-control": "public, max-age=3600" },
          },
        );
      },
    },
  },
});
