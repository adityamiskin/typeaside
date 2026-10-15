import { createFileRoute, Outlet } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { useFumadocsLoader } from "fumadocs-core/source/client";
import { DocsLayout } from "fumadocs-ui/layouts/docs";
import { RootProvider } from "fumadocs-ui/provider/tanstack";
import { Waveform } from "../components/site";
import { source } from "../lib/docs-source";
import docsCss from "../styles/docs.css?url";

const loadTree = createServerFn({ method: "GET" }).handler(async () => ({
  tree: await source.serializePageTree(source.getPageTree()),
}));

export const Route = createFileRoute("/docs")({
  loader: () => loadTree(),
  head: () => ({ links: [{ rel: "stylesheet", href: docsCss }] }),
  component: Documentation,
});

function Documentation() {
  const { tree } = useFumadocsLoader(Route.useLoaderData());
  return (
    <RootProvider theme={{ enabled: false }} search={{ options: { api: "/api/docs-search" } }}>
      <DocsLayout
        tree={tree}
        themeSwitch={{ enabled: false }}
        nav={{
          title: (
            <span className="docs-logo">
              <Waveform />
              Hermes
            </span>
          ),
          url: "/",
        }}
        links={[{ text: "Download", url: "/download" }]}
      >
        <Outlet />
      </DocsLayout>
    </RootProvider>
  );
}
