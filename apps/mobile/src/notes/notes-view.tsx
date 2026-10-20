import {
  Box,
  Card,
  Column,
  DockedSearchBar,
  DropdownMenu,
  DropdownMenuItem,
  ExtendedFloatingActionButton,
  FilledTonalIconButton,
  Host,
  Icon,
  IconButton,
  LazyColumn,
  Row,
  SmallFloatingActionButton,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  align,
  background,
  border,
  clip,
  combinedClickable,
  fillMaxSize,
  fillMaxWidth,
  height,
  padding,
  Shapes,
  size,
  weight,
} from "@expo/ui/jetpack-compose/modifiers";
import { noteColors as noteColorNames } from "@typeaside/backend/convex/noteColors";
import * as Haptics from "expo-haptics";
import { useEffect, useState } from "react";
import { BackHandler, useColorScheme, View } from "react-native";
import { icons } from "@/ui/icons";
import { seedColor, useColors, useNoteColors } from "@/ui/theme";
import type { NoteActions } from "./actions";
import { type EmptyState, emptyMessages, type Note, summarize } from "./model";

const PREVIEW_LINES = 8;
const PREVIEW_ITEMS = 6;

export type NotesViewProps = {
  notes: Note[];
  empty: EmptyState | undefined;
  archive: boolean;
  onArchive: (archive: boolean) => void;
  onSearch: (search: string) => void;
  onOpen: (note: Note) => void;
  onCreate: () => void;
  onDictate: () => void;
  actions: NoteActions;
};

export function NotesView(props: NotesViewProps) {
  const colors = useColors();
  const dark = useColorScheme() === "dark";
  return (
    <View
      style={{
        flex: 1,
        backgroundColor: dark ? "#000" : colors.background,
      }}
    >
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <NotesList {...props} />
      </Host>
    </View>
  );
}

function NotesList({
  notes,
  empty,
  archive,
  onArchive,
  onSearch,
  onOpen,
  onCreate,
  onDictate,
  actions,
}: NotesViewProps) {
  const material = useMaterialColors();
  const [grid, setGrid] = useState(true);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  // Notes that leave the view (archived, deleted) drop out of the selection on their own.
  const selection = notes.filter((note) => selectedIds.includes(note.id));
  const selecting = selection.length > 0;
  const clear = () => setSelectedIds([]);
  const toggle = (note: Note) =>
    setSelectedIds((ids) =>
      ids.includes(note.id) ? ids.filter((id) => id !== note.id) : [...ids, note.id],
    );

  useEffect(() => {
    if (!selecting) return;
    const subscription = BackHandler.addEventListener("hardwareBackPress", () => {
      setSelectedIds([]);
      return true;
    });
    return () => subscription.remove();
  }, [selecting]);

  const columns = distribute(notes, grid ? 2 : 1);

  return (
    <Box modifiers={[fillMaxSize()]}>
      <Column modifiers={[fillMaxSize()]}>
        {selecting ? (
          <SelectionBar selection={selection} archive={archive} actions={actions} onClear={clear} />
        ) : (
          <Row
            verticalAlignment="center"
            horizontalArrangement={{ spacedBy: 4 }}
            modifiers={[fillMaxWidth(), padding(16, 8, 8, 8)]}
          >
            <DockedSearchBar onQueryChange={onSearch} modifiers={[weight(1)]}>
              <DockedSearchBar.LeadingIcon>
                <Icon source={icons.search} size={24} />
              </DockedSearchBar.LeadingIcon>
              <DockedSearchBar.Placeholder>
                <Text>{archive ? "Search archive" : "Search notes"}</Text>
              </DockedSearchBar.Placeholder>
            </DockedSearchBar>
            <IconButton onClick={() => setGrid(!grid)}>
              <Icon
                source={grid ? icons.viewAgenda : icons.gridView}
                size={24}
                contentDescription={grid ? "List view" : "Grid view"}
              />
            </IconButton>
            {archive ? (
              <FilledTonalIconButton onClick={() => onArchive(false)}>
                <Icon source={icons.archive} size={24} contentDescription="Close archive" />
              </FilledTonalIconButton>
            ) : (
              <IconButton onClick={() => onArchive(true)}>
                <Icon source={icons.archive} size={24} contentDescription="Archive" />
              </IconButton>
            )}
          </Row>
        )}
        <LazyColumn
          modifiers={[weight(1), fillMaxWidth()]}
          contentPadding={{ top: 8, bottom: 160 }}
        >
          {empty ? (
            <Column
              horizontalAlignment="center"
              verticalArrangement={{ spacedBy: 16 }}
              modifiers={[fillMaxWidth(), padding(32, 160, 32, 0)]}
            >
              <Icon
                source={
                  { notes: icons.noteAdd, archive: icons.archive, search: icons.search }[empty]
                }
                size={96}
                tint={material.outline}
              />
              <Text color={material.onSurfaceVariant} style={{ typography: "bodyLarge" }}>
                {emptyMessages[empty]}
              </Text>
            </Column>
          ) : (
            <Row
              horizontalArrangement={{ spacedBy: 8 }}
              modifiers={[fillMaxWidth(), padding(12, 0, 12, 0)]}
            >
              {columns.map((column, index) => (
                <Column key={index} verticalArrangement={{ spacedBy: 8 }} modifiers={[weight(1)]}>
                  {column.map((note) => (
                    <NoteCard
                      key={note.id}
                      note={note}
                      selected={selectedIds.includes(note.id)}
                      onPress={() => (selecting ? toggle(note) : onOpen(note))}
                      onLongPress={() => {
                        void Haptics.performAndroidHapticsAsync(Haptics.AndroidHaptics.Long_Press);
                        toggle(note);
                      }}
                    />
                  ))}
                </Column>
              ))}
            </Row>
          )}
        </LazyColumn>
      </Column>
      {!selecting && !archive && (
        <Column
          horizontalAlignment="end"
          verticalArrangement={{ spacedBy: 16 }}
          modifiers={[align("bottomEnd"), padding(16, 16, 16, 16)]}
        >
          <SmallFloatingActionButton
            containerColor={material.surfaceContainerHighest}
            onClick={onDictate}
          >
            <SmallFloatingActionButton.Icon>
              <Icon source={icons.mic} size={24} contentDescription="Voice note" />
            </SmallFloatingActionButton.Icon>
          </SmallFloatingActionButton>
          <ExtendedFloatingActionButton onClick={onCreate}>
            <ExtendedFloatingActionButton.Icon>
              <Icon source={icons.add} size={24} contentDescription="New note" />
            </ExtendedFloatingActionButton.Icon>
            <ExtendedFloatingActionButton.Text>
              <Text>New note</Text>
            </ExtendedFloatingActionButton.Text>
          </ExtendedFloatingActionButton>
        </Column>
      )}
    </Box>
  );
}

function SelectionBar({
  selection,
  archive,
  actions,
  onClear,
}: {
  selection: Note[];
  archive: boolean;
  actions: NoteActions;
  onClear: () => void;
}) {
  const material = useMaterialColors();
  const tints = useNoteColors();
  const [palette, setPalette] = useState(false);
  const pinned = selection.every((note) => note.pinned);
  const run = (action: () => void) => {
    action();
    onClear();
  };

  return (
    <Row
      verticalAlignment="center"
      modifiers={[
        fillMaxWidth(),
        padding(16, 8, 16, 8),
        height(56),
        clip(Shapes.RoundedCorner(28)),
        background(material.secondaryContainer),
        padding(4, 0, 4, 0),
      ]}
    >
      <IconButton onClick={onClear}>
        <Icon source={icons.close} size={24} contentDescription="Clear selection" />
      </IconButton>
      <Text
        color={material.onSecondaryContainer}
        style={{ typography: "titleMedium" }}
        modifiers={[weight(1), padding(8, 0, 0, 0)]}
      >
        {String(selection.length)}
      </Text>
      {!archive && (
        <IconButton onClick={() => run(() => actions.pin(selection, !pinned))}>
          <Icon
            source={pinned ? icons.pinned : icons.pin}
            size={24}
            contentDescription={pinned ? "Unpin" : "Pin"}
          />
        </IconButton>
      )}
      <DropdownMenu expanded={palette} onDismissRequest={() => setPalette(false)}>
        <DropdownMenu.Trigger>
          <IconButton onClick={() => setPalette(true)}>
            <Icon source={icons.palette} size={24} contentDescription="Color" />
          </IconButton>
        </DropdownMenu.Trigger>
        <DropdownMenu.Items>
          {noteColorNames.map((name) => (
            <DropdownMenuItem
              key={name}
              onClick={() => {
                setPalette(false);
                run(() => actions.color(selection, name));
              }}
            >
              <DropdownMenuItem.LeadingIcon>
                <Box
                  modifiers={[
                    size(24, 24),
                    clip(Shapes.Circle),
                    background(name === "none" ? material.surface : tints[name].background),
                    border(1, material.outline),
                  ]}
                />
              </DropdownMenuItem.LeadingIcon>
              <DropdownMenuItem.Text>
                <Text>{name === "none" ? "Default" : name[0]?.toUpperCase() + name.slice(1)}</Text>
              </DropdownMenuItem.Text>
            </DropdownMenuItem>
          ))}
        </DropdownMenu.Items>
      </DropdownMenu>
      <IconButton onClick={() => run(() => actions.archive(selection, !archive))}>
        <Icon
          source={archive ? icons.unarchive : icons.archive}
          size={24}
          contentDescription={archive ? "Unarchive" : "Archive"}
        />
      </IconButton>
      <IconButton
        onClick={() =>
          actions.confirmDelete(
            selection.map((note) => note.id),
            onClear,
          )
        }
      >
        <Icon source={icons.delete} size={24} contentDescription="Delete" />
      </IconButton>
    </Row>
  );
}

function NoteCard({
  note,
  selected,
  onPress,
  onLongPress,
}: {
  note: Note;
  selected: boolean;
  onPress: () => void;
  onLongPress: () => void;
}) {
  const material = useMaterialColors();
  const dark = useColorScheme() === "dark";
  const tint = useNoteColors()[note.color];
  const { title, items, preview } = summarize(note.text);
  const plain = note.color === "none";
  const outline = selected
    ? { width: 2, color: material.primary }
    : plain
      ? { width: 1, color: material.outlineVariant }
      : undefined;

  return (
    <Card
      colors={{
        containerColor: plain ? (dark ? "#000" : material.surface) : tint.background,
        contentColor: tint.text,
      }}
      border={outline}
      modifiers={[fillMaxWidth()]}
    >
      <Column
        verticalArrangement={{ spacedBy: 6 }}
        modifiers={[
          fillMaxWidth(),
          combinedClickable({ onClick: onPress, onLongClick: onLongPress }),
          padding(16, 12, 16, 16),
        ]}
      >
        {(title || note.pinned) && (
          <Row verticalAlignment="top" horizontalArrangement={{ spacedBy: 8 }}>
            <Text
              style={{ typography: "titleMedium" }}
              maxLines={3}
              overflow="ellipsis"
              modifiers={[weight(1)]}
            >
              {title}
            </Text>
            {note.pinned && <Icon source={icons.pinned} size={18} tint={tint.accent} />}
          </Row>
        )}
        {preview ? (
          <Text style={{ typography: "bodyMedium" }} maxLines={PREVIEW_LINES} overflow="ellipsis">
            {preview}
          </Text>
        ) : null}
        {items.slice(0, PREVIEW_ITEMS).map((item, index) => (
          <Row key={index} verticalAlignment="center" horizontalArrangement={{ spacedBy: 8 }}>
            <Icon
              source={item.checked ? icons.checkBox : icons.checkBoxBlank}
              size={18}
              tint={tint.accent}
            />
            <Text
              style={{
                typography: "bodyMedium",
                textDecoration: item.checked ? "lineThrough" : "none",
              }}
              maxLines={1}
              overflow="ellipsis"
            >
              {item.text}
            </Text>
          </Row>
        ))}
        {items.length > PREVIEW_ITEMS && (
          <Text style={{ typography: "labelMedium" }}>
            {`+${items.length - PREVIEW_ITEMS} more`}
          </Text>
        )}
      </Column>
    </Card>
  );
}

// Staggered grid: each note goes to the column that currently ends highest.
function distribute(notes: Note[], count: number) {
  const columns = Array.from({ length: count }, (): Note[] => []);
  const heights = columns.map(() => 0);
  for (const note of notes) {
    const shortest = heights.indexOf(Math.min(...heights));
    columns[shortest]?.push(note);
    heights[shortest] = (heights[shortest] ?? 0) + estimateLines(note, count);
  }
  return columns;
}

function estimateLines(note: Note, columns: number) {
  const { title, items, preview } = summarize(note.text);
  // Rough characters per line of body text in a grid or full-width card.
  const width = columns === 1 ? 48 : 24;
  const previewLines = preview
    .split("\n")
    .reduce((sum, line) => sum + Math.max(1, Math.ceil(line.length / width)), 0);
  return (
    2 +
    (title ? 1.3 * Math.min(3, Math.ceil(title.length / (width * 0.8))) : 0) +
    (preview ? Math.min(PREVIEW_LINES, previewLines) : 0) +
    Math.min(PREVIEW_ITEMS, items.length)
  );
}
