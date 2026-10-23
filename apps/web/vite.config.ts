import { cloudflare } from "@cloudflare/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import { fumadocsMdx } from "fumadocs-mdx/vite";
import viteReact from "@vitejs/plugin-react";
import { defineConfig, lazyPlugins } from "vite-plus";

export default defineConfig(() => {
  return {
    css: {
      postcss: {
        plugins: [
          {
            postcssPlugin: "scope-site-styles",
            OnceExit(root, { result }) {
              const file = result.opts.from?.split("?")[0];
              const scope = file?.endsWith("/src/styles/docs.css")
                ? "html[data-docs]"
                : file?.endsWith("/src/index.css")
                  ? "html:not([data-docs])"
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
