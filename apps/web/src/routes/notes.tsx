import { useRef, useState, type KeyboardEvent } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Authenticated, AuthLoading, Unauthenticated, useMutation, useQuery } from "convex/react";
import type { FunctionReturnType } from "convex/server";
import { ConvexError } from "convex/values";
import { api } from "@typeaside/backend/convex/_generated/api";
import { Button } from "@typeaside/ui/components/button";
import { Input } from "@typeaside/ui/components/input";
import { Textarea } from "@typeaside/ui/components/textarea";
import { Empty, EmptyTitle, EmptyDescription } from "@typeaside/ui/components/empty";
import { Copy, Pin, Search, Trash2 } from "lucide-react";
import { SignIn } from "../components/site";
import { noindex } from "../lib/seo";

export const Route = createFileRoute("/notes")({
  head: () => ({ meta: [{ title: "Notes · TypeAside" }, ...noindex.meta] }),
  component: Notes,
});

type Note = FunctionReturnType<typeof api.notes.list>[number];

function Notes() {
  const { viewer } = Route.useRouteContext();
  if (!viewer) return <SignIn returnTo="/notes" />;
  return (
    <>
      <Authenticated>
        <NotesContent />
      </Authenticated>
      <AuthLoading>
        <main className="notes-page">
          <p>Loading…</p>
        </main>
      </AuthLoading>
      <Unauthenticated>
        <SignIn returnTo="/notes" />
      </Unauthenticated>
    </>
  );
}

function dayLabel(timestamp: number) {
  const date = new Date(timestamp);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return "Today";
  if (date.toDateString() === yesterday.toDateString()) return "Yesterday";
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    ...(date.getFullYear() !== today.getFullYear() ? { year: "numeric" } : {}),
  });
}

function submitOnEnter(event: KeyboardEvent<HTMLTextAreaElement>, save: () => void) {
  if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
    event.preventDefault();
    save();
  }
}

function KeyHints() {
  return (
    <div className="notes-hints">
      <span>
        <kbd>Enter</kbd> Save
      </span>
      <span>
        <kbd>Shift</kbd>
        <kbd>Enter</kbd> New line
      </span>
    </div>
  );
}

function NotesContent() {
  const notes = useQuery(api.notes.list);
  const save = useMutation(api.notes.save);
  const [search, setSearch] = useState("");
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const notificationTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const submitting = useRef(false);
  function notify(text: string) {
    clearTimeout(notificationTimer.current);
    setMessage(text);
    notificationTimer.current = setTimeout(() => setMessage(""), 2500);
  }
  function reportError(cause: unknown) {
    setError(
      cause instanceof ConvexError && typeof cause.data === "string"
        ? cause.data
        : "Your note could not be saved. Please try again.",
    );
  }
  async function createNote() {
    if (!draft.trim() || submitting.current) return;
    submitting.current = true;
    setBusy(true);
    setError("");
    try {
      await save({ id: crypto.randomUUID(), text: draft, pinned: false });
      setDraft("");
    } catch (cause) {
      reportError(cause);
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  }
  const groups = new Map<string, Note[]>();
  const matching =
    notes?.filter((note) => note.text.toLocaleLowerCase().includes(search.toLocaleLowerCase())) ??
    [];
  const pinned = matching.filter((note) => note.pinned);
  if (pinned.length) groups.set("Pinned", pinned);
  for (const note of matching) {
    if (note.pinned) continue;
    const label = dayLabel(note.createdAt);
    const group = groups.get(label);
    if (group) group.push(note);
    else groups.set(label, [note]);
  }
  return (
    <main className="notes-page">
      <div className="notes-heading">
        <h1>Notes</h1>
        <span className="notes-count">{notes?.length ?? 0}</span>
        <label className="notes-search">
          <Search aria-hidden="true" />
          <Input
            aria-label="Search notes"
            placeholder="Search notes"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
      </div>
      <div className="notes-composer">
        <Textarea
          aria-label="New note"
          placeholder="Write a note…"
          rows={1}
          maxLength={20_000}
          value={draft}
          readOnly={busy}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => submitOnEnter(event, () => void createNote())}
        />
        <div className="notes-composer-footer" data-has-text={!!draft.trim()}>
          <KeyHints />
          {draft.trim() && (
            <Button className="notes-save" disabled={busy} onClick={() => void createNote()}>
              Save
            </Button>
          )}
        </div>
      </div>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {notes === undefined && <p>Loading…</p>}
      {notes?.length === 0 && (
        <Empty className="notes-empty">
          <EmptyTitle>No notes yet</EmptyTitle>
          <EmptyDescription>
            Press Left Shift twice in the TypeAside app to write one, or type above.
          </EmptyDescription>
        </Empty>
      )}
      {[...groups].map(([label, items]) => (
        <section key={label} aria-label={label}>
          <h2 className="notes-group">
            {label === "Pinned" && <Pin aria-hidden="true" />}
            {label}
          </h2>
          {items.map((note) => (
            <NoteRow key={note.id} note={note} notify={notify} reportError={reportError} />
          ))}
        </section>
      ))}
      <div className="notes-toast" role="status">
        {message}
      </div>
    </main>
  );
}

function NoteRow({
  note,
  notify,
  reportError,
}: {
  note: Note;
  notify: (message: string) => void;
  reportError: (cause: unknown) => void;
}) {
  const save = useMutation(api.notes.save);
  const remove = useMutation(api.notes.remove);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(note.text);
  const [busy, setBusy] = useState(false);
  const submitting = useRef(false);
  const [title, ...body] = note.text.split("\n");
  async function commit(pinned = note.pinned) {
    if (submitting.current) return;
    submitting.current = true;
    setBusy(true);
    try {
      const text = editing ? draft : note.text;
      if (!text.trim()) {
        await remove({ id: note.id });
        notify("Deleted");
      } else if (text.trim() !== note.text || pinned !== note.pinned) {
        await save({ id: note.id, text, pinned });
      }
      setEditing(false);
    } catch (cause) {
      reportError(cause);
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  }
  async function deleteNote() {
    setBusy(true);
    try {
      await remove({ id: note.id });
      notify("Deleted");
    } catch (cause) {
      reportError(cause);
      setBusy(false);
    }
  }
  const actions = (
    <div className="note-actions">
      <Button
        variant="ghost"
        size="icon"
        className="note-action"
        aria-label={note.pinned ? "Unpin" : "Pin"}
        title={note.pinned ? "Unpin" : "Pin"}
        aria-pressed={note.pinned}
        disabled={busy}
        onClick={() => void commit(!note.pinned)}
      >
        <Pin fill={note.pinned ? "currentColor" : "none"} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="note-action"
        aria-label="Copy"
        title="Copy"
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(editing ? draft : note.text);
            notify("Copied");
          } catch {
            reportError(new ConvexError("Your note could not be copied. Please try again."));
          }
        }}
      >
        <Copy />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="note-action"
        aria-label="Delete"
        title="Delete"
        disabled={busy}
        onClick={() => void deleteNote()}
      >
        <Trash2 />
      </Button>
    </div>
  );
  return (
    <div
      className="note-row"
      data-editing={editing}
      onMouseDown={(event) => {
        // Keep the empty composer's hints from collapsing before the row receives its click.
        if (!editing) event.preventDefault();
      }}
      onBlur={(event) => {
        if (editing && !event.currentTarget.contains(event.relatedTarget)) void commit();
      }}
    >
      {editing ? (
        <>
          <Textarea
            className="note-editor"
            aria-label="Edit note"
            autoFocus
            rows={1}
            maxLength={20_000}
            value={draft}
            disabled={busy}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                setEditing(false);
                setDraft(note.text);
              } else submitOnEnter(event, () => void commit());
            }}
          />
          <div className="note-edit-bar">
            {actions}
            <span className="note-edit-hint">Enter to save</span>
            <Button className="notes-save" disabled={busy} onClick={() => void commit()}>
              Save
            </Button>
          </div>
        </>
      ) : (
        <>
          <button
            className="note-text"
            onClick={() => {
              setDraft(note.text);
              setEditing(true);
            }}
          >
            {body.length ? (
              <>
                <span className="note-title">{title}</span>
                <span className="note-body">{body.join("\n")}</span>
              </>
            ) : (
              note.text
            )}
          </button>
          <time className="note-time" dateTime={new Date(note.createdAt).toISOString()}>
            {new Date(note.createdAt).toLocaleString(
              undefined,
              note.pinned
                ? { month: "short", day: "numeric" }
                : { hour: "numeric", minute: "2-digit" },
            )}
          </time>
          {actions}
        </>
      )}
    </div>
  );
}
