import { createFileRoute } from "@tanstack/react-router";
import { buttonVariants } from "@hermes/ui/components/button";
import {
  AppWindow,
  Sparkles,
  WandSparkles,
  BookA,
  TextQuote,
  Palette,
  Languages,
  ClipboardPaste,
  ShieldCheck,
  Check,
  Plus,
} from "lucide-react";
import { useState } from "react";
import { Demo } from "../components/demo";
import { Reveal } from "../components/reveal";
import { DownloadButton } from "../components/site";
import { BillingToggle, CheckoutButton, type Interval } from "../components/billing";

export const Route = createFileRoute("/")({ component: Landing });
const features = [
  {
    icon: AppWindow,
    title: "Works in every app",
    body: "Slack, email, docs, your editor, the terminal. If there's a cursor, Hermes types there.",
  },
  {
    icon: Sparkles,
    title: "Cleans as you speak",
    body: "Drops filler words, fixes false starts and punctuates. Change your mind mid-sentence and it keeps the final version.",
  },
  {
    icon: WandSparkles,
    title: "Command mode",
    body: "Select text, hold the command shortcut and say what to do: make it friendlier, shorten it, translate it.",
  },
  {
    icon: BookA,
    title: "Personal dictionary",
    body: "Names, jargon and product terms, spelled your way every time.",
  },
  {
    icon: TextQuote,
    title: "Snippets",
    body: 'Say a cue like "my calendar link" and get the full text.',
  },
  {
    icon: Palette,
    title: "Styles per app",
    body: "Casual in chat, polished in email. The tone follows the app you're in.",
  },
  {
    icon: Languages,
    title: "100+ languages",
    body: "Dictate in the language you think in, or mix a few.",
  },
  {
    icon: ClipboardPaste,
    title: "Paste last",
    body: "Clicked away too soon? One shortcut pastes your last dictation again.",
  },
  {
    icon: ShieldCheck,
    title: "Private by default",
    body: "Audio isn't stored. History stays on your device. Bring your own key and requests go straight to your provider.",
  },
];
const faqs = [
  [
    "Is my audio stored?",
    "No. It's sent for transcription and discarded. Your history lives only on your computer.",
  ],
  [
    "What counts as a word?",
    "Words in the final text Hermes types. Commands and edits count the same way.",
  ],
  [
    "Can I switch between Pro and my own key?",
    "Yes, any time in Settings. Your own key is used directly and never touches our servers.",
  ],
  ["Does it work offline?", "Not yet. Transcription runs in the cloud for speed and accuracy."],
  ["Which languages?", "Over 100, detected automatically."],
  ["Which computers?", "macOS 13+, Windows 10 and 11, and Linux with Wayland."],
];
function Landing() {
  const [interval, setInterval] = useState<Interval>("monthly");
  return (
    <main className="container">
      <section className="hero">
        <div className="hero-copy">
          <h1>
            Speak, and it's <em>written</em>.
          </h1>
          <p className="lead">
            Hermes turns your voice into clean, punctuated text in any app. Hold a key, talk, let
            go.
          </p>
          <div className="hero-actions">
            <DownloadButton />
            <a href="#pricing" className={buttonVariants({ variant: "outline" })}>
              See pricing
            </a>
          </div>
          <p className="small hero-note">macOS, Windows and Linux. Free to start.</p>
        </div>
        <Demo />
      </section>
      <Reveal id="features">
        <h2>Writes the way you would.</h2>
        <div className="feature-grid">
          {features.map(({ icon: Icon, title, body }) => (
            <article key={title}>
              <Icon size={20} aria-hidden="true" />
              <h3>{title}</h3>
              <p>{body}</p>
            </article>
          ))}
        </div>
      </Reveal>
      <Reveal>
        <h2>Fast because it's native.</h2>
        <p className="lead">
          Written in Rust with a GPU-rendered interface.
          <br className="desktop-break" /> No browser engine bundled inside.
        </p>
        <div className="columns stats">
          {[
            ["~100 ms", "to open the window"],
            ["~25 MB", "memory while idle"],
            ["1 key", "hold to talk, release to type"],
          ].map(([number, label]) => (
            <div key={number}>
              <div className="stat-number">{number}</div>
              <p>{label}</p>
            </div>
          ))}
        </div>
      </Reveal>
      <Reveal>
        <h2>Three steps. No new habits.</h2>
        <div className="columns steps">
          <div>
            <span>1</span>
            <h3>Hold your shortcut</h3>
            <p>
              Press and hold <kbd>Right Alt</kbd> to start dictating.
            </p>
          </div>
          <div>
            <span>2</span>
            <h3>Speak naturally</h3>
            <p>Ramble, pause, correct yourself. Hermes keeps up.</p>
          </div>
          <div>
            <span>3</span>
            <h3>Let go</h3>
            <p>Clean text lands where your cursor is.</p>
          </div>
        </div>
      </Reveal>
      <Reveal id="pricing">
        <h2>Simple pricing.</h2>
        <BillingToggle interval={interval} setInterval={setInterval} />
        <div className="columns pricing">
          <Price
            name="Free"
            price="$0"
            description="Hosted transcription with a weekly allowance."
            items={[
              "2,000 words a week",
              "Every feature included",
              "No API key needed",
              "macOS, Windows and Linux",
            ]}
          >
            <a
              href="/download"
              className={buttonVariants({ variant: "outline", className: "w-full" })}
            >
              Download
            </a>
          </Price>
          <Price
            name="Pro"
            pro
            price={interval === "monthly" ? "$12" : "$8"}
            yearly={interval === "yearly"}
            description="Unlimited dictation, nothing to manage."
            items={[
              "Unlimited words",
              "Every feature included",
              "No API key needed",
              "Cancel anytime",
            ]}
          >
            <CheckoutButton interval={interval} full />
          </Price>
          <Price
            name="Bring your own key"
            price="$0"
            description="Use your own Groq or OpenAI key."
            items={[
              "Unlimited, billed by your provider",
              "Every feature included",
              "Key stored in your OS keychain",
              "No account needed",
            ]}
          >
            <a
              href="/download"
              className={buttonVariants({ variant: "outline", className: "w-full" })}
            >
              Download
            </a>
          </Price>
        </div>
        <p className="small pricing-note">Prices in USD. Taxes calculated at checkout.</p>
      </Reveal>
      <Reveal className="faq">
        <h2>Questions.</h2>
        <div>
          {faqs.map(([question, answer]) => (
            <details key={question}>
              <summary>
                {question}
                <Plus size={18} aria-hidden="true" />
              </summary>
              <p>{answer}</p>
            </details>
          ))}
        </div>
      </Reveal>
      <Reveal className="closing">
        <h2>Stop typing everything.</h2>
        <a href="/download" className={buttonVariants()}>
          Download
        </a>
      </Reveal>
    </main>
  );
}
function Price({
  name,
  price,
  pro = false,
  yearly = false,
  description,
  items,
  children,
}: {
  name: string;
  price: string;
  pro?: boolean;
  yearly?: boolean;
  description: string;
  items: string[];
  children: React.ReactNode;
}) {
  return (
    <article className={pro ? "pro" : ""}>
      <h3>{name}</h3>
      <div className="price">
        {price}
        {pro && <span>/mo</span>}
      </div>
      <div className="price-billing small">{yearly ? "Billed $96 yearly" : "\u00a0"}</div>
      <p className="price-description">{description}</p>
      {children}
      <ul>
        {items.map((item) => (
          <li key={item}>
            <Check size={16} aria-hidden="true" />
            {item}
          </li>
        ))}
      </ul>
    </article>
  );
}
