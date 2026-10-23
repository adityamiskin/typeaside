import { createFileRoute } from "@tanstack/react-router";
import { seo } from "../lib/seo";
export const Route = createFileRoute("/privacy")({
  head: () =>
    seo({
      path: "/privacy",
      title: "Privacy · TypeAside",
      description:
        "TypeAside is a local app. Your audio, text and history stay on your computer unless you connect your own transcription provider.",
    }),
  component: () => (
    <main className="container page legal">
      <h1>Privacy.</h1>
      <h2>Local by default</h2>
      <p>
        TypeAside runs on your computer. Your audio, transcripts, notes, dictionary, snippets and
        dictation history are stored locally and never sent to us. There are no accounts and no
        telemetry.
      </p>
      <h2>Your own provider</h2>
      <p>
        If you configure a cloud transcription or editing provider, audio and text are sent directly
        from your computer to that provider. Those requests do not pass through any TypeAside
        server, and we never see them. Provider keys are stored in your operating system's keychain.
      </p>
      <h2>Model downloads</h2>
      <p>
        On-device speech models are downloaded from typeaside.app over HTTPS. Downloading a model
        does not send us your dictations.
      </p>
      <h2>Contact</h2>
      <p>
        TypeAside is an open-source project maintained by Aditya Miskin. Send privacy questions to
        adityamiskin@gmail.com.
      </p>
    </main>
  ),
});
