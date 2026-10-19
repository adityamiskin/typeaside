import { createFileRoute } from "@tanstack/react-router";
import { Mic, Pin, Search, Type, WifiOff } from "lucide-react";
import type { CSSProperties } from "react";
import { CaptureBox, MarketingFAQ, NotesStage, Stream, useEntered } from "../components/marketing";
import { DownloadButton } from "../components/site";
import { seo } from "../lib/seo";

export const Route = createFileRoute("/features/notes")({
  head: () =>
    seo({
      path: "/features/notes",
      title: "Quick notes for Linux, macOS and Windows · TypeAside",
      description:
        "Double-tap Left Shift anywhere, type or dictate a quick note, and press Enter. Notes sync across Linux, macOS, Windows and the web.",
    }),
  component: NotesFeature,
});
const faqs = [
  [
    "Is it free?",
    "Yes. Unlimited notes and sync are included in Free and Pro. Notes you type do not count toward your dictation word allowance.",
  ],
  [
    "Where are my notes stored?",
    "Notes save on your computer and sync through our servers so they appear on every device you sign in to. Deleting a note deletes it. Signed-out notes stay local until you sign in.",
  ],
  [
    "Can I change the shortcut?",
    "Yes. Change the New note shortcut in Settings. The default is a double tap of Left Shift.",
  ],
  [
    "Does it work offline?",
    "Yes. Type and save notes offline. They sync when you're back online. Dictation needs your transcription service or a configured local model.",
  ],
] as const;
const notes = [
  "Gift idea for Maya: the green ceramic lamp",
  '"The best predictor of a good quarter…"',
  "Ask Siobhán about the Kubernetes upgrade",
];
function NotesFeature() {
  const grab = useEntered<HTMLDivElement>();
  const say = useEntered<HTMLDivElement>();
  const devices = useEntered<HTMLDivElement>();
  return (
    <main className="notes-feature landing">
      <header className="notes-hero wrap">
        <h1>
          Notes at the speed
          <br />
          of a thought.
        </h1>
        <p>
          Double-tap Left Shift anywhere, write it down, press Enter. You never leave what you were
          doing.
        </p>
        <DownloadButton />
        <div className="avail">Free. Linux, macOS and Windows.</div>
      </header>
      <div className="wrap">
        <NotesStage
          centered
          selection=""
          text="Gift idea for Maya: the green ceramic lamp from the market"
        />
      </div>
      <div className="wrap notes-feature-rows">
        <div className={`notes-feature-row ${grab.entered ? "entered" : ""}`} ref={grab.ref}>
          <div>
            <h3>Grab what you're reading.</h3>
            <p>
              Select text first and it lands in the note in quotes, with the app it came from. Add a
              line of your own and press Enter.
            </p>
          </div>
          <div
            className="notes-panel grab-panel photo"
            style={{ "--img": "url(/img/s22.jpg)" } as CSSProperties}
          >
            <div className="note-back glass">
              <p>
                Most teams over-index on velocity.{" "}
                <mark>
                  The best predictor of a good quarter was how often people talked to customers.
                </mark>
              </p>
            </div>
            <CaptureBox source="Firefox" footer={false}>
              "The best predictor of a good quarter was how often people talked to customers."
              <br />
              <Stream text="Use in Monday's all-hands" active={grab.entered} delay={1000} />
            </CaptureBox>
          </div>
        </div>
        <div className={`notes-feature-row reverse ${say.entered ? "entered" : ""}`} ref={say.ref}>
          <div>
            <h3>Or just say it.</h3>
            <p>
              Hold your dictation key while the box is open. Ums drop out, names come out right, and
              the note is clean before you let go.
            </p>
          </div>
          <div
            className="notes-panel say-panel photo"
            style={{ "--img": "url(/img/s18.jpg)" } as CSSProperties}
          >
            <CaptureBox footer={false}>
              <Stream
                text="Ask Siobhán whether the Kubernetes upgrade can wait until after the launch"
                active={say.entered}
              />
              <div className="note-listening">
                <Mic />
                Listening
              </div>
            </CaptureBox>
          </div>
        </div>
        <div className={`notes-feature-row ${devices.entered ? "entered" : ""}`} ref={devices.ref}>
          <div>
            <h3>On every device.</h3>
            <p>
              Notes sync to your account. Open typeaside.app/notes on your phone and they're already
              there.
            </p>
          </div>
          <div
            className="notes-panel photo"
            style={{ "--img": "url(/img/s16.jpg)" } as CSSProperties}
          >
            <div className="notes-desktop">
              <aside>
                <div>Home</div>
                <div className="selected">Notes</div>
                <div>Dictionary</div>
                <div>Snippets</div>
              </aside>
              <div>
                <h4>Notes</h4>
                {notes.map((note) => (
                  <p key={note}>{note}</p>
                ))}
              </div>
            </div>
            <div className="notes-phone">
              <h4>Notes</h4>
              {notes.map((note) => (
                <p key={note}>{note}</p>
              ))}
            </div>
          </div>
        </div>
      </div>
      <div className="wrap notes-four">
        {[
          {
            Icon: Pin,
            title: "Pin what matters",
            text: "Keep a few notes at the top, everything else in order by day.",
          },
          {
            Icon: Search,
            title: "Find anything",
            text: "Search every note from the app or the web.",
          },
          {
            Icon: WifiOff,
            title: "Works offline",
            text: "Notes save on your computer first and sync when you're back online.",
          },
          {
            Icon: Type,
            title: "Just text",
            text: "No formatting to fight. The first line is the title.",
          },
        ].map(({ Icon, title, text }) => (
          <div key={title}>
            <span>
              <Icon />
            </span>
            <h4>{title}</h4>
            <p>{text}</p>
          </div>
        ))}
      </div>
      <MarketingFAQ items={faqs} />
      <div className="wrap">
        <div className="final photo" style={{ "--img": "url(/img/s22.jpg)" } as CSSProperties}>
          <div>
            <h2>Write it down.</h2>
            <DownloadButton />
          </div>
        </div>
      </div>
    </main>
  );
}
