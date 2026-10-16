import { createFileRoute } from "@tanstack/react-router";
import { SignIn } from "../components/site";
import { noindex } from "../lib/seo";
export const Route = createFileRoute("/sign-in")({
  head: () => ({ meta: [{ title: "Sign in · TypeAside" }, ...noindex.meta] }),
  component: () => <SignIn />,
});
