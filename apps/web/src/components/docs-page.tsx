import { notFound } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import browserCollections from "collections/browser";
import { DocsBody, DocsDescription, DocsPage, DocsTitle } from "fumadocs-ui/layouts/docs/page";
import defaultMdxComponents from "fumadocs-ui/mdx";
import { source } from "../lib/docs-source";

const loadPage = createServerFn({ method: "GET" })
  .validator((slugs: string[]) => slugs)
  .handler(({ data }) => {
    const page = source.getPage(data);
    if (!page) throw notFound();
    return { path: page.path, title: page.data.title, description: page.data.description };
  });

export async function loadDocsPage(slugs: string[]) {
  const page = await loadPage({ data: slugs });
  await clientLoader.preload(page.path);
  return page;
}

const clientLoader = browserCollections.docs.createClientLoader({
  component({ default: MDX, frontmatter, toc }) {
    return (
      <DocsPage toc={toc}>
        <DocsTitle>{frontmatter.title}</DocsTitle>
        <DocsDescription>{frontmatter.description}</DocsDescription>
        <DocsBody>
          <MDX components={defaultMdxComponents} />
        </DocsBody>
      </DocsPage>
    );
  },
});

export function DocsContent({ path }: { path: string }) {
  return clientLoader.useContent(path);
}
