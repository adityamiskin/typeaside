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
              <ThemeToggle />
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
    <button type="button" className="theme-toggle" aria-label="Toggle dark mode" onClick={toggle}>
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
