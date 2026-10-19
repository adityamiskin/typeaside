import { useEffect, useRef, useState, type CSSProperties } from "react";
import { ArrowRight, Sparkles } from "lucide-react";
import { Stream } from "./marketing";
import { Waveform } from "./site";

const dictionary = [
  "Siobhán",
  "Nguyen",
  "Saoirse",
  "Kubernetes",
  "PostgreSQL",
  "Joaquín",
  "Tchaikovsky",
  "Worcestershire",
] as const;
const commands = [
  "make it friendlier",
  "shorter",
  "turn this into a list",
  "translate to French",
  "fix the grammar",
] as const;

export function LandingBento() {
  const [answer, setAnswer] = useState(false);
  const grid = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = grid.current;
    if (!element) return;
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.add("entered");
          if (entry.target.classList.contains("bento-ask")) setAnswer(true);
          observer.unobserve(entry.target);
        }
      },
      { threshold: 0.3 },
    );
    element.querySelectorAll(".bento-tile").forEach((tile) => observer.observe(tile));
    return () => observer.disconnect();
  }, []);
  return (
    <div className="bento-grid" ref={grid}>
      <div className="bento-tile bento-dictionary">
        <div className="bento-tags">
          {dictionary.map((word, i) => (
            <span key={word} style={{ "--i": i } as CSSProperties}>
              {word}
            </span>
          ))}
        </div>
        <div className="bento-copy">
          <h3>Your words, spelled right</h3>
          <p>Names, products and jargon from your personal dictionary come out right every time.</p>
        </div>
      </div>
      <div
        className="bento-tile bento-languages photo"
        style={{ "--img": "url(/img/s16.jpg)" } as CSSProperties}
      >
        <div className="bento-vignette bento-translation">
          <div className="bento-mini glass">
            <div className="mini-question">
              <Waveform />
              dile que llego a las cinco
            </div>
            Tell him I'll be there at five.
          </div>
        </div>
        <div className="bento-copy glass">
          <h3>100+ languages</h3>
          <p>Speak the language you think in. TypeAside can also translate as you go.</p>
        </div>
      </div>
      <div className="bento-tile bento-ask">
        <div className="bento-vignette">
          <div className="bento-mini tilt">
            <div className="mini-question">
              <Sparkles />
              what's the capital of Australia
            </div>
            <Stream
              text="Canberra. It was purpose-built as a compromise between Sydney and Melbourne."
              active={answer}
              delay={400}
              speed={25}
            />
          </div>
        </div>
        <div className="bento-copy">
          <h3>Ask anything</h3>
          <p>Hold Right Alt and ask. The answer appears in an overlay, ready to copy.</p>
        </div>
      </div>
      <div className="bento-tile bento-edit">
        <div className="bento-tags">
          {commands.map((word, i) => (
            <span key={word} style={{ "--i": i } as CSSProperties}>
              {word}
            </span>
          ))}
        </div>
        <div className="bento-copy">
          <h3>Edit by voice</h3>
          <p>Select text, hold Right Alt, and say what to change.</p>
        </div>
      </div>
      <div className="bento-tile bento-snippets">
        <div className="bento-vignette">
          {[
            ["my address", "14 Rue Oberkampf, 75011 Paris"],
            ["calendly", "calendly.com/aditya/30min"],
          ].map(([trigger, text]) => (
            <div className="snippet-example" key={trigger}>
              <span className="snippet-trigger">{trigger}</span>
              <ArrowRight />
              <div className="snippet-expand">
                <span>{text}</span>
              </div>
            </div>
          ))}
        </div>
        <div className="bento-copy">
          <h3>Snippets</h3>
          <p>Dictate a short trigger and get the full text, every time, exactly.</p>
        </div>
      </div>
      <div
        className="bento-tile bento-private photo"
        style={{ "--img": "url(/img/s55.jpg)" } as CSSProperties}
      >
        <div className="bento-copy glass">
          <h3>Private by default</h3>
          <p>
            Audio is transcribed, then discarded. Your dictation history stays on your computer. Use
            your own key without signing in and dictation goes directly to your provider.
          </p>
        </div>
      </div>
    </div>
  );
}
