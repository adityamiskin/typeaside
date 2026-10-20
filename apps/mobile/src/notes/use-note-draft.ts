import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation } from "convex/react";
import { useEffect, useEffectEvent, useRef, useState } from "react";
import { Alert, AppState } from "react-native";
import { failureMessage } from "./actions";
import type { Draft } from "./model";

const SAVE_DELAY = 600;

// Local draft is the source of truth while the editor is open; saves are debounced and flushed on close.
export function useNoteDraft(initial: Draft, stored: boolean) {
  const save = useMutation(api.notes.save);
  const [draft, setDraft] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const latest = useRef(initial);
  const saved = useRef(stored ? JSON.stringify(initial) : null);
  const mounted = useRef(true);
  const discarded = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  function flush() {
    clearTimeout(timer.current);
    if (discarded.current) return;
    const note = latest.current;
    const snapshot = JSON.stringify(note);
    // The server rejects blank text; an emptied note keeps its last saved text.
    if (!note.text.trim() || snapshot === saved.current) return;
    saved.current = snapshot;
    save(note).then(
      () => {
        if (mounted.current) setError(null);
      },
      (cause: unknown) => {
        if (saved.current === snapshot) saved.current = null;
        if (mounted.current) setError(failureMessage(cause));
        else Alert.alert("Could not save note", failureMessage(cause));
      },
    );
  }

  // Pass a function when the change depends on the draft at apply time (e.g. after an await).
  function update(
    changes: Partial<Draft> | ((draft: Draft) => Partial<Draft>),
    { now = false } = {},
  ) {
    const next = typeof changes === "function" ? changes(latest.current) : changes;
    latest.current = { ...latest.current, ...next };
    setDraft(latest.current);
    clearTimeout(timer.current);
    if (now) flush();
    else timer.current = setTimeout(flush, SAVE_DELAY);
  }

  const flushOnClose = useEffectEvent(flush);
  useEffect(() => {
    mounted.current = true;
    const subscription = AppState.addEventListener("change", (state) => {
      if (state !== "active") flushOnClose();
    });
    return () => {
      mounted.current = false;
      subscription.remove();
      flushOnClose();
    };
  }, []);

  return {
    draft,
    error: stored && !draft.text.trim() ? "Empty notes are not saved." : error,
    update,
    discard: () => {
      discarded.current = true;
      clearTimeout(timer.current);
    },
  };
}
