import { createFileRoute, notFound } from "@tanstack/react-router";
import { useEffect, useRef, useState, type CSSProperties } from "react";
import {
  ArrowRight,
  Check,
  Copy,
  File,
  Folder,
  MousePointer2,
  Package,
  Search,
  ShieldCheck,
} from "lucide-react";
import { Button } from "@typeaside/ui/components/button";
import { Logo } from "../../components/site";
import { downloadFiles, type DownloadFile, type InstallStep } from "../../lib/downloads";
import { seo } from "../../lib/seo";

export const Route = createFileRoute("/download/$file")({
  loader: ({ params }) => {
    const file = downloadFiles.find((file) => file.slug === params.file);
    if (!file) throw notFound();
    return file;
  },
  head: ({ params }) => seo({ path: `/download/${params.file}`, title: "Install TypeAside" }),
  component: Install,
});

function Command({ command }: { command: string }) {
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  return (
    <span className="steps-command">
      <code>{command}</code>
      <Button
        variant="ghost"
        size="icon-xs"
        className="steps-copy"
        aria-label={copied ? "Copied command" : "Copy command"}
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(command);
            setCopied(true);
            setFailed(false);
            clearTimeout(timer.current);
            timer.current = setTimeout(() => setCopied(false), 1500);
          } catch {
            setFailed(true);
          }
        }}
      >
        {copied ? <Check /> : <Copy />}
      </Button>
      <span className="sr-only" role="status">
        {copied ? "Command copied" : failed ? "Could not copy. Select and copy the command." : ""}
      </span>
    </span>
  );
}

function Vignette({ step, file }: { step: InstallStep; file: DownloadFile }) {
  switch (step.vignette) {
    case "downloads":
      return (
        <div className="steps-window glass">
          <div className="steps-title">
            {file.slug === "mac" && (
              <span className="steps-dots">
                <i />
                <i />
                <i />
              </span>
            )}
            Downloads
          </div>
          <div className="steps-file">
            {file.slug === "deb" || file.slug === "appimage" ? <Package /> : <File />}
            <span>{file.filename}</span>
            <MousePointer2 className="steps-pointer" />
          </div>
        </div>
      );
    case "applications":
      return (
        <div className="steps-window steps-applications glass">
          <div>
            <Logo />
            <span>TypeAside</span>
          </div>
          <ArrowRight />
          <div>
            <Folder />
            <span>Applications</span>
          </div>
        </div>
      );
    case "privacy":
      return (
        <div className="steps-window glass">
          <div className="steps-title">Privacy &amp; Security</div>
          <p>TypeAside was blocked to protect your Mac.</p>
          <span className="steps-mini-button">Open Anyway</span>
        </div>
      );
    case "smartscreen":
      return (
        <div className="steps-window glass">
          <div className="steps-title">Windows protected your PC</div>
          <span className="steps-more">More info</span>
          <span className="steps-mini-button steps-outline">Run anyway</span>
        </div>
      );
    case "launcher":
      return (
        <div className="steps-window glass">
          <div className="steps-search">
            <Search />
            typeaside
          </div>
          <div className="steps-launcher">
            <Logo />
            TypeAside
          </div>
        </div>
      );
    case "terminal":
      return (
        <div className="steps-window steps-terminal">
          <span>$ </span>
          {step.command}
          <span className="steps-caret">▏</span>
        </div>
      );
    case "permissions":
      return (
        <div className="steps-window glass">
          <div className="steps-title">
            <ShieldCheck />
            Input permissions
          </div>
          <p>/dev/input · /dev/uinput</p>
          <span className="steps-signout">Sign out, then sign back in</span>
        </div>
      );
  }
}

function Install() {
  const file = Route.useLoaderData();
  return <InstallFile key={file.slug} file={file} />;
}

function InstallFile({ file }: { file: DownloadFile }) {
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    window.location.assign(file.href);
  }, [file.href]);
  return (
    <main className="container page steps">
      <h1>Your download has started.</h1>
      <p className="lead">Three steps and you're dictating.</p>
      <ol className="steps-grid">
        {file.steps.map((step, index) => (
          <li className="steps-item" key={index} style={{ "--step": index } as CSSProperties}>
            <span className="steps-number" aria-hidden="true">
              {index + 1}
            </span>
            <div
              className="panel photo steps-panel"
              aria-hidden="true"
              style={{ "--img": `url(/img/${["s16", "s22", "s55"][index]}.jpg)` } as CSSProperties}
            >
              <Vignette step={step} file={file} />
            </div>
            <p className="steps-caption">
              {step.caption.map((part, index) =>
                typeof part === "string" ? (
                  part
                ) : "strong" in part ? (
                  <strong key={index}>{part.strong}</strong>
                ) : (
                  <Command key={index} command={part.command} />
                ),
              )}
            </p>
          </li>
        ))}
      </ol>
      {file.slug === "deb" && (
        <p className="steps-note">
          On Arch: <code>yay -S typeaside-bin</code>
        </p>
      )}
      <p className="steps-help">
        Not working? <a href={file.href}>Download again</a>
        <span> · </span>
        <a href="/download">Other platforms</a>
      </p>
    </main>
  );
}
