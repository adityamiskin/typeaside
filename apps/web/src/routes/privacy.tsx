import { createFileRoute } from "@tanstack/react-router";
import { seo } from "../lib/seo";
export const Route = createFileRoute("/privacy")({
  head: () =>
    seo({
      path: "/privacy",
      title: "Privacy · TypeAside",
      description:
        "How TypeAside handles your audio, text and account data. Audio is never stored and your history stays on your device.",
    }),
  component: () => (
    <main className="container page legal">
      <h1>Privacy.</h1>
      <h2>Your voice and text</h2>
      <p>
        Audio is sent to the transcription provider for processing and is not stored by us. Text is
        sent to the provider for cleanup. Your dictation history stays on your device.
      </p>
      <h2>Hosted dictation</h2>
      <p>
        When you use Free, TypeAside forwards requests to Groq. On Pro, audio goes to Microsoft
        Azure for transcription and text goes to Groq. We keep your account email, subscription
        status, device token hashes and daily word totals and an all-time word count to provide the
        service. We do not save the audio or the content of your dictations.
      </p>
      <h2>Anonymous usage stats</h2>
      <p>
        Anonymous usage stats are opt-in and contain counts only, with no text or audio. Fields:
        random install ID, UTC day, app version, operating system, hosted or own-key source,
        dictation count, word count, command count and total latency in milliseconds.
      </p>
      <h2>Your own key</h2>
      <p>
        BYOK requests go directly from your computer to your chosen Groq or OpenAI provider. Those
        requests do not pass through TypeAside servers. Your key is stored in your operating
        system's keychain.
      </p>
      <h2>Accounts and payments</h2>
      <p>
        WorkOS handles sign-in. Dodo Payments handles payments as merchant of record, including
        payment details and applicable taxes. Their privacy policies apply to the information they
        process.
      </p>
      <h2>Your control</h2>
      <p>
        You can revoke a connected computer from your account and delete local history in the app.
        You can permanently delete your account from the account page. This cancels active
        subscriptions and removes your profile, usage and connected computers, including your WorkOS
        sign-in account. Payment records are kept as required by law.
      </p>
      <h2>Contact</h2>
      <p>
        TypeAside is operated by Aditya Miskin in India. Send privacy questions to
        adityamiskin@gmail.com.
      </p>
    </main>
  ),
});
