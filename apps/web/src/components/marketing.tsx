import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { ArrowRight, Check, Mic, NotebookPen, Zap, PanelsTopLeft } from "lucide-react";

export function useEntered<T extends HTMLElement>(threshold = 0.3) {
  const ref = useRef<T>(null);
  const [entered, setEntered] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry.isIntersecting) return;
        setEntered(true);
        observer.disconnect();
      },
      { threshold },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [threshold]);
  return { ref, entered };
}

export function Stream({
  text,
  active,
  delay = 0,
  speed = 38,
  wordwise = false,
  codeTerm,
}: {
  text: string;
  active: boolean;
  delay?: number;
  speed?: number;
  wordwise?: boolean;
  codeTerm?: string;
}) {
  const [length, setLength] = useState(0);
  useEffect(() => {
    if (!active) {
      setLength(0);
      return;
    }
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setLength(text.length);
      return;
    }
    const units = wordwise ? (text.match(/\S+\s*/g) ?? []) : Array.from(text);
    let count = 0;
    const timers = units.map((unit, index) => {
      count += unit.length;
      const next = count;
      return setTimeout(() => setLength(next), delay + index * speed);
    });
    return () => timers.forEach(clearTimeout);
  }, [active, delay, speed, text, wordwise]);
  return (
    <span aria-label={text}>
      <span aria-hidden="true">
        {codeTerm
          ? text
              .slice(0, length)
              .split(new RegExp(`(${codeTerm})`))
              .map((part, index) => (part === codeTerm ? <code key={index}>{part}</code> : part))
          : text.slice(0, length)}
      </span>
    </span>
  );
}

export function CaptureBox({
  children,
  source,
  saved = false,
  footer = true,
}: {
  children: ReactNode;
  source?: string;
  saved?: boolean;
  footer?: boolean;
}) {
  return (
    <div className="note-capture glass">
      <div className="capture-head">
        <NotebookPen />
        New note{source && <span>From {source}</span>}
      </div>
      <div className="capture-text">
        {children}
        <i className="note-caret" />
      </div>
      {footer && (
        <div className="capture-foot">
          {saved ? (
            <span className="note-saved">
              <Check />
              Saved to Notes
            </span>
          ) : (
            <>
              <span>
                Hold <kbd>Right Ctrl</kbd> to dictate
              </span>
              <span className="capture-actions">
                <span>
                  <kbd className="save-key">Enter</kbd> Save
                </span>
                <span>
                  <kbd>Esc</kbd> Close
                </span>
              </span>
            </>
          )}
        </div>
      )}
    </div>
  );
}

const quote = "Riya has the final launch Figma and needs comments by noon.";
export function NotesStage({
  text = "Block 11:00 for this",
  selection = quote,
  centered = false,
}: {
  text?: string;
  selection?: string;
  centered?: boolean;
}) {
  const { ref, entered } = useEntered<HTMLDivElement>(0.5);
  const [round, setRound] = useState(0);
  const [phase, setPhase] = useState(0);
  useEffect(() => {
    if (!entered) return;
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setPhase(5);
      return;
    }
    setPhase(1);
    const saveAt = Math.max(2450, 1350 + text.length * 38);
    const timers = [
      setTimeout(() => setPhase(2), 810),
      setTimeout(() => setPhase(3), saveAt),
      setTimeout(() => setPhase(4), saveAt + 450),
      setTimeout(() => setPhase(5), saveAt + 2150),
    ];
    return () => timers.forEach(clearTimeout);
  }, [entered, round, text]);
  return (
    <div
      ref={ref}
      className={`notes-stage photo ${entered ? "entered" : ""} ${centered ? "notes-stage-centered" : ""}`}
      data-phase={phase}
      style={{ "--img": "url(/img/s31.jpg)" } as CSSProperties}
    >
      <div className="notes-canvas" key={round}>
        <div className="note-back glass">
          <div className="window-dots">
            <i />
            <i />
            <i />
          </div>
          {selection ? (
            <p>
              The launch moves to Thursday. <mark>{selection}</mark> Marketing will send the
              announcement once the build is out.
            </p>
          ) : null}
          <div className="note-lines">
            <i />
            <i />
            <i />
            <i />
          </div>
        </div>
        <CaptureBox source={selection ? "Chrome" : undefined} saved={phase >= 3}>
          {selection && (
            <>
              {`"${selection}"`}
              <br />
            </>
          )}
          <Stream text={text} active={phase >= 2} delay={420} />
        </CaptureBox>
        <div className="note-keys">
          <kbd>Shift</kbd>
          <kbd>Shift</kbd>
        </div>
        {!centered && (
          <div className="note-stack glass">
            <div className="stack-label">Today</div>
            <div className="stack-insert">
              <div className="stack-new">
                <b>{`"${selection.slice(0, 37)}…"`}</b>
                <small>{text}</small>
              </div>
            </div>
            <div>Call the bank before Friday</div>
            <div>
              <b>Trip</b>
              <small>Passports, chargers, train tickets</small>
            </div>
            <div>Ask Leo for the Linux logs</div>
          </div>
        )}
      </div>
      <button
        className="note-replay"
        hidden={phase < 5}
        onClick={() => {
          setPhase(0);
          setRound(round + 1);
        }}
      >
        Play again
      </button>
    </div>
  );
}

export function LandingNotes() {
  return (
    <section className="marketing-notes wrap">
      <h2>
        Catch the thought.
        <br />
        Keep your place.
      </h2>
      <p className="lead">
        Double-tap Left Shift in any app. Type or say the note, press Enter, and you're right back
        where you were. Your notes stay on your computer.
      </p>
      <NotesStage />
      <div className="notes-trio">
        <div>
          <h3>
            <Zap />
            From anywhere
          </h3>
          <p>
            No window to find, no tab to open. The box comes to you and leaves when you're done.
          </p>
        </div>
        <div>
          <h3>
            <Mic />
            Type it or say it
          </h3>
          <p>
            Hold your dictation key in the box and the note writes itself, cleaned up like
            everything else.
          </p>
        </div>
        <div>
          <h3>
            <PanelsTopLeft />
            Yours alone
          </h3>
          <p>Notes save on your computer. Open the sidebar any time to search, edit or pin them.</p>
        </div>
      </div>
      <a className="marketing-more" href="/docs/features/notes">
        More about Notes <ArrowRight />
      </a>
    </section>
  );
}

export function MarketingFAQ({ items }: { items: readonly (readonly [string, string])[] }) {
  return (
    <section>
      <div className="wrap faq">
        <h2>Questions</h2>
        <div>
          {items.map(([question, answer]) => (
            <details key={question}>
              <summary>
                {question}
                <i aria-hidden="true" />
              </summary>
              <p>{answer}</p>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}

const prompt =
  "Refactor save_history so it writes a temp file first, then renames it. Keep the public signature and add a test for a crash halfway through.";
export function DevelopersStrip() {
  const { ref, entered } = useEntered<HTMLDivElement>();
  return (
    <section className="wrap developers-strip" ref={ref}>
      <div>
        <h2>Talk to your agents.</h2>
        <p className="lead">
          Prompts are long and typing them is slow. Say what you want in Claude Code, Codex or
          Cursor. TypeAside keeps file names and code terms exact.
        </p>
      </div>
      <div
        className="developer-stage photo"
        style={{ "--img": "url(/img/hero.jpg)" } as CSSProperties}
      >
        <div className="developer-window">
          <div className="terminal-bar">
            <div className="window-dots">
              <i />
              <i />
              <i />
            </div>
            <span>~/code/typeaside</span>
          </div>
          <div className="terminal-body">
            <span className="terminal-dim">claude</span>
            <br />
            <span className="terminal-ready">●</span> Ready.
            <div className="terminal-prompt">
              &gt;{" "}
              <Stream
                text={prompt}
                active={entered}
                delay={1800}
                speed={70}
                wordwise
                codeTerm="save_history"
              />
            </div>
          </div>
        </div>
        <div className="developer-speech glass">
          <Mic />
          <Stream
            text="“refactor save history so it writes a temp file first then renames it…”"
            active={entered}
            speed={24}
          />
        </div>
      </div>
    </section>
  );
}
