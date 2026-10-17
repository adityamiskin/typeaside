import { createFileRoute } from "@tanstack/react-router";
import { seo } from "../../lib/seo";
import { buttonVariants } from "@typeaside/ui/components/button";
import { downloads, useOS } from "../../lib/downloads";
export const Route = createFileRoute("/download/")({
  head: () =>
    seo({
      path: "/download",
      title: "Download TypeAside",
      description:
        "Download TypeAside voice dictation for macOS, Windows and Linux. Free to start, or bring your own API key.",
    }),
  component: Download,
});
function Download() {
  const os = useOS();
  const systems = (Object.keys(downloads) as (keyof typeof downloads)[]).sort(
    (a, b) => Number(b === os) - Number(a === os),
  );
  return (
    <main className="container page">
      <h1>Make room for your voice.</h1>
      <p className="lead">TypeAside for your computer. Free to start.</p>
      <div className="download-list">
        {systems.map((system) => (
          <section className="download-row" key={system}>
            <div>
              <h2>{system}</h2>
              <p>{downloads[system].requirement}</p>
            </div>
            <div className="download-actions">
              {downloads[system].files.map((file) => (
                <a
                  key={file.label}
                  className={buttonVariants({ variant: os === system ? "default" : "outline" })}
                  href={`/download/${file.slug}`}
                >
                  {file.label}
                </a>
              ))}
            </div>
          </section>
        ))}
      </div>
    </main>
  );
}
