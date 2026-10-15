import { useEffect, useRef, useState } from "react";
const heard = "um so let's meet tomorrow at uh three, no actually four".split(" ");
const bars = [0.2, 0.3, 0.45, 0.6, 0.75, 0.9, 1, 0.9, 0.75, 0.6, 0.45, 0.3, 0.2];
export function Demo() {
  const ref = useRef<HTMLDivElement>(null);
  const [step, setStep] = useState(20);
  useEffect(() => {
    const node = ref.current;
    if (!node || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    setStep(-1);
    let timer: ReturnType<typeof setInterval> | undefined;
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        let current = 0;
        setStep(current);
        timer = setInterval(() => {
          current += 1;
          setStep(current);
          if (current >= 20) clearInterval(timer);
        }, 80);
      },
      { threshold: 0.5 },
    );
    observer.observe(node);
    return () => {
      observer.disconnect();
      clearInterval(timer);
    };
  }, []);
  const finished = step >= 20;
  const count =
    step < 0 ? 0 : finished ? heard.length : Math.ceil(((step + 1) / 20) * heard.length);
  return (
    <div ref={ref} className="hero-demo" aria-label="Hermes turns spoken words into clean text">
      <div className="overlay-pill" aria-hidden="true">
        {bars.map((height, index) => (
          <span
            key={index}
            style={{
              height:
                finished || step < 0
                  ? height * 30
                  : 4 + ((index * 7 + step * 11 + index * step * 3) % 19),
              opacity: finished || step < 0 ? 0.55 : 1,
            }}
          />
        ))}
      </div>
      <div className="demo-row">
        <span className="demo-label">Heard</span>
        <p>{heard.slice(0, count).join(" ")}</p>
      </div>
      <div className="demo-row typed">
        <span className="demo-label">Typed</span>
        <p style={{ opacity: finished ? 1 : 0 }}>Let's meet tomorrow at 4.</p>
      </div>
    </div>
  );
}
