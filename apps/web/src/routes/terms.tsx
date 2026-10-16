import { createFileRoute } from "@tanstack/react-router";
export const Route = createFileRoute("/terms")({
  head: () => ({ meta: [{ title: "Terms · TypeAside" }] }),
  component: () => (
    <main className="container page legal">
      <h1>Terms.</h1>
      <h2>Using TypeAside</h2>
      <p>
        TypeAside converts speech into text and helps edit it. Review the results before relying on
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
        account. You can cancel at any time and keep Pro until the end of the paid period. If
        TypeAside did not work for you, ask for a refund within 14 days of a charge and we will
        refund it in full.
      </p>
      <h2>Processing and availability</h2>
      <p>
        Audio is sent to the transcription provider for processing and is not stored by us. History
        stays on your device. BYOK requests go directly to your provider. TypeAside requires an
        internet connection and depends on third-party providers. Availability and transcription
        accuracy are not guaranteed.
      </p>
      <h2>Your account</h2>
      <p>
        Keep your sign-in and desktop tokens secure. Revoke tokens for computers you no longer use.
        Do not share access to evade plan limits or misuse the hosted service.
      </p>
      <h2>Changes</h2>
      <p>
        We may update these terms as TypeAside changes. Material changes will be announced on this
        site before they take effect.
      </p>
      <h2>Contact</h2>
      <p>
        TypeAside is built and operated by Aditya Miskin in India. Email adityamiskin@gmail.com for
        support, billing or refund requests.
      </p>
    </main>
  ),
});
