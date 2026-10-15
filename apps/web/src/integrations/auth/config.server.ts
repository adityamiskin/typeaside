export async function authConfigured() {
  let bindings: Record<string, unknown> = process.env;
  if (import.meta.env.PROD) {
    try {
      bindings = (await import("cloudflare:workers")).env;
    } catch {
      /* Plain Vite builds also run without workerd. */
    }
  }
  return [
    "WORKOS_CLIENT_ID",
    "WORKOS_API_KEY",
    "WORKOS_REDIRECT_URI",
    "WORKOS_COOKIE_PASSWORD",
  ].every((key) => typeof bindings[key] === "string" && !String(bindings[key]).includes("replace"));
}
