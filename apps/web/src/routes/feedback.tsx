import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useConvexAuth, useMutation } from "convex/react";
import { ConvexError } from "convex/values";
import type { FunctionArgs } from "convex/server";
import { api } from "@hermes/backend/convex/_generated/api";
import { Button } from "@hermes/ui/components/button";
import { Field, FieldGroup } from "@hermes/ui/components/field";
import { Input } from "@hermes/ui/components/input";
import { Label } from "@hermes/ui/components/label";
import { Textarea } from "@hermes/ui/components/textarea";
import { ToggleGroup, ToggleGroupItem } from "@hermes/ui/components/toggle-group";

export const Route = createFileRoute("/feedback")({
  validateSearch: (
    search: Record<string, unknown>,
  ): Pick<FunctionArgs<typeof api.feedback.send>, "version" | "os"> => ({
    version:
      typeof search.version === "string" &&
      search.version.trim().length > 0 &&
      search.version.trim().length <= 64
        ? search.version.trim()
        : undefined,
    os:
      search.os === "linux" || search.os === "macos" || search.os === "windows"
        ? search.os
        : undefined,
  }),
  head: () => ({ meta: [{ title: "Feedback · Hermes" }] }),
  component: Feedback,
});

function Feedback() {
  const { version, os } = Route.useSearch();
  const { isAuthenticated, isLoading } = useConvexAuth();
  const send = useMutation(api.feedback.send);
  const [kind, setKind] = useState<FunctionArgs<typeof api.feedback.send>["kind"]>("idea");
  const [message, setMessage] = useState("");
  const [email, setEmail] = useState("");
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <main className="container page">
      <h1>Tell us what to build.</h1>
      <p className="lead">Bugs, ideas, anything. We read every note.</p>
      {sent ? (
        <div className="feedback-form" role="status">
          <p>Thanks. We got it.</p>
          <a
            href="#"
            className="text-button"
            onClick={(event) => {
              event.preventDefault();
              setSent(false);
              setMessage("");
              setError(null);
            }}
          >
            Send another
          </a>
        </div>
      ) : (
        <form
          className="feedback-form"
          onSubmit={async (event) => {
            event.preventDefault();
            if (sending) return;
            setSending(true);
            setError(null);
            try {
              await send({
                kind,
                message,
                email: isAuthenticated ? undefined : email.trim() || undefined,
                version,
                os,
              });
              setSent(true);
            } catch (cause) {
              setError(
                cause instanceof ConvexError && typeof cause.data === "string"
                  ? cause.data
                  : "Could not send your note. Please try again.",
              );
            } finally {
              setSending(false);
            }
          }}
        >
          <FieldGroup>
            <Field>
              <Label id="feedback-kind" className="sr-only">
                Kind of feedback
              </Label>
              <ToggleGroup
                aria-labelledby="feedback-kind"
                value={[kind]}
                variant="outline"
                spacing={0}
                disabled={sending}
                onValueChange={(values) => {
                  const value = values[0];
                  if (value === "bug" || value === "idea" || value === "other") setKind(value);
                }}
              >
                <ToggleGroupItem value="bug">Bug</ToggleGroupItem>
                <ToggleGroupItem value="idea">Idea</ToggleGroupItem>
                <ToggleGroupItem value="other">Other</ToggleGroupItem>
              </ToggleGroup>
            </Field>
            <Field>
              <Label htmlFor="feedback-message" className="sr-only">
                Your note
              </Label>
              <Textarea
                id="feedback-message"
                rows={6}
                placeholder="What happened, or what would help?"
                required
                maxLength={4000}
                value={message}
                disabled={sending}
                onChange={(event) => setMessage(event.target.value)}
              />
            </Field>
            {!isAuthenticated && !isLoading && (
              <Field>
                <Label htmlFor="feedback-email">Email, if you want a reply</Label>
                <Input
                  id="feedback-email"
                  type="email"
                  autoComplete="email"
                  maxLength={254}
                  value={email}
                  disabled={sending}
                  onChange={(event) => setEmail(event.target.value)}
                />
              </Field>
            )}
            <div>
              <Button type="submit" disabled={sending || isLoading || !message.trim()}>
                Send
              </Button>
              {error && (
                <p className="error" role="alert">
                  {error}
                </p>
              )}
            </div>
          </FieldGroup>
        </form>
      )}
    </main>
  );
}
