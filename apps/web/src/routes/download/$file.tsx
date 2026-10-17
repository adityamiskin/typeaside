import { createFileRoute, notFound } from "@tanstack/react-router";
import { useEffect, useRef, type CSSProperties, type ReactNode } from "react";
import {
  ArrowRight,
  File,
  Folder,
  MousePointer2,
  Package,
  Search,
  ToggleRight,
} from "lucide-react";
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

const launcher = (
  <Window>
    <div className="steps-search">
      <Search />
      typeaside
    </div>
    <div className="steps-launcher">
      <Logo />
      TypeAside
    </div>
  </Window>
);

function packageSteps(filename: string, store: string): Step[] {
  return [
    [
      <Downloads name={filename} icon={<Package />} />,
      <>
        Open <strong>{filename}</strong> from your <strong>Downloads</strong> folder.
      </>,
    ],
    [
      <Window>
        <div className="steps-title">
          <Logo />
          TypeAside
        </div>
        <span className="steps-mini-button">Install</span>
      </Window>,
      <>
        Click <strong>Install</strong> in <strong>{store}</strong>.
      </>,
    ],
    [
      launcher,
      <>
        Open <strong>TypeAside</strong> from your apps.
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
      launcher,
      <>
        Open <strong>TypeAside</strong> from the <strong>Start</strong> menu.
      </>,
    ],
  ],
  deb: packageSteps("typeaside.deb", "App Center"),
  rpm: packageSteps("typeaside.rpm", "Software"),
  appimage: [
    [
      <Downloads name="TypeAside.AppImage" icon={<Package />} />,
      <>
        Right-click <strong>TypeAside.AppImage</strong> in <strong>Downloads</strong> and choose{" "}
        <strong>Properties</strong>.
      </>,
    ],
    [
      <Window>
        <div className="steps-title">TypeAside.AppImage</div>
        <div className="steps-toggle">
          Executable as Program
          <ToggleRight />
        </div>
      </Window>,
      <>
        Turn on <strong>Executable as Program</strong>.
      </>,
    ],
    [
      launcher,
      <>
        Double-click <strong>TypeAside.AppImage</strong> to open it.
      </>,
    ],
  ],
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
      <p className="steps-help">
        Not working? <a href={file.href}>Download again</a>
        <span> · </span>
        <a href="/download">Other platforms</a>
      </p>
    </main>
  );
}
