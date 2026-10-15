import { cloudflare } from "@cloudflare/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
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
    server: {
      port: 3001,
    },
    resolve: {
      tsconfigPaths: true,
    },
    plugins: lazyPlugins(() => [
      cloudflare({ viteEnvironment: { name: "ssr" } }),
      tailwindcss(),
      tanstackStart(),
      viteReact(),
    ]),
  };
});
