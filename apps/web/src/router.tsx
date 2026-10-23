import { QueryClient } from "@tanstack/react-query";
import { createRouter as createTanStackRouter } from "@tanstack/react-router";
import { setupRouterSsrQueryIntegration } from "@tanstack/react-router-ssr-query";
import { routeTree } from "./routeTree.gen";

export function getRouter() {
  const queryClient = new QueryClient();

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
