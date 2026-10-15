import { useEffect, useRef, useState } from "react";
import { appIcons, type AppIconName } from "../lib/app-icons";

export function AppTile({ name }: { name: AppIconName }) {
  const { title, path } = appIcons[name];
  return (
    <span className="app-tile" title={title} role="img" aria-label={title}>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d={path} />
      </svg>
    </span>
  );
}

const marqueeApps = Object.keys(appIcons) as AppIconName[];

export function AppMarquee() {
  const marquee = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = marquee.current;
    if (!element) return;
    const observer = new IntersectionObserver(([entry]) => {
      element.classList.toggle("in-view", entry.isIntersecting);
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
  { apps: ["notion", "obsidian", "googledocs"], text: "Get ideas down before they slip away." },
  {
    apps: ["cursor", "github", "linear"],
    text: "Prompt AI and write tickets at the speed you think.",
  },
  {
    apps: ["gmail", "whatsapp", "discord"],
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
    let frame = 0;
    let near = false;
    const update = () => {
      frame = 0;
      const rect = element.getBoundingClientRect();
      const progress = (window.innerHeight - rect.top) / (window.innerHeight + rect.height);
      setIndex(Math.max(0, Math.min(words.length - 1, Math.floor(progress * words.length))));
    };
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update);
    };
    const sync = () => {
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
      cancelAnimationFrame(frame);
      frame = 0;
      if (near && !media.matches) {
        window.addEventListener("scroll", onScroll, { passive: true });
        window.addEventListener("resize", onScroll);
        onScroll();
      }
    };
    const observer = new IntersectionObserver(
      ([entry]) => {
        near = entry.isIntersecting;
        sync();
      },
      { rootMargin: "200px" },
    );
    observer.observe(element);
    media.addEventListener("change", sync);
    return () => {
      observer.disconnect();
      media.removeEventListener("change", sync);
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
      cancelAnimationFrame(frame);
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
