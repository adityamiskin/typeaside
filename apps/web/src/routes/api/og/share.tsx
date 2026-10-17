import { createFileRoute } from "@tanstack/react-router";
import { ImageResponse } from "workers-og";
import regularFont from "../../../assets/hanken-grotesk-400.ttf?inline";
import semiboldFont from "../../../assets/hanken-grotesk-600.ttf?inline";
import { Waveform } from "../../../components/site";
import { parseShareSearch, shareContent } from "../../../lib/share";

function fontData(dataUrl: string) {
  return Uint8Array.from(atob(dataUrl.split(",")[1]!), (character) => character.charCodeAt(0))
    .buffer;
}
const fonts = [
  { name: "Hanken Grotesk", data: fontData(regularFont), weight: 400 as const },
  { name: "Hanken Grotesk", data: fontData(semiboldFont), weight: 600 as const },
];

export const Route = createFileRoute("/api/og/share")({
  server: {
    handlers: {
      GET: async ({ request }) => {
        const stats = parseShareSearch(Object.fromEntries(new URL(request.url).searchParams));
        const content = shareContent(stats);
        const items = [
          ...(stats.m > 0 ? [{ value: content.saved, label: "saved" }] : []),
          ...(stats.wpm > 0 ? [{ value: String(stats.wpm), label: "wpm" }] : []),
          ...(stats.d > 0 ? [{ value: `${stats.d} days`, label: "streak" }] : []),
        ];
        return new ImageResponse(
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              width: 1200,
              height: 630,
              padding: 80,
              backgroundColor: "#f9f9f8",
              backgroundImage:
                "radial-gradient(circle 700px at 100% 0%, #e6effa, rgba(230, 239, 250, 0)), radial-gradient(circle 600px at 0% 100%, #efe6f5, rgba(239, 230, 245, 0))",
              color: "#16171a",
              fontFamily: "Hanken Grotesk",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 12,
                fontSize: 32,
                fontWeight: 600,
              }}
            >
              <Waveform size={40} />
              <span>TypeAside</span>
            </div>
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                flexGrow: 1,
                justifyContent: "center",
              }}
            >
              <div
                style={{
                  display: "flex",
                  fontSize: 168,
                  fontWeight: 600,
                  letterSpacing: "-0.04em",
                  lineHeight: 1,
                }}
              >
                {content.words}
              </div>
              <div style={{ display: "flex", fontSize: 40, color: "#6b6e76", marginTop: 12 }}>
                words typed by voice
              </div>
            </div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 64 }}>
              {items.map((item) => (
                <div
                  key={item.label}
                  style={{ display: "flex", flexDirection: "column", flexShrink: 0 }}
                >
                  <span style={{ fontSize: 36, fontWeight: 600 }}>{item.value}</span>
                  <span style={{ fontSize: 22, color: "#6b6e76" }}>{item.label}</span>
                </div>
              ))}
              <span style={{ marginLeft: "auto", fontSize: 24, color: "#6b6e76", flexShrink: 0 }}>
                typeaside.app
              </span>
            </div>
          </div>,
          {
            width: 1200,
            height: 630,
            fonts,
            headers: { "Cache-Control": "public, max-age=31536000, immutable" },
          },
        );
      },
    },
  },
});
