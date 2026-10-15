// Types `import { env } from "cloudflare:workers"` with the bindings declared in
// `alchemy.run.ts`.
declare module "cloudflare:workers" {
  export const env: import("../alchemy.run").WebsiteEnv;
}
