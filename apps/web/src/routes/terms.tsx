import { createFileRoute } from "@tanstack/react-router";
export const Route = createFileRoute("/terms")({
  head: () => ({ meta: [{ title: "Terms · Hermes" }] }),
  component: () => (
    <main className="container page legal">
      <p className="draft">Draft, pending legal review.</p>
      <h1>Terms.</h1>
      <h2>Using Hermes</h2>
      <p>
        Hermes converts speech into text and helps edit it. Review the results before relying on
        them. You are responsible for your content and for using the service lawfully.
      </p>
      <h2>Plans</h2>
      <p>
        Free includes 2,000 hosted words each week and requires an account. The allowance resets on
        Monday at 00:00 UTC. Pro includes unlimited hosted words for $12 a month or $96 a year. BYOK
        is free to use without an account, with provider charges billed to you directly.
      </p>
      <h2>Billing</h2>
      <p>
        Dodo Payments is the merchant of record and handles payments and applicable taxes. Pro
        renews at your selected interval until cancelled. Manage your subscription through your
        account. Cancellation and refund details will be finalized before launch.
      </p>
      <h2>Processing and availability</h2>
      <p>
        Audio is sent to the transcription provider for processing and is not stored by us. History
        stays on your device. BYOK requests go directly to your provider. Hermes requires an
        internet connection and depends on third-party providers. Availability and transcription
        accuracy are not guaranteed.
      </p>
      <h2>Your account</h2>
      <p>
        Keep your sign-in and desktop tokens secure. Revoke tokens for computers you no longer use.
        Do not share access to evade plan limits or misuse the hosted service.
      </p>
      <h2>Before launch</h2>
      <p>
        This draft is not the final agreement. Business identity, contact details, refund terms and
        other required legal provisions are pending legal review.
      </p>
    </main>
  ),
});
