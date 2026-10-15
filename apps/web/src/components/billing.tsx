import { useState } from "react";
import { useAction } from "convex/react";
import { useRouteContext } from "@tanstack/react-router";
import { api } from "@hermes/backend/convex/_generated/api";
import { Button, buttonVariants } from "@hermes/ui/components/button";
import { auth } from "../integrations/auth";
export type Interval = "monthly" | "yearly";
export function BillingToggle({
  interval,
  setInterval,
}: {
  interval: Interval;
  setInterval: (value: Interval) => void;
}) {
  return (
    <div className="billing-toggle" role="group" aria-label="Billing interval">
      <button
        type="button"
        aria-pressed={interval === "monthly"}
        onClick={() => setInterval("monthly")}
      >
        Monthly
      </button>
      <button
        type="button"
        aria-pressed={interval === "yearly"}
        onClick={() => setInterval("yearly")}
      >
        Yearly
      </button>
      <span>Save 33%</span>
    </div>
  );
}
export function CheckoutButton({
  interval,
  children = "Get Pro",
  full = false,
}: {
  interval: Interval;
  children?: React.ReactNode;
  full?: boolean;
}) {
  const { viewer } = useRouteContext({ from: "__root__" });
  const checkout = useAction(api.billing.checkout);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (!viewer)
    return (
      <a
        className={buttonVariants({ className: full ? "w-full" : "" })}
        href={auth.signInHref(`/account?checkout=${interval}`)}
      >
        {children}
      </a>
    );
  return (
    <>
      <Button
        className={full ? "w-full" : ""}
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          setError(null);
          try {
            const result = await checkout({ interval });
            window.location.assign(result.checkout_url);
          } catch {
            setError("Checkout is unavailable. Please try again.");
            setBusy(false);
          }
        }}
      >
        {busy ? "Opening checkout…" : children}
      </Button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </>
  );
}
