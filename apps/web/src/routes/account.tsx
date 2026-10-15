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
import { api } from "@hermes/backend/convex/_generated/api";
import { Button } from "@hermes/ui/components/button";
import { SignIn } from "../components/site";
import { BillingToggle, CheckoutButton, type Interval } from "../components/billing";
import { auth } from "../integrations/auth";
export const Route = createFileRoute("/account")({
  validateSearch: (search: Record<string, unknown>) => ({
    checkout:
      search.checkout === "monthly" || search.checkout === "yearly" ? search.checkout : undefined,
  }),
  head: () => ({ meta: [{ title: "Account · Hermes" }] }),
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
  const { checkout } = Route.useSearch();
  const [interval, setInterval] = useState<Interval>(checkout === "yearly" ? "yearly" : "monthly");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const account = useQuery(api.viewer.current);
  const devices = useQuery(api.desktop.list);
  const sync = useMutation(api.viewer.sync);
  const revoke = useMutation(api.desktop.revoke);
  const portal = useAction(api.billing.portal);
  const createCheckout = useAction(api.billing.checkout);
  const checkoutStarted = useRef(false);
  useEffect(() => {
    void sync({}).catch(() => setError("Your account could not load. Please refresh."));
  }, [sync]);
  useEffect(() => {
    if ((checkout !== "monthly" && checkout !== "yearly") || checkoutStarted.current) return;
    checkoutStarted.current = true;
    window.history.replaceState(window.history.state, "", "/account");
    void createCheckout({ interval: checkout })
      .then((result) => window.location.assign(result.checkout_url))
      .catch(() => setError("Checkout is unavailable. Please try again."));
  }, [checkout, createCheckout]);
  return (
    <main className="container page account">
      <h1>Your account.</h1>
      {account && <p className="lead">{account.email}</p>}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <section className="account-row">
        <h2>Plan</h2>
        <div>
          {account ? (
            <>
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
              {account.plan === "pro" ? (
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
                  Manage billing
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
        <h2>Usage this week</h2>
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
        </div>
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
      <div className="account-signout">
        <a href={auth.signOutHref}>Sign out</a>
      </div>
    </main>
  );
}
