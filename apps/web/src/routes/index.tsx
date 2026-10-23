import { createFileRoute } from "@tanstack/react-router";
import type { CSSProperties } from "react";
import { DownloadButton } from "../components/site";
import { InstallCommand } from "../components/install-command";
import { useOS } from "../lib/downloads";
import { HeroScene } from "../components/hero-scene";
import { AppMarquee, LandingApps } from "../components/landing-apps";
import { LandingBento } from "../components/landing-bento";
import { LandingNotes, DevelopersStrip, MarketingFAQ } from "../components/marketing";
import { LandingFeatures } from "../components/landing-features";
import { seo, siteUrl } from "../lib/seo";

const description =
  "Open-source voice dictation for Linux, macOS and Windows. Hold a key, talk, and TypeAside types it out cleanly in whatever app you're using — with on-device models or your own API key.";
export const Route = createFileRoute("/")({
  head: () => ({
    ...seo({
      path: "/",
      title: "TypeAside: voice dictation for macOS, Windows and Linux",
      description,
    }),
    scripts: [
      {
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        name: "TypeAside",
        url: siteUrl,
        description,
        applicationCategory: "ProductivityApplication",
        operatingSystem: "macOS, Windows, Linux",
        downloadUrl: `${siteUrl}/download`,
      },
      {
        "@context": "https://schema.org",
        "@type": "FAQPage",
        mainEntity: faqs.map(([name, text]) => ({
          "@type": "Question",
          name,
          acceptedAnswer: { "@type": "Answer", text },
        })),
      },
    ].map((data) => ({ type: "application/ld+json", children: JSON.stringify(data) })),
  }),
  component: Landing,
});
const faqs = [
  [
    "Where are my notes kept?",
    "On your computer. Notes, your dictionary, snippets and dictation history are all stored locally.",
  ],
  [
    "Where does my voice go?",
    "Wherever you point it. With on-device models, audio never leaves your computer. With your own API key, audio goes straight from your computer to your provider. TypeAside runs no servers.",
  ],
  [
    "Which apps does it work in?",
    "Any app with a text field. TypeAside types where your cursor is, the same way you would.",
  ],
  [
    "Is there a phone app?",
    "Not yet. TypeAside today is a desktop app for Linux, macOS and Windows.",
  ],
  [
    "Can I use it offline?",
    "Yes, with on-device models. Cloud transcription and editing need an internet connection.",
  ],
] as const;
function Landing() {
  const os = useOS();
  return (
    <main className="landing">
      <div className="wrap">
        <header className="hero">
          <h1>Just say it.</h1>
          <p className="sub">
            Hold a key and talk the way you normally would. TypeAside writes it up cleanly, in
            whatever app you're typing in.
          </p>
          <DownloadButton />
          {os === "Linux" && <InstallCommand />}
          <p className="avail">Free and open source. Linux, macOS and Windows.</p>
        </header>
      </div>
      <HeroScene />
      <AppMarquee />
      <section id="features">
        <div className="wrap">
          <h2>Talk like you talk.</h2>
          <p className="lead">
            Nobody speaks in finished sentences. TypeAside keeps what you meant and quietly drops
            the rest.
          </p>
          <LandingFeatures />
        </div>
      </section>
      <LandingNotes />
      <section className="marketing-bento">
        <div className="wrap">
          <h2>Made for how you work.</h2>
          <p className="lead">
            It learns your words, speaks your language and keeps your dictation history on your
            computer.
          </p>
          <LandingBento />
        </div>
      </section>
      <DevelopersStrip />
      <div className="band photo" style={{ "--img": "url(/img/s56.jpg)" } as CSSProperties}>
        <div>
          <h2>Light enough to forget.</h2>
          <p>
            A small native app, not a browser in disguise. It opens in a tenth of a second, uses
            about 70 MB, and waits in your tray until you hold the key.
          </p>
        </div>
      </div>
      <LandingApps />
      <MarketingFAQ items={faqs} />
      <div className="wrap">
        <div className="final photo" style={{ "--img": "url(/img/s22.jpg)" } as CSSProperties}>
          <div>
            <h2>Your voice, typed.</h2>
            <DownloadButton />
          </div>
        </div>
      </div>
    </main>
  );
}
