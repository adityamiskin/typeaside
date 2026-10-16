export const siteUrl = "https://typeaside.app";

export function seo({
  path,
  title,
  description,
}: {
  path: `/${string}`;
  title: string;
  description?: string;
}) {
  const url = siteUrl + path;
  return {
    meta: [
      { title },
      { property: "og:title", content: title },
      { property: "og:url", content: url },
      ...(description
        ? [
            { name: "description", content: description },
            { property: "og:description", content: description },
          ]
        : []),
    ],
    links: [{ rel: "canonical", href: url }],
  };
}

export const noindex = { meta: [{ name: "robots", content: "noindex" }] };
