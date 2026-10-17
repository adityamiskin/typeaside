import { createFileRoute } from "@tanstack/react-router";
import { Button, buttonVariants } from "@typeaside/ui/components/button";
import { useEffect, useRef, useState } from "react";
import { noindex, seo, siteUrl } from "../lib/seo";
import { parseShareSearch, shareContent } from "../lib/share";

export const Route = createFileRoute("/share")({
  validateSearch: parseShareSearch,
  loaderDeps: ({ search }) => search,
  head: ({ match }) => {
    const stats = match.search;
    const content = shareContent(stats);
    const title = `${content.words} words typed by voice with TypeAside`;
    const description = `${content.saved} saved. Dictation that cleans up as you talk.`;
    const base = seo({ path: content.path, title, description });
    return {
      ...base,
      meta: [
        ...base.meta,
        ...noindex.meta,
        { property: "og:image", content: siteUrl + content.image },
        { property: "og:image:alt", content: content.headline },
        { name: "twitter:title", content: title },
        { name: "twitter:description", content: description },
        { name: "twitter:image", content: siteUrl + content.image },
        { name: "twitter:card", content: "summary_large_image" },
      ],
    };
  },
  component: Share,
});

function Share() {
  const stats = Route.useSearch();
  const content = shareContent(stats);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(location.href);
      setCopyState("copied");
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopyState("idle"), 2000);
    } catch {
      setCopyState("failed");
    }
  }

  return (
    <main className="container page share-page">
      <img
        className="share-card"
        src={content.image}
        width={1200}
        height={630}
        alt={content.headline}
      />
      <h1>{content.headline}</h1>
      <p className="lead">
        {content.saved} saved{stats.wpm > 0 ? ` at ${stats.wpm} wpm` : ""}.
      </p>
      <div className="share-actions">
        <a className={buttonVariants()} href="/download">
          Get TypeAside
        </a>
        <Button variant="outline" onClick={copyLink}>
          {copyState === "copied" ? "Copied" : "Copy link"}
        </Button>
      </div>
      <span role="status" className={copyState === "failed" ? "share-copy-error" : "sr-only"}>
        {copyState === "copied"
          ? "Link copied"
          : copyState === "failed"
            ? "Could not copy. Copy the link from your address bar."
            : ""}
      </span>
    </main>
  );
}
