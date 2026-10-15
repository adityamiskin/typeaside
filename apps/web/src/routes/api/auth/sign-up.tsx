import { createFileRoute } from "@tanstack/react-router";
import { getSignUpUrl } from "@workos/authkit-tanstack-react-start";

import { authConfigured } from "../../../integrations/auth/config.server";

export const Route = createFileRoute("/api/auth/sign-up")({
  server: {
    handlers: {
      GET: async ({ request }: { request: Request }) => {
        if (!(await authConfigured()))
          return Response.redirect(new URL("/sign-in", request.url), 303);
        const candidate = new URL(request.url).searchParams.get("returnPathname");
        const returnPathname =
          candidate?.startsWith("/") &&
          !candidate.startsWith("//") &&
          !["\\", "\n", "\r", "\t"].some((character) => candidate.includes(character))
            ? candidate
            : undefined;
        const url = await getSignUpUrl(returnPathname ? { data: { returnPathname } } : undefined);

        return new Response(null, {
          status: 307,
          headers: {
            Location: url,
          },
        });
      },
    },
  },
});
