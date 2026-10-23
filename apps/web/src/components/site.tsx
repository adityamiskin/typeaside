import { Button } from "@typeaside/ui/components/button";
import {
  Sheet,
  SheetTrigger,
  SheetContent,
  SheetTitle,
  SheetDescription,
  SheetClose,
} from "@typeaside/ui/components/sheet";
import { useEffect, useState } from "react";
import { Menu, Moon, Sun } from "lucide-react";
import { comingSoon, useOS } from "../lib/downloads";

const repo = "https://github.com/adityamiskin/typeaside";
const links = [
  { label: "Docs", href: "/docs" },
  { label: "Features", href: "/#features" },
];
export function Waveform({ size }: { size?: number } = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.2"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M4 11v2M8 8v8M12 4v16M16 8v8M20 11v2" />
    </svg>
  );
}
export function Logo({ size }: { size?: number } = {}) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M4.5 11a9 9 0 0 1 18 0Z" />
      <path d="M1.5 13a9 9 0 0 0 18 0Z" />
    </svg>
  );
}
function GitHubMark() {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}
function GitHubLink() {
  return (
    <a className="icon-button" href={repo} aria-label="TypeAside on GitHub">
      <GitHubMark />
    </a>
  );
}
export function Header() {
  const [condensed, setCondensed] = useState(false);
  useEffect(() => {
    const update = () => setCondensed(window.scrollY > 80);
    update();
    window.addEventListener("scroll", update, { passive: true });
    return () => window.removeEventListener("scroll", update);
  }, []);
  const navLinks = [...links];
  return (
    <header className={`site-header wrap ${condensed ? "condensed" : ""}`}>
      <nav className="nav" aria-label="Main navigation">
        <a href="/" className="logo">
          <Logo />
          TypeAside
        </a>
        <div className="nav-links">
          {navLinks.map((link) => (
            <a className="link" key={link.label} href={link.href}>
              {link.label}
            </a>
          ))}
        </div>
        <div className="nav-actions">
          <GitHubLink />
          <ThemeToggle />
          <a className="btn sm" href="/download">
            Download
          </a>
          <Sheet>
            <SheetTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon"
                  className="mobile-menu"
                  aria-label="Open menu"
                />
              }
            >
              <Menu />
            </SheetTrigger>
            <SheetContent className="menu-panel">
              <SheetTitle className="logo">TypeAside</SheetTitle>
              <SheetDescription className="sr-only">Site navigation</SheetDescription>
              {[...navLinks, { label: "Download", href: "/download" }].map((link) => (
                <SheetClose nativeButton={false} key={link.label} render={<a href={link.href} />}>
                  {link.label}
                </SheetClose>
              ))}
              <div className="menu-icons">
                <GitHubLink />
                <ThemeToggle />
              </div>
            </SheetContent>
          </Sheet>
        </div>
      </nav>
    </header>
  );
}
function ThemeToggle() {
  const toggle = () => {
    const dark = document.documentElement.classList.toggle("dark");
    localStorage.setItem("theme", dark ? "dark" : "light");
  };
  return (
    <button
      type="button"
      className="icon-button theme-toggle"
      aria-label="Toggle dark mode"
      onClick={toggle}
    >
      <Sun className="sun" />
      <Moon className="moon" />
    </button>
  );
}
export function DownloadButton() {
  const os = useOS();
  return (
    <a href="/download" className="btn">
      {!os ? "Download" : os in comingSoon ? `${os} coming soon` : `Download for ${os}`}
    </a>
  );
}
export function Footer() {
  return (
    <footer className="wrap">
      <nav className="footer-columns" aria-label="Footer navigation">
        <div className="footer-brand">
          <b>TypeAside</b>
          <p>
            Open-source voice dictation for Linux, macOS and Windows. Your words stay on your
            computer.
          </p>
        </div>
        <div>
          <h4>Product</h4>
          <a href="/download">Download</a>
          <a href="/#features">Features</a>
        </div>
        <div>
          <h4>Resources</h4>
          <a href="/docs">Docs</a>
          <a href={repo}>GitHub</a>
          <a href="/privacy">Privacy</a>
          <a href="/terms">License</a>
        </div>
      </nav>
      <div className="wordmark" aria-hidden="true">
        TypeAside
      </div>
    </footer>
  );
}
