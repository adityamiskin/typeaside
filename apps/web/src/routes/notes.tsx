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
import {
  Archive,
  ArchiveRestore,
  Check,
  Copy,
  Palette,
  Pin,
  Search,
  Square,
  SquareCheck,
  Trash2,
} from "lucide-react";
import { Popover, PopoverContent, PopoverTrigger } from "@typeaside/ui/components/popover";
import { noteColors, type NoteColor } from "@typeaside/backend/convex/noteColors";
import { labels, hashtagParts, checklistItem, toggleChecklist } from "../lib/notes";
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
  const [archive, setArchive] = useState(false);
  const [label, setLabel] = useState<string | null>(null);
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
      await save({
        id: crypto.randomUUID(),
        text: draft,
        pinned: false,
        archived: false,
        color: "none",
      });
      setDraft("");
    } catch (cause) {
      reportError(cause);
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  }
  const view = notes?.filter((note) => note.archived === archive) ?? [];
  const counts = new Map<string, number>();
  for (const note of view)
    for (const name of labels(note.text)) counts.set(name, (counts.get(name) ?? 0) + 1);
  const sortedLabels = [...counts].sort(([a, ac], [b, bc]) => bc - ac || a.localeCompare(b));
  const groups = new Map<string, Note[]>();
  const matching = view.filter(
    (note) =>
      note.text.toLowerCase().includes(search.toLowerCase()) &&
      (!label || labels(note.text).includes(label)),
  );
  const pinned = matching.filter((note) => !archive && note.pinned);
  if (pinned.length) groups.set("Pinned", pinned);
  for (const note of matching) {
    if (!archive && note.pinned) continue;
    const label = dayLabel(note.createdAt);
    const group = groups.get(label);
    if (group) group.push(note);
    else groups.set(label, [note]);
  }
  return (
    <main className="notes-page">
      <div className="notes-heading">
        <h1>Notes</h1>
        <span className="notes-count">{view.length}</span>
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
      <div className="notes-filters">
        {sortedLabels.length > 0 && (
          <>
            <button data-selected={!label} onClick={() => setLabel(null)}>
              All
            </button>
            {sortedLabels.map(([name]) => (
              <button
                key={name}
                data-selected={label === name}
                onClick={() => setLabel(label === name ? null : name)}
              >
                #{name}
              </button>
            ))}
          </>
        )}
        <button
          className="notes-archive"
          data-selected={archive}
          onClick={() => {
            setArchive(!archive);
            setLabel(null);
          }}
        >
          <Archive />
          Archive
        </button>
      </div>
      {!archive && (
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
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {notes === undefined && <p>Loading…</p>}
      {notes !== undefined && matching.length === 0 && (
        <Empty className="notes-empty" data-archive={archive}>
          <EmptyTitle>{archive ? "No archived notes" : "No notes yet"}</EmptyTitle>
          {!archive && (
            <EmptyDescription>
              Press Left Shift twice in the TypeAside app to write one, or type above.
            </EmptyDescription>
          )}
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
  const [colorOpen, setColorOpen] = useState(false);
  async function commit(
    changes: Partial<Pick<Note, "pinned" | "archived" | "color" | "text">> = {},
  ) {
    if (submitting.current) return;
    submitting.current = true;
    setBusy(true);
    try {
      const text = changes.text ?? (editing ? draft : note.text);
      if (!text.trim()) {
        await remove({ id: note.id });
        notify("Deleted");
      } else if (text.trim() !== note.text || Object.keys(changes).length > 0) {
        await save({
          id: note.id,
          text,
          pinned: note.pinned,
          archived: note.archived,
          color: note.color,
          ...changes,
        });
      }
      setEditing(false);
      return true;
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
    <div className="note-actions" onClick={(event) => event.stopPropagation()}>
      {!note.archived && (
        <Button
          variant="ghost"
          size="icon"
          className="note-action"
          aria-label={note.pinned ? "Unpin" : "Pin"}
          title={note.pinned ? "Unpin" : "Pin"}
          aria-pressed={note.pinned}
          disabled={busy}
          onClick={() => void commit({ pinned: !note.pinned })}
        >
          <Pin fill={note.pinned ? "currentColor" : "none"} />
        </Button>
      )}
      <Popover open={colorOpen} onOpenChange={setColorOpen}>
        <PopoverTrigger
          render={
            <Button
              variant="ghost"
              size="icon"
              className="note-action"
              aria-label="Color"
              title="Color"
              disabled={busy}
            />
          }
        >
          <Palette />
        </PopoverTrigger>
        <PopoverContent className="note-colors" align="start" aria-label="Note color">
          {noteColors.map((color) => (
            <button
              key={color}
              aria-label={color}
              aria-pressed={note.color === color}
              disabled={busy}
              style={{ background: noteTints[color] }}
              onClick={() => {
                setColorOpen(false);
                void commit({ color });
              }}
            >
              {note.color === color && <Check />}
            </button>
          ))}
        </PopoverContent>
      </Popover>
      <Button
        variant="ghost"
        size="icon"
        className="note-action"
        aria-label={note.archived ? "Unarchive" : "Archive"}
        title={note.archived ? "Unarchive" : "Archive"}
        disabled={busy}
        onClick={async () => {
          if (await commit({ archived: !note.archived, pinned: false }))
            notify(note.archived ? "Restored" : "Archived");
        }}
      >
        {note.archived ? <ArchiveRestore /> : <Archive />}
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
      data-color-open={colorOpen}
      style={{ "--note-tint": noteTints[note.color] } as React.CSSProperties}
      onMouseDown={(event) => {
        // Keep the empty composer's hints from collapsing before the row receives its click.
        if (!editing && !(event.target instanceof Element && event.target.closest(".note-actions")))
          event.preventDefault();
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
          <NoteText
            text={note.text}
            busy={busy}
            edit={() => {
              setDraft(note.text);
              setEditing(true);
            }}
            toggle={(index) => void commit({ text: toggleChecklist(note.text, index) })}
          />
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

const noteTints = {
  none: "var(--bg)",
  sky: "#e6effa",
  lilac: "#efe7f5",
  sage: "#e5f1e8",
  sand: "#f6efdf",
  rose: "#f8e6e6",
} satisfies Record<NoteColor, string>;

function Hashtags({ text }: { text: string }) {
  return hashtagParts(text).map((part, index) =>
    index % 2 ? (
      <span className="note-hashtag" key={index}>
        {part}
      </span>
    ) : (
      part
    ),
  );
}

function NoteText({
  text,
  busy,
  edit,
  toggle,
}: {
  text: string;
  busy: boolean;
  edit: () => void;
  toggle: (index: number) => void;
}) {
  const lines = text.split("\n");
  const hasChecklist = lines.some((line) => checklistItem(line));
  if (!hasChecklist)
    return (
      <button className="note-text" onClick={edit}>
        {lines.length > 1 ? (
          <>
            <span className="note-title">
              <Hashtags text={lines[0]!} />
            </span>
            <span className="note-body">
              <Hashtags text={lines.slice(1).join("\n")} />
            </span>
          </>
        ) : (
          <Hashtags text={text} />
        )}
      </button>
    );
  let shown = 0;
  const total = lines.filter((line) => checklistItem(line)).length;
  return (
    <div className="note-text" onClick={edit}>
      {lines.map((line, index) => {
        const item = checklistItem(line);
        if (!item)
          return (
            <div key={index} className={index === 0 ? "note-title" : "note-plain-line"}>
              <Hashtags text={line} />
            </div>
          );
        if (++shown > 8) return null;
        return (
          <div className="note-checklist" key={index} data-checked={item.checked}>
            <button
              aria-label={item.checked ? "Uncheck item" : "Check item"}
              disabled={busy}
              onClick={(event) => {
                event.stopPropagation();
                toggle(index);
              }}
            >
              {item.checked ? <SquareCheck /> : <Square />}
            </button>
            <span>
              <Hashtags text={item.text} />
            </span>
          </div>
        );
      })}
      {total > 8 && <div className="note-more">+{total - 8} more</div>}
    </div>
  );
}
