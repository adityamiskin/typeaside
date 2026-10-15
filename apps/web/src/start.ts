import { createCsrfMiddleware, createStart, createIsomorphicFn } from "@tanstack/react-start";

import { authConfigured } from "./integrations/auth/config.server";
import { auth } from "./integrations/auth";

const csrfMiddleware = createCsrfMiddleware({
  filter: (ctx) => ctx.handlerType === "serverFn",
});

const authMiddleware = createIsomorphicFn()
  .client(async () => [])
  .server(async () => ((await authConfigured()) ? auth.middleware : []));

export const startInstance = createStart(async () => ({
  requestMiddleware: [csrfMiddleware, ...(await authMiddleware())],
}));
