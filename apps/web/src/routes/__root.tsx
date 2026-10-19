import type { ConvexQueryClient } from "@convex-dev/react-query";
import type { QueryClient } from "@tanstack/react-query";
import {
  HeadContent,
  Outlet,
  Scripts,
  useRouterState,
  createRootRouteWithContext,
} from "@tanstack/react-router";
import type { ConvexReactClient } from "convex/react";
import bodyFont from "@fontsource-variable/hanken-grotesk/files/hanken-grotesk-latin-wght-normal.woff2?url";
import { Header, Footer } from "../components/site";
import { auth } from "../integrations/auth";
import marketingCss from "../styles/marketing.css?url";
import appCss from "../index.css?url";
import { siteUrl } from "../lib/seo";
export interface RouterAppContext {
  queryClient: QueryClient;
  convexClient: ConvexReactClient;
  convexQueryClient: ConvexQueryClient;
}
export const Route = createRootRouteWithContext<RouterAppContext>()({
  beforeLoad: async ({ context }) => {
    const session = await auth.getSession();
    if (session.accessToken)
      context.convexQueryClient.serverHttpClient?.setAuth(session.accessToken);
    return { viewer: session.viewer };
  },
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "TypeAside" },
      { name: "theme-color", content: "#f9f9f8" },
      { property: "og:type", content: "website" },
      { property: "og:site_name", content: "TypeAside" },
      { property: "og:image", content: `${siteUrl}/og.png` },
      { property: "og:image:width", content: "1200" },
      { property: "og:image:height", content: "630" },
      { property: "og:image:alt", content: "TypeAside: just say it." },
      { name: "twitter:card", content: "summary_large_image" },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      { rel: "stylesheet", href: marketingCss },
      { rel: "icon", type: "image/svg+xml", href: "/favicon.svg" },
      { rel: "apple-touch-icon", href: "/apple-touch-icon.png" },
      ...[bodyFont].map((href) => ({
        rel: "preload",
        href,
        as: "font",
        type: "font/woff2",
        crossOrigin: "anonymous" as const,
      })),
    ],
  }),
  component: RootDocument,
});
function RootDocument() {
  const isDocs = useRouterState({
    select: (state) =>
      state.location.pathname === "/docs" || state.location.pathname.startsWith("/docs/"),
  });
  return (
    <html lang="en" className={isDocs ? "typeaside-docs" : undefined}>
      <head>
        <HeadContent />
      </head>
      <body>
        <a className="skip-link" href="#main">
          Skip to content
        </a>
        {!isDocs && <Header />}
        <div id="main">
          <Outlet />
        </div>
        {!isDocs && <Footer />}
        <Scripts />
      </body>
    </html>
  );
}
