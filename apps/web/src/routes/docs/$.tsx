import { createFileRoute } from "@tanstack/react-router";
import { DocsContent, loadDocsPage } from "../../components/docs-page";
import { seo } from "../../lib/seo";

export const Route = createFileRoute("/docs/$")({
  loader: ({ params }) => loadDocsPage(params._splat?.split("/").filter(Boolean) ?? []),
  head: ({ loaderData, params }) =>
    seo({
      path: `/docs/${params._splat}`,
      title: `${loaderData?.title ?? "Docs"} | TypeAside`,
      description: loaderData?.description,
    }),
  component: Page,
});
function Page() {
  return <DocsContent path={Route.useLoaderData().path} />;
}
