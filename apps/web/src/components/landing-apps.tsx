import { useEffect, useRef, useState } from "react";
import { apps, type AppName } from "../lib/apps";

export function AppTile({ name }: { name: AppName }) {
  return (
    <span className="app-tile" title={apps[name]}>
      <img src={`/img/apps/${name}.svg`} alt={apps[name]} width={28} height={28} />
    </span>
  );
}

const marqueeApps = Object.keys(apps) as AppName[];

export function AppMarquee() {
  const marquee = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = marquee.current;
    if (!element) return;
    const observer = new IntersectionObserver(([entry]) => {
      if (!entry.isIntersecting) return;
      element.classList.add("in-view");
      observer.disconnect();
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return (
    <div className="app-marquee" ref={marquee}>
      <p>Works wherever you type</p>
      <div className="marquee-window">
        <div className="marquee-track">
          <div className="marquee-copy">
            {marqueeApps.map((name) => (
              <AppTile key={name} name={name} />
            ))}
          </div>
          <div className="marquee-copy" aria-hidden="true">
            {marqueeApps.map((name) => (
              <AppTile key={name} name={name} />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

const words = ["Gmail", "Slack", "Notion", "Cursor", "Linear", "Docs", "ChatGPT"] as const;
const useCases = [
  { apps: ["notion", "obsidian", "drive"], text: "Get ideas down before they slip away." },
  {
    apps: ["cursor", "claude", "linear"],
    text: "Prompt AI and write tickets at the speed you think.",
  },
  {
    apps: ["gmail", "slack", "whatsapp"],
    text: "Answer email and chat without touching the keyboard.",
  },
] as const;

export function LandingApps() {
  const section = useRef<HTMLElement>(null);
  const [index, setIndex] = useState(0);
  useEffect(() => {
    const element = section.current;
    if (!element) return;
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const timers: ReturnType<typeof setTimeout>[] = [];
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry.isIntersecting) return;
        observer.disconnect();
        if (media.matches) return;
        words.forEach((_, i) => timers.push(setTimeout(() => setIndex(i), i * 650)));
      },
      { threshold: 0.3 },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
      timers.forEach(clearTimeout);
    };
  }, []);
  return (
    <section className="app-uses" ref={section}>
      <div className="wrap">
        <h2 className="rolling-heading">
          <span className="rolling-static">Just say it in any app.</span>
          <span className="rolling-line" aria-label={`Just say it in ${words[index]}`}>
            <span aria-hidden="true">Just say it in</span>{" "}
            <span className="rolling-window" aria-hidden="true">
              <span
                className="rolling-column"
                style={{ transform: `translateY(${(1 - index) * 1.12}em)` }}
              >
                {words.map((word, i) => (
                  <span key={word} className={i === index ? "active" : ""}>
                    {word}
                  </span>
                ))}
              </span>
            </span>
          </span>
        </h2>
        <div className="use-cases">
          {useCases.map((useCase) => (
            <div className="use-case" key={useCase.text}>
              <div className="use-case-apps">
                {useCase.apps.map((name) => (
                  <AppTile key={name} name={name} />
                ))}
              </div>
              <p>{useCase.text}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
