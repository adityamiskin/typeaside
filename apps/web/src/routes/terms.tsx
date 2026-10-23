import { createFileRoute } from "@tanstack/react-router";
import { seo } from "../lib/seo";
export const Route = createFileRoute("/terms")({
  head: () =>
    seo({
      path: "/terms",
      title: "License · TypeAside",
      description: "TypeAside is open source under the MIT license, with no warranty.",
    }),
  component: () => (
    <main className="container page legal">
      <h1>License.</h1>
      <h2>MIT</h2>
      <p>
        TypeAside is free and open-source software released under the MIT license. You can use,
        copy, modify and distribute it, including commercially, subject to the license terms.
      </p>
      <h2>No warranty</h2>
      <p>
        TypeAside is provided "as is", without warranty of any kind. The authors are not liable for
        any claim, damages or other liability arising from its use. Transcription output can be
        wrong; review it before relying on it.
      </p>
      <h2>Third-party services</h2>
      <p>
        If you connect TypeAside to a cloud provider with your own API key, that provider's terms
        and privacy policy apply to your use of it.
      </p>
      <h2>Contact</h2>
      <p>TypeAside is maintained by Aditya Miskin. Email adityamiskin@gmail.com with questions.</p>
    </main>
  ),
});
