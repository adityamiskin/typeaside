import { useEffect, useRef, type CSSProperties } from "react";

const spoken =
  "um so I think we should uh move the launch to thursday no friday and loop in priya".split(" ");
const cut = new Set([0, 1, 6, 11, 12]);
const corrections: Readonly<Partial<Record<number, string>>> = { 13: "Friday", 17: "Priya." };
const easing = "cubic-bezier(.22,1,.36,1)";

export function HeroScene() {
  const scene = useRef<HTMLDivElement>(null);
  const replay = useRef<(() => void) | null>(null);
  useEffect(() => {
    const element = scene.current;
    if (!element) return;
    const said = element.querySelector<HTMLDivElement>(".said");
    const out = element.querySelector<HTMLDivElement>(".out");
    const placeholder = element.querySelector<HTMLSpanElement>(".ph");
    const pill = element.querySelector<HTMLDivElement>(".pill");
    const again = element.querySelector<HTMLButtonElement>(".hint button");
    if (!said || !out || !placeholder || !pill || !again) return;
    const bars = [...pill.querySelectorAll("i")];
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const timers = new Map<ReturnType<typeof setTimeout>, () => void>();
    let meter: ReturnType<typeof setInterval> | undefined;
    const animations: Animation[] = [];
    let run = 0;
    let disposed = false;
    const sleep = (ms: number) =>
      new Promise<void>((resolve) => {
        const timer = setTimeout(() => {
          timers.delete(timer);
          resolve();
        }, ms);
        timers.set(timer, resolve);
      });
    function stop() {
      clearInterval(meter);
      for (const [timer, resolve] of timers) {
        clearTimeout(timer);
        resolve();
      }
      timers.clear();
      animations.forEach((animation) => animation.cancel());
      animations.length = 0;
    }
    const play = async () => {
      stop();
      const id = ++run;
      const current = () => !disposed && id === run;
      said.replaceChildren();
      out.replaceChildren();
      placeholder.style.opacity = "1";
      again.hidden = true;
      pill.classList.remove("on");
      bars.forEach((bar) => (bar.style.height = "4px"));
      const words = spoken.map((word, index) => {
        const source = document.createElement("span");
        source.textContent = `${word} `;
        said.append(source);
        if (cut.has(index)) return { source, target: null };
        const target = document.createElement("span");
        target.textContent = `${corrections[index] ?? word} `;
        out.append(target);
        return { source, target };
      });
      if (reduce) {
        placeholder.style.opacity = "0";
        words.forEach(({ target }) => {
          if (target) target.style.opacity = "1";
        });
        again.hidden = false;
        return;
      }
      await sleep(400);
      if (!current()) return;
      pill.classList.add("on");
      meter = setInterval(
        () =>
          bars.forEach((bar, index) => {
            bar.style.height = `${4 + Math.round(Math.random() * 18 * Math.sin(((index + 0.5) / bars.length) * Math.PI))}px`;
          }),
        100,
      );
      for (const { source } of words) {
        if (!current()) return;
        source.classList.add("in");
        await sleep(150);
      }
      await sleep(350);
      if (!current()) return;
      clearInterval(meter);
      bars.forEach((bar) => (bar.style.height = "4px"));
      pill.classList.remove("on");
      words
        .filter((word) => !word.target)
        .forEach(({ source }) => {
          animations.push(
            source.animate(
              [{ opacity: 1 }, { opacity: 0, filter: "blur(6px)", transform: "scale(.94)" }],
              { duration: 500, easing, fill: "forwards" },
            ),
          );
        });
      await sleep(550);
      if (!current()) return;
      placeholder.style.opacity = "0";
      const kept = words.filter((word) => word.target !== null);
      const positions = kept.map(({ source, target }) => ({
        source,
        target,
        a: source.getBoundingClientRect(),
        b: target.getBoundingClientRect(),
      }));
      positions.forEach(({ source, target, a, b }, index) => {
        const dx = a.left - b.left,
          dy = a.top - b.top,
          scale = a.height / b.height;
        const options: KeyframeAnimationOptions = {
          duration: 950,
          delay: index * 30,
          easing,
          fill: "forwards",
        };
        animations.push(
          source.animate(
            [
              { transform: "none", opacity: 1 },
              { transform: `translate(${-dx}px, ${-dy}px) scale(${1 / scale})`, opacity: 0 },
            ],
            options,
          ),
        );
        animations.push(
          target.animate(
            [
              { transform: `translate(${dx}px, ${dy}px) scale(${scale})`, opacity: 0 },
              { transform: "none", opacity: 1 },
            ],
            options,
          ),
        );
      });
      await sleep(950 + kept.length * 30);
      if (current()) again.hidden = false;
    };
    replay.current = () => {
      void play();
    };
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        void document.fonts.ready.then(() => {
          if (!disposed) void play();
        });
      },
      { threshold: 0.5 },
    );
    observer.observe(element);
    return () => {
      disposed = true;
      run++;
      observer.disconnect();
      stop();
      replay.current = null;
    };
  }, []);
  return (
    <div
      className="scene photo"
      id="scene"
      ref={scene}
      style={{ "--img": "url(/img/hero.jpg)" } as CSSProperties}
    >
      <div className="compose glass">
        <div className="meta">
          <span>
            To <b>Maya</b>
          </span>
          <span>Re: launch date</span>
        </div>
        <div className="body">
          <span className="ph">Write a reply</span>
          <div className="out" aria-live="off" />
        </div>
      </div>
      <div className="said" aria-hidden="true" />
      <div className="pill" aria-hidden="true">
        {Array.from({ length: 17 }, (_, index) => (
          <i key={index} />
        ))}
      </div>
      <div className="hint">
        <button hidden onClick={() => replay.current?.()}>
          Play again
        </button>
      </div>
    </div>
  );
}
