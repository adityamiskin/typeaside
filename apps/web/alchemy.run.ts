import * as Alchemy from "alchemy";
import * as Cloudflare from "alchemy/Cloudflare";
import * as Config from "effect/Config";
import * as Effect from "effect/Effect";

// The TanStack Start app as a Worker plus static assets. Each `env` entry becomes a
// Worker binding: `Config` values are read from `.env` (or the shell) at deploy time.
export class Website extends Cloudflare.Website.Vite<Website>()("Web", {
  env: {
    VITE_CONVEX_URL: Config.String("VITE_CONVEX_URL"),
    WORKOS_CLIENT_ID: Config.String("WORKOS_CLIENT_ID"),
    WORKOS_API_KEY: Config.Redacted("WORKOS_API_KEY"),
    WORKOS_REDIRECT_URI: Config.String("WORKOS_REDIRECT_URI"),
    WORKOS_COOKIE_PASSWORD: Config.Redacted("WORKOS_COOKIE_PASSWORD"),
  },
}) {}

export type WebsiteEnv = Cloudflare.InferEnv<typeof Website>;

export default Alchemy.Stack(
  "hermes",
  {
    providers: Cloudflare.providers(),
    state: Cloudflare.state(),
  },
  Effect.gen(function* () {
    const website = yield* Website;
    return { url: website.url.as<string>() };
  }),
);
