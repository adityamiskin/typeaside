import { createFileRoute } from "@tanstack/react-router";
import { buttonVariants } from "@hermes/ui/components/button";
import { downloads, useOS } from "../lib/downloads";
export const Route = createFileRoute("/download")({
  head: () => ({ meta: [{ title: "Download Hermes" }] }),
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
      <p className="lead">Hermes for your computer. Free to start.</p>
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
                  href={file.href}
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
