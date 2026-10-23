declare module "cloudflare:workers" {
  export const env: {
    RELEASES: {
      get(key: string): Promise<{ body: ReadableStream; size: number; httpEtag: string } | null>;
    };
  };
}
