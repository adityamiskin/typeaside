import { api } from "@typeaside/backend/convex/_generated/api";
import { useQuery } from "convex/react";
import { useRouter } from "expo-router";
import { useState } from "react";
import { useNoteActions } from "@/notes/actions";
import { arrange } from "@/notes/model";
import { NotesView } from "@/notes/notes-view";

export default function NotesScreen() {
  const router = useRouter();
  const notes = useQuery(api.notes.list);
  const actions = useNoteActions();
  const [archive, setArchive] = useState(false);
  const [search, setSearch] = useState("");
  const visible = notes ? arrange(notes, archive, search) : [];
  const empty =
    notes && visible.length === 0
      ? search.trim()
        ? "search"
        : archive
          ? "archive"
          : "notes"
      : undefined;

  return (
    <NotesView
      notes={visible}
      empty={empty}
      archive={archive}
      onArchive={setArchive}
      onSearch={setSearch}
      onOpen={(note) => router.push({ pathname: "/notes/[id]", params: { id: note.id } })}
      onCreate={() => router.push({ pathname: "/notes/[id]", params: { id: "new" } })}
      onDictate={() =>
        router.push({ pathname: "/notes/[id]", params: { id: "new", dictate: "1" } })
      }
      actions={actions}
    />
  );
}
