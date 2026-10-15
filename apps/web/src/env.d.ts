declare module "cloudflare:workers" {
  export const env: {
    RELEASES: {
      get(key: string): Promise<{ body: ReadableStream; size: number; httpEtag: string } | null>;
    };
    VITE_CONVEX_URL: string;
    WORKOS_CLIENT_ID: string;
    WORKOS_API_KEY: string;
    WORKOS_REDIRECT_URI: string;
    WORKOS_COOKIE_PASSWORD: string;
  };
}
