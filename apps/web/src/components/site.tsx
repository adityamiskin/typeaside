import { useRouteContext, useRouterState } from "@tanstack/react-router";
import { Button, buttonVariants } from "@typeaside/ui/components/button";
import {
  Sheet,
  SheetTrigger,
  SheetContent,
  SheetTitle,
  SheetDescription,
  SheetClose,
} from "@typeaside/ui/components/sheet";
import { useEffect, useState } from "react";
import { Menu } from "lucide-react";
import { auth } from "../integrations/auth";
import { useOS } from "../lib/downloads";

const links = [
  { label: "Docs", href: "/docs" },
  { label: "Features", href: "/#features" },
  { label: "Pricing", href: "/#pricing" },
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
  const isNotes = useRouterState({ select: (state) => state.location.pathname === "/notes" });
  const [condensed, setCondensed] = useState(false);
  useEffect(() => {
    const update = () => setCondensed(window.scrollY > 80);
    update();
    window.addEventListener("scroll", update, { passive: true });
    return () => window.removeEventListener("scroll", update);
  }, []);
  const { viewer } = useRouteContext({ from: "__root__" });
  const accountLink = {
    label: viewer ? "Account" : "Sign in",
    href: viewer ? "/account" : auth.signInHref("/account"),
  };
  const navLinks = [
    ...links.slice(0, 2),
    { label: "Notes", href: viewer ? "/notes" : "/features/notes" },
    ...links.slice(2),
  ];
  return (
    <header className={`site-header wrap ${condensed ? "condensed" : ""}`} data-notes={isNotes}>
      <nav className="nav" aria-label="Main navigation">
        <a href="/" className="logo">
          <Logo />
          TypeAside
        </a>
        {navLinks.map((link) => (
          <a className="link opt" key={link.label} href={link.href}>
            {link.label}
          </a>
        ))}
        <a className="link" href={accountLink.href}>
          {accountLink.label}
        </a>
        <a className="btn sm" href="/download">
          Download
        </a>
        <Sheet>
          <SheetTrigger
            render={
              <Button variant="ghost" size="icon" className="mobile-menu" aria-label="Open menu" />
            }
          >
            <Menu />
            {isNotes && <span className="notes-menu-label">Menu</span>}
          </SheetTrigger>
          <SheetContent className="menu-panel">
            <SheetTitle className="logo">TypeAside</SheetTitle>
            <SheetDescription className="sr-only">Site navigation</SheetDescription>
            {[...navLinks, { label: "Download", href: "/download" }, accountLink].map((link) => (
              <SheetClose nativeButton={false} key={link.label} render={<a href={link.href} />}>
                {link.label}
              </SheetClose>
            ))}
          </SheetContent>
        </Sheet>
      </nav>
    </header>
  );
}
export function DownloadButton() {
  const os = useOS();
  return (
    <a href="/download" className="btn">
      {os ? `Download for ${os}` : "Download"}
    </a>
  );
}
export function Footer() {
  return (
    <footer className="wrap">
      <nav className="footer-columns" aria-label="Footer navigation">
        <div className="footer-brand">
          <b>TypeAside</b>
          <p>Voice dictation and quick notes for Linux, macOS and Windows.</p>
        </div>
        <div>
          <h4>Product</h4>
          <a href="/download">Download</a>
          <a href="/features/notes">Notes</a>
          <a href="/#pricing">Pricing</a>
        </div>
        <div>
          <h4>Resources</h4>
          <a href="/docs">Docs</a>
          <a href="/feedback">Feedback</a>
          <a href="/privacy">Privacy</a>
          <a href="/terms">Terms</a>
        </div>
      </nav>
      <div className="wordmark" aria-hidden="true">
        TypeAside
      </div>
    </footer>
  );
}
export function SignIn({ returnTo = "/account" }: { returnTo?: string }) {
  return (
    <main className="container page narrow">
      <h1>Sign in to TypeAside.</h1>
      <p className="lead">Your plan, usage and connected computers.</p>
      <a className={buttonVariants()} href={auth.signInHref(returnTo)}>
        Sign in
      </a>
    </main>
  );
}
