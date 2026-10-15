import { useEffect, useRef, type ComponentProps } from "react";

export function Reveal({ children, className = "", ...props }: ComponentProps<"section">) {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    const element = ref.current;
    if (!element || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    element.classList.add("reveal-ready");
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          element.classList.add("revealed");
          observer.disconnect();
        }
      },
      { threshold: 0.08 },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return (
    <section ref={ref} className={`section ${className}`} {...props}>
      {children}
    </section>
  );
}
