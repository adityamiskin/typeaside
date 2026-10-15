import { createFileRoute } from "@tanstack/react-router";
import { DocsContent, loadDocsPage } from "../../components/docs-page";

export const Route = createFileRoute("/docs/$")({
  loader: ({ params }) => loadDocsPage(params._splat?.split("/").filter(Boolean) ?? []),
  head: ({ loaderData }) => ({
    meta: [
      { title: `${loaderData?.title ?? "Docs"} | Hermes` },
      { name: "description", content: loaderData?.description },
    ],
  }),
  component: Page,
});
function Page() {
  return <DocsContent path={Route.useLoaderData().path} />;
}
