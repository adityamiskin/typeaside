import { createFileRoute, redirect } from "@tanstack/react-router";
import { ConvexError } from "convex/values";
import { useMutation, Authenticated, AuthLoading, Unauthenticated } from "convex/react";
import { useEffect, useRef, useState } from "react";
import { api } from "@typeaside/backend/convex/_generated/api";
import { Button } from "@typeaside/ui/components/button";
import { auth } from "../../integrations/auth";
import { detectOS } from "../../lib/downloads";
import { SignIn } from "../../components/site";
import { noindex } from "../../lib/seo";
export const Route = createFileRoute("/desktop/connect")({
  validateSearch: (search: Record<string, unknown>) => {
    const port =
      typeof search.port === "string" && /^\d+$/.test(search.port)
        ? Number(search.port)
        : typeof search.port === "number"
          ? search.port
          : NaN;
    return {
      port: Number.isInteger(port) && port >= 1024 && port <= 65535 ? port : undefined,
      state: typeof search.state === "string" ? search.state : undefined,
      connected: search.connected === "true" || search.connected === true || undefined,
    };
  },
  beforeLoad: ({ context, search }) => {
    if (search.port === undefined || search.state === undefined) return;
    if (!context.viewer)
      throw redirect({
        href: auth.signInHref(
          `/desktop/connect?${new URLSearchParams({ port: String(search.port), state: search.state })}`,
        ),
      });
  },
  head: () => ({
    meta: [
      { title: "Connect TypeAside" },
      { name: "referrer", content: "no-referrer" },
      ...noindex.meta,
    ],
  }),
  component: Connect,
});
function Connect() {
  const { port, state, connected } = Route.useSearch();
  if (connected)
    return (
      <main className="container page narrow">
        <p className="connect-prompt">The desktop app is connected.</p>
        <p>You can close this tab and start dictating.</p>
      </main>
    );
  if (port === undefined || state === undefined)
    return (
      <main className="container page narrow">
        <h1>Invalid connection link.</h1>
        <p className="lead">Open Connect from the TypeAside app to try again.</p>
      </main>
    );
  return (
    <>
      <Authenticated>
        <ConnectContent port={port} state={state} />
      </Authenticated>
      <AuthLoading>
        <main className="container page narrow">
          <p>Loading…</p>
        </main>
      </AuthLoading>
      <Unauthenticated>
        <SignIn
          returnTo={`/desktop/connect?${new URLSearchParams({ port: String(port), state })}`}
        />
      </Unauthenticated>
    </>
  );
}
function ConnectContent({ port, state }: { port: number; state: string }) {
  const createToken = useMutation(api.desktop.createToken);
  const { viewer } = Route.useRouteContext();
  const [busy, setBusy] = useState(false);
  const [token, setToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    if (token) form.current?.submit();
  }, [token]);
  return (
    <main className="container page narrow">
      {token && (
        <form ref={form} method="post" action={`http://127.0.0.1:${port}/callback`}>
          <input type="hidden" name="state" value={state} />
          <input type="hidden" name="token" value={token} />
        </form>
      )}
      <p className="connect-prompt">Connect the TypeAside desktop app</p>
      <p>
        {viewer?.email
          ? `The app on this computer will use ${viewer.email}.`
          : "The app on this computer will use this account."}
      </p>
      <div className="connect-actions">
        <Button
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            setError(null);
            try {
              setToken(await createToken({ name: detectOS(navigator.userAgent) ?? "Computer" }));
            } catch (cause) {
              setError(
                cause instanceof ConvexError && typeof cause.data === "string"
                  ? cause.data
                  : "Could not connect. Please try again.",
              );
              setBusy(false);
            }
          }}
        >
          {busy ? "Connecting…" : "Connect"}
        </Button>
        <a href="/">Cancel</a>
      </div>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </main>
  );
}
