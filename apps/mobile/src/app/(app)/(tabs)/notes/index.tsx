import { api } from "@typeaside/backend/convex/_generated/api";
import { useQuery } from "convex/react";
import { DrawerActions } from "@react-navigation/native";
import { useNavigation, useRouter } from "expo-router";
import { useState } from "react";
import { useNoteActions } from "@/notes/actions";
import { arrange } from "@/notes/model";
import { NotesView } from "@/notes/notes-view";

export default function NotesScreen() {
  const router = useRouter();
  const navigation = useNavigation();
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
      onMenu={() => navigation.dispatch(DrawerActions.openDrawer())}
      onAccount={() => router.push("/account")}
      notes={visible}
      empty={empty}
      archive={archive}
      onArchive={setArchive}
      onSearch={setSearch}
      onOpen={(note) => router.push({ pathname: "/note/[id]", params: { id: note.id } })}
      onCreate={() => router.push({ pathname: "/note/[id]", params: { id: "new" } })}
      onDictate={() => router.push({ pathname: "/note/[id]", params: { id: "new", dictate: "1" } })}
      actions={actions}
    />
  );
}
