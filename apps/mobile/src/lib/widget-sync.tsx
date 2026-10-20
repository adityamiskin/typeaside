import { api } from "@typeaside/backend/convex/_generated/api";
import { useConvexAuth, useQuery } from "convex/react";
import { useEffect } from "react";
import { widgets, type WidgetSnapshot } from "../../modules/widgets";
import { checklistItem, splitTitle } from "@/notes/model";
import { useSession } from "@/lib/session";

let pendingWrite = Promise.resolve();

export function WidgetSync() {
  const status = useSession((session) => session.status);
  const { isAuthenticated } = useConvexAuth();
  const ready = status === "signedIn" && isAuthenticated;
  const notes = useQuery(api.notes.list, ready ? {} : "skip");
  const account = useQuery(api.viewer.current, ready ? {} : "skip");

  useEffect(() => {
    if (status === "signedOut") {
      pendingWrite = pendingWrite.catch(() => undefined).then(() => widgets.clear());
      void pendingWrite.catch((error: unknown) => console.warn("Widget snapshot:", error));
      return;
    }
    if (!ready || notes === undefined || account === undefined) return;
    const snapshot: WidgetSnapshot = {
      wordsLeft:
        account.usage.cap === null ? null : Math.max(0, account.usage.cap - account.usage.words),
      notes: notes
        .filter((note) => !note.archived)
        .sort((a, b) => b.updatedAt - a.updatedAt)
        .slice(0, 6)
        .map((note) => {
          const { title, body } = splitTitle(note.text);
          return {
            id: note.id,
            title: title.trim(),
            preview: body
              .split("\n")
              .map((line) => checklistItem(line)?.text ?? line)
              .join("\n")
              .trim()
              .slice(0, 500),
            color: note.color,
            updatedAt: note.updatedAt,
          };
        }),
    };
    pendingWrite = pendingWrite
      .catch(() => undefined)
      .then(() =>
        useSession.getState().status === "signedIn"
          ? widgets.setSnapshot(JSON.stringify(snapshot))
          : widgets.clear(),
      );
    void pendingWrite.catch((error: unknown) => console.warn("Widget snapshot:", error));
  }, [status, ready, notes, account]);
  return null;
}
