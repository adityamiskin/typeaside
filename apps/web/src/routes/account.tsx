import { useEffect, useRef, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import {
  useAction,
  useMutation,
  useQuery,
  Authenticated,
  AuthLoading,
  Unauthenticated,
} from "convex/react";
import { api } from "@typeaside/backend/convex/_generated/api";
import { noindex } from "../lib/seo";
import type { FunctionReturnType } from "convex/server";
import { Field, FieldGroup, FieldLabel } from "@typeaside/ui/components/field";
import { Input } from "@typeaside/ui/components/input";
import { Button } from "@typeaside/ui/components/button";
import { SignIn } from "../components/site";
import { BillingToggle, CheckoutButton, type Interval } from "../components/billing";
import { auth } from "../integrations/auth";
export const Route = createFileRoute("/account")({
  validateSearch: (search: Record<string, unknown>) => ({
    upgrade: search.upgrade === "1" ? "1" : undefined,
    checkout:
      search.checkout === "monthly" || search.checkout === "yearly" ? search.checkout : undefined,
  }),
  head: () => ({ meta: [{ title: "Account · TypeAside" }, ...noindex.meta] }),
  component: Account,
});
function Account() {
  const { viewer } = Route.useRouteContext();
  if (!viewer) return <SignIn />;
  return (
    <>
      <Authenticated>
        <AccountContent />
      </Authenticated>
      <AuthLoading>
        <main className="container page">
          <p>Loading your account…</p>
        </main>
      </AuthLoading>
      <Unauthenticated>
        <SignIn />
      </Unauthenticated>
    </>
  );
}
function relativeTime(value: number | undefined) {
  if (value === undefined) return "Not used yet";
  const minutes = Math.max(0, Math.floor((Date.now() - value) / 60000));
  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes} ${minutes === 1 ? "minute" : "minutes"} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} ${hours === 1 ? "hour" : "hours"} ago`;
  const days = Math.floor(hours / 24);
  return `${days} ${days === 1 ? "day" : "days"} ago`;
}
function AccountContent() {
  const { checkout, upgrade } = Route.useSearch();
  const [interval, setInterval] = useState<Interval>(checkout === "yearly" ? "yearly" : "monthly");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const account = useQuery(api.viewer.current);
  const devices = useQuery(api.desktop.list);
  const sync = useAction(api.viewer.sync);
  const revoke = useMutation(api.desktop.revoke);
  const portal = useAction(api.billing.portal);
  const createCheckout = useAction(api.billing.checkout);
  const checkoutStarted = useRef(false);
  useEffect(() => {
    void sync({}).catch(() => setError("Your account could not load. Please refresh."));
  }, [sync]);
  useEffect(() => {
    const requested = checkout === "monthly" || checkout === "yearly";
    if ((!requested && !(upgrade && account?.plan === "free")) || checkoutStarted.current) return;
    checkoutStarted.current = true;
    window.history.replaceState(window.history.state, "", "/account");
    void createCheckout({ interval: checkout === "yearly" ? "yearly" : "monthly" })
      .then((result) => window.location.assign(result.url))
      .catch(() => setError("Checkout is unavailable. Please try again."));
  }, [checkout, upgrade, account?.plan, createCheckout]);
  return (
    <main className="container page account">
      <h1>Your account.</h1>

      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <section className="account-row">
        <h2>Profile</h2>
        <div>
          {account ? (
            <Profile name={account.name} email={account.email} />
          ) : (
            <p>Loading profile…</p>
          )}
        </div>
      </section>
      <section className="account-row">
        <h2>Plan</h2>
        <div>
          {account ? (
            <>
              {account.billingIssue && (
                <p className="error">Payment failed. Update your card to keep Pro.</p>
              )}
              <h3>{account.plan === "pro" ? "Pro" : "Free"}</h3>
              {account.currentPeriodEnd && (
                <p>
                  Renews{" "}
                  {new Date(account.currentPeriodEnd).toLocaleDateString(undefined, {
                    year: "numeric",
                    month: "long",
                    day: "numeric",
                  })}
                </p>
              )}
              {account.plan === "pro" || account.billingIssue ? (
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={async () => {
                    setBusy(true);
                    try {
                      const result = await portal({});
                      window.location.assign(result.portal_url);
                    } catch {
                      setError("Billing is unavailable. Please try again.");
                      setBusy(false);
                    }
                  }}
                >
                  {account.billingIssue ? "Fix billing" : "Manage billing"}
                </Button>
              ) : (
                <div className="upgrade">
                  <BillingToggle interval={interval} setInterval={setInterval} />
                  <CheckoutButton interval={interval}>Upgrade to Pro</CheckoutButton>
                </div>
              )}
            </>
          ) : (
            <p>Loading plan…</p>
          )}
        </div>
      </section>
      <section className="account-row">
        <h2>Usage</h2>
        <div>
          {account ? (
            account.usage.cap === null ? (
              <p>Unlimited</p>
            ) : (
              <>
                <p className="usage-number">
                  {account.usage.words.toLocaleString()} of {account.usage.cap.toLocaleString()}{" "}
                  words
                </p>
                <div
                  className="usage-track"
                  role="progressbar"
                  aria-label="Weekly word usage"
                  aria-valuenow={Math.min(account.usage.words, account.usage.cap)}
                  aria-valuemin={0}
                  aria-valuemax={account.usage.cap}
                >
                  <span
                    style={{
                      width: `${Math.min(100, (account.usage.words / account.usage.cap) * 100)}%`,
                    }}
                  />
                </div>
                <p className="small">
                  Resets{" "}
                  {new Date(account.usage.resetsAt).toLocaleDateString(undefined, {
                    month: "long",
                    day: "numeric",
                    timeZone: "UTC",
                  })}{" "}
                  at 00:00 UTC
                </p>
              </>
            )
          ) : (
            <p>Loading usage…</p>
          )}
          <UsageHistory />
        </div>
      </section>
      <section className="account-row">
        <h2>Invoices</h2>
        <Invoices />
      </section>
      <section className="account-row">
        <h2>Devices</h2>
        <div>
          {devices?.length === 0 && <p>No connected computers.</p>}
          {devices?.map((device) => (
            <div key={device._id} className="device-row">
              <div>
                <h3>{device.name}</h3>
                <p>{relativeTime(device.lastUsedAt)}</p>
              </div>
              <button
                type="button"
                className="text-button"
                onClick={async () => {
                  try {
                    await revoke({ id: device._id });
                  } catch {
                    setError("Device could not be revoked. Please try again.");
                  }
                }}
              >
                Revoke
              </button>
            </div>
          ))}
        </div>
      </section>
      <section className="account-row">
        <h2>Delete account</h2>
        <DeleteAccount email={account?.email ?? null} />
      </section>
      <div className="account-signout">
        <a href={auth.signOutHref}>Sign out</a>
      </div>
    </main>
  );
}

function Profile({ name, email }: { name: string; email: string | null }) {
  const rename = useMutation(api.viewer.rename);
  const [draft, setDraft] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const value = draft ?? name;
  return (
    <>
      <p>{email}</p>
      <form
        className="account-form"
        onSubmit={async (event) => {
          event.preventDefault();
          setSaving(true);
          setSaved(false);
          setError(null);
          try {
            await rename({ name: value.trim() });
            setDraft(null);
            setSaved(true);
          } catch {
            setError("Your name could not be saved. Please try again.");
          } finally {
            setSaving(false);
          }
        }}
      >
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="profile-name">Name</FieldLabel>
            <Input
              id="profile-name"
              autoComplete="name"
              maxLength={80}
              value={value}
              disabled={saving}
              onChange={(event) => {
                setDraft(event.target.value);
                setSaved(false);
              }}
            />
          </Field>
        </FieldGroup>
        <Button
          type="submit"
          disabled={saving || !value.trim() || value.trim() === name || value.trim().length > 80}
        >
          {saving ? "Saving…" : "Save"}
        </Button>
        {saved && (
          <p className="small" role="status">
            Saved
          </p>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
      </form>
    </>
  );
}
function UsageHistory() {
  const history = useQuery(api.viewer.history);
  if (!history) return <p>Loading history…</p>;
  const max = Math.max(1, ...history.days.map((day) => day.words));
  const minutes = history.totalWords / 40 - history.totalWords / 150;
  const saved =
    minutes >= 60 ? `${(minutes / 60).toFixed(1)} hours` : `${minutes.toFixed(1)} minutes`;
  return (
    <>
      <div
        className="usage-history"
        role="img"
        aria-label={`Daily usage over the last 30 days, ${history.recentWords.toLocaleString()} words. Today is on the right. Dates are UTC.`}
      >
        {history.days.map(({ day, words }) => (
          <div
            key={day}
            className="usage-day"
            style={{ height: `${(words / max) * 100}%` }}
            title={`${new Date(day).toLocaleDateString(undefined, { month: "short", day: "numeric", timeZone: "UTC" })}: ${words.toLocaleString()} words`}
          />
        ))}
      </div>
      <p>
        {history.recentWords.toLocaleString()} words in the last 30 days,{" "}
        {history.totalWords.toLocaleString()} all time, about {saved} saved.
      </p>
      <p className="small">Compared with typing at 40 words per minute.</p>
    </>
  );
}
function Invoices() {
  const list = useAction(api.billing.invoices);
  const download = useAction(api.billing.invoice);
  const [invoices, setInvoices] = useState<FunctionReturnType<typeof api.billing.invoices>>();
  const [error, setError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void list({})
      .then((result) => {
        if (active) setInvoices(result);
      })
      .catch(() => {
        if (active) setError("Invoices could not load. Please refresh.");
      });
    return () => {
      active = false;
    };
  }, [list]);
  return (
    <div>
      {invoices === undefined && !error && <p>Loading invoices…</p>}
      {invoices?.length === 0 && <p>No invoices yet.</p>}
      {invoices?.map((invoice) => {
        const date = new Date(invoice.date).toISOString().slice(0, 10);
        const currency = new Intl.NumberFormat(undefined, {
          style: "currency",
          currency: invoice.currency,
        });
        const amount =
          invoice.amount / 10 ** (currency.resolvedOptions().maximumFractionDigits ?? 2);
        return (
          <div className="invoice-row" key={invoice.paymentId}>
            <time dateTime={invoice.date}>
              {new Date(invoice.date).toLocaleDateString(undefined, {
                year: "numeric",
                month: "short",
                day: "numeric",
                timeZone: "UTC",
              })}
            </time>
            <span>{currency.format(amount)}</span>
            <span>{invoice.status.replaceAll("_", " ")}</span>
            <button
              type="button"
              className="text-button"
              disabled={downloading !== null}
              onClick={async () => {
                setDownloading(invoice.paymentId);
                setError(null);
                try {
                  const bytes = await download({ paymentId: invoice.paymentId });
                  const url = URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
                  const link = document.createElement("a");
                  link.href = url;
                  link.download = `typeaside-invoice-${date}.pdf`;
                  document.body.append(link);
                  link.click();
                  link.remove();
                  setTimeout(() => URL.revokeObjectURL(url), 1000);
                } catch {
                  setError("Invoice could not download. Please try again.");
                } finally {
                  setDownloading(null);
                }
              }}
            >
              {downloading === invoice.paymentId ? "Downloading…" : "Download"}
            </button>
          </div>
        );
      })}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
function DeleteAccount({ email }: { email: string | null }) {
  const remove = useAction(api.viewer.remove);
  const [confirming, setConfirming] = useState(false);
  const [confirmation, setConfirmation] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <div>
      <p>
        Permanently delete your account, usage and connected computers. Any active subscription is
        cancelled. Payment records are kept as required by law.
      </p>
      {confirming ? (
        <form
          className="account-form"
          onSubmit={async (event) => {
            event.preventDefault();
            if (!email || confirmation !== email) return;
            setDeleting(true);
            setError(null);
            try {
              await remove({ email: confirmation });
              window.location.assign(auth.signOutHref);
            } catch {
              setError("Account could not be deleted. Please try again.");
              setDeleting(false);
            }
          }}
        >
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="delete-email">Type your email to confirm</FieldLabel>
              <Input
                id="delete-email"
                type="email"
                autoComplete="off"
                value={confirmation}
                disabled={deleting}
                onChange={(event) => setConfirmation(event.target.value)}
              />
            </Field>
          </FieldGroup>
          <div className="account-actions">
            <Button
              type="submit"
              variant="destructive"
              disabled={deleting || !email || confirmation !== email}
            >
              {deleting ? "Deleting…" : "Delete permanently"}
            </Button>
            <Button
              type="button"
              variant="outline"
              disabled={deleting}
              onClick={() => {
                setConfirming(false);
                setConfirmation("");
                setError(null);
              }}
            >
              Cancel
            </Button>
          </div>
        </form>
      ) : (
        <Button variant="outline" disabled={!email} onClick={() => setConfirming(true)}>
          Delete account
        </Button>
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
