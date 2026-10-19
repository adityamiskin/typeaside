import { useState } from "react";
import { useAction } from "convex/react";
import { useRouteContext } from "@tanstack/react-router";
import { api } from "@typeaside/backend/convex/_generated/api";
import { Button, buttonVariants } from "@typeaside/ui/components/button";
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
    <div
      className="period"
      data-year={interval === "yearly" ? "" : undefined}
      role="group"
      aria-label="Billing period"
    >
      <span className="thumb" aria-hidden="true" />
      <button
        type="button"
        data-p="month"
        aria-pressed={interval === "monthly"}
        onClick={() => setInterval("monthly")}
      >
        Monthly
      </button>
      <button
        type="button"
        data-p="year"
        aria-pressed={interval === "yearly"}
        onClick={() => setInterval("yearly")}
      >
        Yearly
      </button>
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
            window.location.assign(result.url);
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
