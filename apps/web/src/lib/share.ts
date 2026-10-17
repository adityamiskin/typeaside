import { z } from "zod";

const count = (max: number) =>
  z.preprocess(
    (value) => (value === undefined ? 0 : value),
    z
      .union([z.number(), z.string().regex(/^\d+$/).transform(Number)])
      .pipe(z.number().int().nonnegative())
      .transform((value) => Math.min(value, max)),
  );

const shareSearch = z.object({
  w: count(100_000_000),
  m: count(10_000_000),
  wpm: count(400),
  d: count(10_000),
});

export function parseShareSearch(search: unknown) {
  const result = shareSearch.safeParse(search);
  return result.success ? result.data : { w: 0, m: 0, wpm: 0, d: 0 };
}

export type ShareStats = ReturnType<typeof parseShareSearch>;

export function shareContent(stats: ShareStats) {
  const words = stats.w.toLocaleString("en-US");
  const saved = `${Math.floor(stats.m / 60)}h ${stats.m % 60}m`;
  const query = new URLSearchParams({
    w: String(stats.w),
    m: String(stats.m),
    wpm: String(stats.wpm),
    d: String(stats.d),
  });
  return {
    words,
    saved,
    headline: `${words} words, typed by voice.`,
    image: `/api/og/share?${query}`,
    path: `/share?${query}` as const,
  };
}
