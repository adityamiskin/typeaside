import { createFileRoute } from "@tanstack/react-router";
import { DocsContent, loadDocsPage } from "../../components/docs-page";
import { seo } from "../../lib/seo";

export const Route = createFileRoute("/docs/")({
  loader: () => loadDocsPage([]),
  head: ({ loaderData }) =>
    seo({
      path: "/docs",
      title: `${loaderData?.title ?? "Docs"} | TypeAside`,
      description: loaderData?.description,
    }),
  component: Page,
});
function Page() {
  return <DocsContent path={Route.useLoaderData().path} />;
}
