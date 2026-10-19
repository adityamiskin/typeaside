import { Link } from "expo-router";
import type { ReactNode } from "react";
import { Pressable, type StyleProp, type ViewStyle } from "react-native";
import type { NoteActions } from "./actions";
import type { Note } from "./model";

// Long press lifts the row into a native UIContextMenu.
export function NoteLink({
  note,
  actions,
  style,
  children,
}: {
  note: Note;
  actions: NoteActions;
  style: StyleProp<ViewStyle>;
  children: ReactNode;
}) {
  return (
    <Link href={{ pathname: "/notes/[id]", params: { id: note.id } }} asChild>
      <Link.Trigger>
        <Pressable style={({ pressed }) => [style, pressed && { opacity: 0.7 }]}>
          {children}
        </Pressable>
      </Link.Trigger>
      <Link.Menu>
        <Link.MenuAction
          hidden={note.archived}
          title={note.pinned ? "Unpin" : "Pin"}
          icon={note.pinned ? "pin.slash" : "pin"}
          onPress={() => actions.togglePin(note)}
        />
        <Link.MenuAction
          title={note.archived ? "Unarchive" : "Archive"}
          icon={note.archived ? "tray.and.arrow.up" : "archivebox"}
          onPress={() => actions.toggleArchive(note)}
        />
        <Link.MenuAction
          title="Delete"
          icon="trash"
          destructive
          onPress={() => actions.confirmDelete(note)}
        />
      </Link.Menu>
    </Link>
  );
}
