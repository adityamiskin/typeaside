import type { ConvexQueryClient } from "@convex-dev/react-query";
import type { QueryClient } from "@tanstack/react-query";
import { HeadContent, Outlet, Scripts, createRootRouteWithContext } from "@tanstack/react-router";
import type { ConvexReactClient } from "convex/react";
import displayFont from "@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff2?url";
import bodyFont from "@fontsource-variable/inter/files/inter-latin-wght-normal.woff2?url";
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
      { title: "Hermes · Speak, and it's written." },
      {
        name: "description",
        content:
          "Clean, punctuated voice dictation in any app. Native on macOS, Windows and Linux. Free to start.",
      },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      ...[displayFont, bodyFont].map((href) => ({
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
const themeScript = `(function(){try{var t=localStorage.getItem('hermes-theme');document.documentElement.className=t==='light'||t==='dark'?t:matchMedia('(prefers-color-scheme: light)').matches?'light':'dark'}catch(e){document.documentElement.className=matchMedia('(prefers-color-scheme: light)').matches?'light':'dark'}})()`;
function RootDocument() {
  return (
    <html lang="en" className="dark" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
        <HeadContent />
      </head>
      <body>
        <a className="skip-link" href="#main">
          Skip to content
        </a>
        <Header />
        <div id="main">
          <Outlet />
        </div>
        <Footer />
        <Scripts />
      </body>
    </html>
  );
}
