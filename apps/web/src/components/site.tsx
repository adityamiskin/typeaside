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
import { Menu } from "lucide-react";
import { auth } from "../integrations/auth";
import { useOS } from "../lib/downloads";

const links = [
  { label: "Features", href: "/#features" },
  { label: "Pricing", href: "/#pricing" },
];
export function Waveform() {
  return (
    <svg
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
export function Header() {
  const { viewer } = useRouteContext({ from: "__root__" });
  const accountLink = {
    label: viewer ? "Account" : "Sign in",
    href: viewer ? "/account" : auth.signInHref("/account"),
  };
  return (
    <header className="site-header wrap">
      <nav className="nav" aria-label="Main navigation">
        <a href="/" className="logo">
          <Waveform />
          Hermes
        </a>
        {links.map((link) => (
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
          </SheetTrigger>
          <SheetContent className="menu-panel">
            <SheetTitle className="logo">Hermes</SheetTitle>
            <SheetDescription className="sr-only">Site navigation</SheetDescription>
            {[...links, { label: "Download", href: "/download" }, accountLink].map((link) => (
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
      <nav className="foot-links" aria-label="Footer navigation">
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
        <span>Made for people who think out loud</span>
      </nav>
      <div className="wordmark" aria-hidden="true">
        Hermes
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
