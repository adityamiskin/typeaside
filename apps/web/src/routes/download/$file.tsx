import { createFileRoute, notFound } from "@tanstack/react-router";
import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
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
import { downloadFiles } from "../../lib/downloads";
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

type Step = [ui: ReactNode, caption: ReactNode];

const permissions = `sudo usermod -aG input "$USER"
echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' | sudo tee /etc/udev/rules.d/99-typeaside.rules
echo uinput | sudo tee /etc/modules-load.d/typeaside.conf`;

function linux(name: string, install: string): Step[] {
  return [
    [
      <Downloads name={name} icon={<Package />} />,
      <>
        Open a terminal in your <strong>Downloads</strong> folder.
      </>,
    ],
    [
      <Terminal command={install} />,
      <>
        Run <Command command={install} />
      </>,
    ],
    [
      <Window>
        <div className="steps-title">
          <ShieldCheck />
          Keyboard access
        </div>
        <p>/dev/input · /dev/uinput</p>
        <span className="steps-mini-button">Restart</span>
      </Window>,
      <>
        Allow <strong>keyboard access</strong> with the commands below, then restart and open{" "}
        <strong>TypeAside</strong>.
      </>,
    ],
  ];
}

const steps: Record<(typeof downloadFiles)[number]["slug"], Step[]> = {
  mac: [
    [
      <Downloads name="TypeAside.dmg" icon={<File />} dots />,
      <>
        Open <strong>TypeAside.dmg</strong> from your <strong>Downloads</strong> folder.
      </>,
    ],
    [
      <Window className="steps-applications">
        <div>
          <Logo />
          TypeAside
        </div>
        <ArrowRight />
        <div>
          <Folder />
          Applications
        </div>
      </Window>,
      <>
        Drag <strong>TypeAside</strong> into <strong>Applications</strong>.
      </>,
    ],
    [
      <Window>
        <div className="steps-title">Privacy &amp; Security</div>
        <p>TypeAside was blocked to protect your Mac.</p>
        <span className="steps-mini-button">Open Anyway</span>
      </Window>,
      <>
        Open <strong>TypeAside</strong>. If macOS blocks it, go to{" "}
        <strong>System Settings › Privacy &amp; Security</strong> and click{" "}
        <strong>Open Anyway</strong>.
      </>,
    ],
  ],
  windows: [
    [
      <Downloads name="TypeAside-setup.exe" icon={<File />} />,
      <>
        Run <strong>TypeAside-setup.exe</strong> from your <strong>Downloads</strong> folder.
      </>,
    ],
    [
      <Window>
        <div className="steps-title">Windows protected your PC</div>
        <span className="steps-more">More info</span>
        <span className="steps-mini-button steps-outline">Run anyway</span>
      </Window>,
      <>
        If SmartScreen appears, click <strong>More info</strong>, then <strong>Run anyway</strong>.
      </>,
    ],
    [
      <Window>
        <div className="steps-search">
          <Search />
          typeaside
        </div>
        <div className="steps-launcher">
          <Logo />
          TypeAside
        </div>
      </Window>,
      <>
        Open <strong>TypeAside</strong> from the <strong>Start</strong> menu.
      </>,
    ],
  ],
  appimage: linux("TypeAside.AppImage", "chmod +x TypeAside.AppImage && ./TypeAside.AppImage"),
  deb: linux("typeaside.deb", "sudo apt install ./typeaside.deb"),
};

function Window({ className = "", children }: { className?: string; children: ReactNode }) {
  return <div className={`steps-window glass ${className}`}>{children}</div>;
}

function Downloads({ name, icon, dots }: { name: string; icon: ReactNode; dots?: boolean }) {
  return (
    <Window>
      <div className="steps-title">
        {dots && (
          <span className="steps-dots">
            <i />
            <i />
            <i />
          </span>
        )}
        Downloads
      </div>
      <div className="steps-file">
        {icon}
        <span>{name}</span>
        <MousePointer2 className="steps-pointer" />
      </div>
    </Window>
  );
}

function Terminal({ command }: { command: string }) {
  return (
    <div className="steps-window steps-terminal">
      <span>$ </span>
      {command}▏
    </div>
  );
}

function Command({ command }: { command: string }) {
  return (
    <span className="steps-command">
      <code>{command}</code>
      <CopyButton text={command} />
    </span>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="steps-copy"
      aria-label="Copy command"
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
    >
      {copied ? <Check /> : <Copy />}
    </Button>
  );
}

function Install() {
  const file = Route.useLoaderData();
  const started = useRef("");
  useEffect(() => {
    if (started.current === file.href) return;
    started.current = file.href;
    window.location.assign(file.href);
  }, [file.href]);

  return (
    <main className="container page steps">
      <h1>Your download has started.</h1>
      <p className="lead">Three steps and you're dictating.</p>
      <ol className="steps-grid">
        {steps[file.slug].map(([ui, caption], index) => (
          <li className="steps-item" key={index} style={{ "--step": index } as CSSProperties}>
            <span className="steps-number" aria-hidden="true">
              {index + 1}
            </span>
            <div
              className="panel photo steps-panel"
              aria-hidden="true"
              style={{ "--img": `url(/img/${["s16", "s22", "s55"][index]}.jpg)` } as CSSProperties}
            >
              {ui}
            </div>
            <p className="steps-caption">{caption}</p>
          </li>
        ))}
      </ol>
      {(file.slug === "deb" || file.slug === "appimage") && (
        <div className="steps-setup">
          <pre>{permissions}</pre>
          <CopyButton text={permissions} />
        </div>
      )}
      <p className="steps-help">
        Not working? <a href={file.href}>Download again</a>
        <span> · </span>
        <a href="/download">Other platforms</a>
      </p>
    </main>
  );
}
