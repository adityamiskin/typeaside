import { createFileRoute } from "@tanstack/react-router";
import { buttonVariants } from "@typeaside/ui/components/button";
import { noindex } from "../lib/seo";
export const Route = createFileRoute("/auth-error")({
  head: () => noindex,
  component: () => (
    <main className="container page narrow">
      <h1>Sign-in couldn't finish.</h1>
      <p className="lead">Please try signing in again.</p>
      <a className={buttonVariants()} href="/api/auth/sign-in">
        Try again
      </a>
    </main>
  ),
});
