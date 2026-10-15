import { useEffect, useRef, type CSSProperties } from "react";
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
  const grid = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = grid.current;
    if (!element) return;
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.add("entered");
          observer.unobserve(entry.target);
        }
      },
      { threshold: 0.4 },
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
        <div className="bento-translation">
          <div className="bubble raw glass">
            <Waveform />
            <div>dile que llego a las cinco</div>
          </div>
          <div className="bubble clean">Tell him I'll be there at five.</div>
        </div>
        <div className="bento-copy glass">
          <h3>100+ languages</h3>
          <p>Speak the language you think in. Hermes can also translate as you go.</p>
        </div>
      </div>
      <div
        className="bento-tile bento-private photo"
        style={{ "--img": "url(/img/s31.jpg)" } as CSSProperties}
      >
        <div className="bento-copy glass">
          <h3>Private by default</h3>
          <p>
            Audio is transcribed, then discarded. Your history stays on your computer. Bring your
            own key and nothing goes through us.
          </p>
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
          <p>Select text, hold the second key, and say what to change.</p>
        </div>
      </div>
    </div>
  );
}
