import { useEffect, useState } from "react";
import { useRouteContext } from "@tanstack/react-router";
import { Button, buttonVariants } from "@hermes/ui/components/button";
import {
  Sheet,
  SheetTrigger,
  SheetContent,
  SheetTitle,
  SheetDescription,
  SheetClose,
} from "@hermes/ui/components/sheet";
import { Menu, Moon, Sun } from "lucide-react";
import { auth } from "../integrations/auth";
import { useOS } from "../lib/downloads";

const links = [
  { label: "Features", href: "/#features" },
  { label: "Pricing", href: "/#pricing" },
  { label: "Download", href: "/download" },
];
export function Header() {
  const { viewer } = useRouteContext({ from: "__root__" });
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const scroll = () => setScrolled(window.scrollY > 0);
    scroll();
    window.addEventListener("scroll", scroll, { passive: true });
    return () => window.removeEventListener("scroll", scroll);
  }, []);
  return (
    <header className={`site-header${scrolled ? " scrolled" : ""}`}>
      <nav className="container nav" aria-label="Main navigation">
        <a href="/" className="wordmark">
          Hermes
        </a>
        <div className="nav-center">
          {links.map((link) => (
            <a key={link.label} href={link.href}>
              {link.label}
            </a>
          ))}
        </div>
        <div className="nav-right">
          <a className="signin-link" href={viewer ? "/account" : auth.signInHref("/account")}>
            {viewer ? "Account" : "Sign in"}
          </a>
          <a className={buttonVariants()} href="/download">
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
              <SheetTitle className="wordmark">Hermes</SheetTitle>
              <SheetDescription className="sr-only">Site navigation</SheetDescription>
              {[
                ...links,
                {
                  label: viewer ? "Account" : "Sign in",
                  href: viewer ? "/account" : auth.signInHref("/account"),
                },
              ].map((link) => (
                <SheetClose nativeButton={false} key={link.label} render={<a href={link.href} />}>
                  {link.label}
                </SheetClose>
              ))}
            </SheetContent>
          </Sheet>
        </div>
      </nav>
    </header>
  );
}
export function DownloadButton() {
  const os = useOS();
  return (
    <a href="/download" className={buttonVariants()}>
      {os ? `Download for ${os}` : "Download"}
    </a>
  );
}
export function Footer() {
  const [theme, setTheme] = useState("dark");
  useEffect(() => {
    setTheme(document.documentElement.classList.contains("light") ? "light" : "dark");
  }, []);
  function toggle() {
    const next = theme === "dark" ? "light" : "dark";
    document.documentElement.classList.remove("dark", "light");
    document.documentElement.classList.add(next);
    try {
      localStorage.setItem("hermes-theme", next);
    } catch {
      /* Theme still applies when storage is unavailable. */
    }
    setTheme(next);
  }
  return (
    <footer className="site-footer">
      <div className="container footer-row">
        <a href="/" className="wordmark">
          Hermes
        </a>
        <nav aria-label="Footer navigation">
          {[
            { label: "Download", href: "/download" },
            { label: "Pricing", href: "/#pricing" },
            { label: "Account", href: "/account" },
            { label: "Privacy", href: "/privacy" },
            { label: "Terms", href: "/terms" },
          ].map((link) => (
            <a key={link.label} href={link.href}>
              {link.label}
            </a>
          ))}
        </nav>
        <Button
          variant="ghost"
          size="icon"
          onClick={toggle}
          aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
        >
          {theme === "dark" ? <Sun /> : <Moon />}
        </Button>
        <small>© 2026 Hermes</small>
      </div>
    </footer>
  );
}
export function SignIn({ returnTo = "/account" }: { returnTo?: string }) {
  return (
    <main className="container page narrow">
      <h1>Sign in to Hermes.</h1>
      <p className="lead">Your plan, usage and connected computers.</p>
      <a className={buttonVariants()} href={auth.signInHref(returnTo)}>
        Sign in
      </a>
    </main>
  );
}
