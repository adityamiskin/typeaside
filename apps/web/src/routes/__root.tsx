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
import appCss from "../index.css?url";
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
      { title: "TypeAside: just say it" },
      {
        name: "description",
        content:
          "Clean, punctuated voice dictation in any app. Native on macOS, Windows and Linux. Free to start.",
      },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      { rel: "icon", type: "image/svg+xml", href: "/favicon.svg" },
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
