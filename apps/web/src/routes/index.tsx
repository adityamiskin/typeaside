import { createFileRoute } from "@tanstack/react-router";
import { useState, type CSSProperties } from "react";
import { DownloadButton } from "../components/site";
import { InstallCommand } from "../components/install-command";
import { useOS } from "../lib/downloads";
import { BillingToggle, CheckoutButton, type Interval } from "../components/billing";
import { HeroScene } from "../components/hero-scene";
import { AppMarquee, LandingApps } from "../components/landing-apps";
import { LandingBento } from "../components/landing-bento";
import { LandingNotes, DevelopersStrip, MarketingFAQ } from "../components/marketing";
import { LandingFeatures } from "../components/landing-features";
import { seo, siteUrl } from "../lib/seo";

const description =
  "Voice dictation and quick notes for Linux, macOS and Windows, with Android and iOS coming soon. Talk in any app or double-tap Left Shift to capture a thought. Free to start.";
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
        offers: [
          { "@type": "Offer", name: "Free", price: "0", priceCurrency: "USD" },
          { "@type": "Offer", name: "Pro", price: "12", priceCurrency: "USD" },
        ],
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
    "Notes sync through our servers so they appear on every device you sign in to. Deleting a note deletes it. Signed-out notes stay on your computer until you sign in.",
  ],
  [
    "Where does my voice go?",
    "Audio goes to the transcription service you choose, TypeAside or your own provider, and TypeAside doesn't keep it. Your dictation history stays on your computer.",
  ],
  [
    "Which apps does it work in?",
    "Any app with a text field. TypeAside types where your cursor is, the same way you would.",
  ],
  [
    "Is there a phone app?",
    "Android and iOS apps are coming soon. Until then, your notes open at typeaside.app/notes on your phone.",
  ],
  [
    "What counts toward the free limit?",
    "Words in your transcripts. The 2,000 word allowance resets every Monday.",
  ],
  [
    "Can I switch between my own key and an account?",
    "Yes, any time in Settings. Your dictionary, snippets and history stay put.",
  ],
] as const;
function Landing() {
  const os = useOS();
  const [interval, setInterval] = useState<Interval>("monthly");
  const yearly = interval === "yearly";
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
          <p className="avail">
            Free to start. Linux, macOS and Windows. Android and iOS coming soon.
          </p>
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
      <section id="pricing" className="center">
        <div className="wrap">
          <h2>Simple pricing</h2>
          <p className="lead">
            Start free, go unlimited with Pro, or bring your own API key and pay nothing.
          </p>
          <BillingToggle interval={interval} setInterval={setInterval} />
          <div className="plans">
            <div className="plan">
              <h3>Free</h3>
              <div className="price">
                <b>$0</b>
              </div>
              <p className="note">2,000 words every week</p>
              <ul>
                <li>Sign in, nothing to set up</li>
                <li>Transcription and editing included</li>
                <li>Resets every Monday</li>
                <li>Unlimited notes, synced everywhere</li>
              </ul>
              <a className="btn light" href="/download">
                Download
              </a>
            </div>
            <div
              className="plan pro photo"
              style={{ "--img": "url(/img/s18.jpg)" } as CSSProperties}
            >
              <div className="inner glass">
                <h3>Pro</h3>
                <div className="price">
                  <b key={interval} className="flip">
                    {yearly ? "$96" : "$12"}
                  </b>
                  <span>{yearly ? "a year" : "a month"}</span>
                </div>
                <p className="note">
                  {yearly ? "Unlimited words, 4 months free" : "Unlimited words"}
                </p>
                <ul>
                  <li>Everything in Free</li>
                  <li>No weekly limit</li>
                  <li>Cancel any time</li>
                  <li>Unlimited notes, synced everywhere</li>
                </ul>
                <CheckoutButton interval={interval} full />
              </div>
            </div>
            <div className="plan">
              <h3>Your own key</h3>
              <div className="price">
                <b>$0</b>
              </div>
              <p className="note">No account, no limits</p>
              <ul>
                <li>Groq or any OpenAI-compatible API</li>
                <li>You pay your provider directly</li>
                <li>Key stays in your system keychain</li>
              </ul>
              <a className="btn light" href="/download">
                Download
              </a>
            </div>
          </div>
        </div>
      </section>
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
