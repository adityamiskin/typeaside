import { ConvexQueryClient } from "@convex-dev/react-query";
import { QueryClient } from "@tanstack/react-query";
import { createRouter as createTanStackRouter } from "@tanstack/react-router";
import { setupRouterSsrQueryIntegration } from "@tanstack/react-router-ssr-query";
import { env } from "@typeaside/env/web";
import { ConvexProviderWithAuth, ConvexReactClient } from "convex/react";

import { auth } from "./integrations/auth";
import { routeTree } from "./routeTree.gen";

export function getRouter() {
  const convexClient = new ConvexReactClient(env.VITE_CONVEX_URL ?? "http://127.0.0.1:3210");
  const convexQueryClient = new ConvexQueryClient(convexClient);

  const queryClient: QueryClient = new QueryClient({
    defaultOptions: {
      queries: {
        queryKeyHashFn: convexQueryClient.hashFn(),
        queryFn: convexQueryClient.queryFn(),
      },
    },
  });
  convexQueryClient.connect(queryClient);

  const router = createTanStackRouter({
    routeTree,
    parseSearch: (search) => Object.fromEntries(new URLSearchParams(search)),
    stringifySearch: (search: Record<string, unknown>) => {
      const params = new URLSearchParams();
      for (const [key, value] of Object.entries(search)) {
        if (typeof value === "string" || typeof value === "number" || typeof value === "boolean")
          params.set(key, String(value));
      }
      const query = params.toString();
      return query ? `?${query}` : "";
    },
    defaultPreload: "intent",
    defaultPendingComponent: () => (
      <main className="container page">
        <p>Loading…</p>
      </main>
    ),
    defaultNotFoundComponent: () => <div>Not Found</div>,
    context: {
      queryClient,
      convexClient,
      convexQueryClient,
    },
    // Providers wrap inside out: data clients first, auth outermost.
    Wrap: ({ children }) => {
      children = (
        <ConvexProviderWithAuth client={convexClient} useAuth={auth.useConvexAuth}>
          {children}
        </ConvexProviderWithAuth>
      );
      children = <auth.Provider>{children}</auth.Provider>;
      return <>{children}</>;
    },
  });

  setupRouterSsrQueryIntegration({
    router,
    queryClient,
  });

  return router;
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
