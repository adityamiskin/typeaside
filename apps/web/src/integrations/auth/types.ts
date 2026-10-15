import type { AnyRequestMiddleware } from "@tanstack/react-start";
import type { ConvexProviderWithAuth } from "convex/react";
import type { ComponentProps } from "react";
import type { ReactNode } from "react";

export type Viewer = { id: string; email: string | null; name: string | null };

// Each auth option implements this in its own file; `./index.ts` re-exports the active one.
export type AuthIntegration = {
  label: string;
  middleware: AnyRequestMiddleware[];
  Provider: (props: { children: ReactNode }) => ReactNode;
  // Server function: safe to call from beforeLoad, loaders, and server routes.
  getSession: () => Promise<{ viewer: Viewer | null; accessToken: string | null }>;
  signInHref: (returnTo?: string) => string;
  signUpHref: string;
  signOutHref: string;
  useConvexAuth: ComponentProps<typeof ConvexProviderWithAuth>["useAuth"];
};
