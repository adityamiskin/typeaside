import { createFileRoute } from "@tanstack/react-router";
import { seo } from "../../lib/seo";
import { buttonVariants } from "@typeaside/ui/components/button";
import { comingSoon, downloads, useOS } from "../../lib/downloads";
import { InstallCommand } from "../../components/install-command";
export const Route = createFileRoute("/download/")({
  head: () =>
    seo({
      path: "/download",
      title: "Download TypeAside",
      description:
        "Download TypeAside voice dictation for macOS, Windows and Linux. Android and iOS coming soon. Free to start, or bring your own API key.",
    }),
  component: Download,
});
function Download() {
  const os = useOS();
  const first = <T extends string>(keys: T[]) =>
    keys.sort((a, b) => Number(b === os) - Number(a === os));
  const systems = first(Object.keys(downloads) as (keyof typeof downloads)[]);
  const phones = first(Object.keys(comingSoon) as (keyof typeof comingSoon)[]);
  const onPhone = phones[0] === os;
  const phoneRows = phones.map((phone) => (
    <section className="download-row" key={phone}>
      <div>
        <h2>{phone}</h2>
        <p>{comingSoon[phone]}</p>
      </div>
      <span className="download-soon">Coming soon</span>
    </section>
  ));
  return (
    <main className="container page">
      <h1>Make room for your voice.</h1>
      <p className="lead">TypeAside for your computer and phone. Free to start.</p>
      <div className="download-list">
        {onPhone && phoneRows}
        {systems.map((system) => (
          <section className="download-row" key={system}>
            <div>
              <h2>{system}</h2>
              <p>{downloads[system].requirement}</p>
              {system === "Linux" && <InstallCommand />}
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
        {!onPhone && phoneRows}
      </div>
    </main>
  );
}
