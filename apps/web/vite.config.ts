import { cloudflare } from "@cloudflare/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import { fumadocsMdx } from "fumadocs-mdx/vite";
import viteReact from "@vitejs/plugin-react";
import { defineConfig, lazyPlugins, loadEnv } from "vite-plus";

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), "");
  for (const key of [
    "WORKOS_CLIENT_ID",
    "WORKOS_API_KEY",
    "WORKOS_REDIRECT_URI",
    "WORKOS_COOKIE_PASSWORD",
  ]) {
    if (localEnv[key] && process.env[key] === undefined) process.env[key] = localEnv[key];
  }
  return {
    css: {
      postcss: {
        plugins: [
          {
            postcssPlugin: "scope-site-styles",
            OnceExit(root, { result }) {
              const file = result.opts.from?.split("?")[0];
              const scope = file?.endsWith("/src/styles/docs.css")
                ? "html.hermes-docs"
                : file?.endsWith("/src/index.css")
                  ? "html:not(.hermes-docs)"
                  : undefined;
              if (!scope) return;
              root.walkRules((rule) => {
                rule.selector = rule.selector.replaceAll(":root", ":scope");
              });
              const nodes = root.nodes.filter(
                (node) =>
                  node.type !== "atrule" ||
                  !["font-face", "property", "charset"].includes(node.name),
              );
              for (const node of nodes) node.remove();
              root.append({ name: "scope", params: `(${scope})`, nodes });
            },
          },
        ],
      },
    },
    server: {
      port: 3001,
    },
    resolve: {
      tsconfigPaths: true,
    },
    plugins: lazyPlugins(() => [
      cloudflare({ viteEnvironment: { name: "ssr" } }),
      fumadocsMdx(),
      tailwindcss(),
      tanstackStart(),
      viteReact(),
    ]),
  };
});
