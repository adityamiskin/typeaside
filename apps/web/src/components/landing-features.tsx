import { useEffect, useRef, type CSSProperties } from "react";
import { Waveform } from "./site";
const FEATURES = [
  {
    img: "s22",
    tint: "#eef5fc",
    hl: "#d7e8f8",
    t: "Fillers drop out",
    d: "Ums, uhs, likes and false starts never make it to the page.",
    raw: "[Um,] can you send me the deck [like] before the call? [uh] thanks",
    out: "Can you send me the deck before the call? Thanks.",
  },
  {
    img: "s55",
    tint: "#fdf1ef",
    hl: "#f8dcd6",
    t: "It follows your corrections",
    d: "Change your mind halfway through. Hermes keeps the version you landed on.",
    raw: "let's meet tomorrow at [seven, no actually,] three",
    out: "Let's meet tomorrow at three.",
  },
  {
    img: "s56",
    tint: "#f4f2fb",
    hl: "#e3def6",
    t: "Spells your words right",
    d: "Names, products and jargon from your personal dictionary come out right every time.",
    raw: "ask priya to look at the kubernetes pr before standup",
    out: "Ask Priya to look at the Kubernetes PR before standup.",
  },
  {
    img: "s18",
    tint: "#f6eef7",
    hl: "#ead9ee",
    t: "Formats as you go",
    d: "Say a list and get a list. Numbers, dates and emails come out the way you'd type them.",
    raw: "for the trip we need passports chargers and the train tickets",
    out: "For the trip we need:\n1. Passports\n2. Chargers\n3. Train tickets",
  },
  {
    img: "s31",
    tint: "#f3f4f3",
    hl: "#e4e6e4",
    t: "Edits by voice",
    d: "Select some text, hold the second key, and say what to change.",
    raw: "make this sound friendlier",
    out: "Hey! Could you send the file over when you get a chance? Thank you!",
  },
] as const;

function Feature({ feature }: { feature: (typeof FEATURES)[number] }) {
  const panel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = panel.current;
    if (!element) return;
    const raw = element.querySelector(".raw");
    const clean = element.querySelector(".clean");
    if (!raw || !clean) return;
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const timers: ReturnType<typeof setTimeout>[] = [];
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        raw.classList.add("play");
        timers.push(
          setTimeout(
            () => {
              raw.classList.add("cut");
              timers.push(setTimeout(() => clean.classList.add("show"), reduce ? 0 : 500));
            },
            reduce ? 0 : raw.querySelectorAll(".w").length * 45 + 350,
          ),
        );
      },
      { threshold: 0.5 },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
      timers.forEach(clearTimeout);
    };
  }, []);
  let index = 0;
  const raw = feature.raw
    .split(/(\[[^\]]+\])/)
    .filter(Boolean)
    .flatMap((part, partIndex) =>
      part.startsWith("[")
        ? [
            <mark className="w" key={partIndex} style={{ "--i": index++ } as CSSProperties}>
              {part.slice(1, -1)}
            </mark>,
          ]
        : part
            .split(/(\s+)/)
            .filter(Boolean)
            .map((word, wordIndex) =>
              /\s/.test(word) ? (
                word
              ) : (
                <span
                  className="w"
                  key={`${partIndex}-${wordIndex}`}
                  style={{ "--i": index++ } as CSSProperties}
                >
                  {word}
                </span>
              ),
            ),
    );
  return (
    <div className="feature">
      <div>
        <h3>{feature.t}</h3>
        <p>{feature.d}</p>
      </div>
      <div
        ref={panel}
        className="panel photo"
        style={
          {
            "--img": `url(/img/${feature.img}.jpg)`,
            "--tint": feature.tint,
            "--hl": feature.hl,
          } as CSSProperties
        }
      >
        <div className="bubble raw glass">
          <Waveform />
          <div>{raw}</div>
        </div>
        <div className="bubble clean">{feature.out}</div>
      </div>
    </div>
  );
}
export function LandingFeatures() {
  return (
    <div id="features-list">
      {FEATURES.map((feature) => (
        <Feature key={feature.t} feature={feature} />
      ))}
    </div>
  );
}
