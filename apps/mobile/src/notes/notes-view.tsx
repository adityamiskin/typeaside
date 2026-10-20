import {
  Box,
  Card,
  Column,
  DockedSearchBar,
  DropdownMenu,
  DropdownMenuItem,
  FloatingActionButton,
  Host,
  Icon,
  LazyColumn,
  Row,
  SegmentedButton,
  SingleChoiceSegmentedButtonRow,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  align,
  combinedClickable,
  fillMaxSize,
  fillMaxWidth,
  padding,
  weight,
} from "@expo/ui/jetpack-compose/modifiers";
import * as Haptics from "expo-haptics";
import { useState } from "react";
import { View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { icons } from "@/ui/icons";
import { LargeTitle } from "@/ui/large-title";
import { seedColor, useColors, useNoteColors } from "@/ui/theme";
import type { NoteActions } from "./actions";
import { type Note, relativeTime, summarize } from "./model";

const PREVIEW_ITEMS = 3;

export type NotesViewProps = {
  notes: Note[];
  empty: string | undefined;
  archive: boolean;
  onArchive: (archive: boolean) => void;
  onSearch: (search: string) => void;
  onOpen: (note: Note) => void;
  onCreate: () => void;
  actions: NoteActions;
};

export function NotesView(props: NotesViewProps) {
  const insets = useSafeAreaInsets();
  const colors = useColors();
  return (
    <View style={{ flex: 1, paddingTop: insets.top, backgroundColor: colors.background }}>
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
  actions,
}: NotesViewProps) {
  const material = useMaterialColors();
  const tints = useNoteColors();
  const [menuFor, setMenuFor] = useState<string | null>(null);

  return (
    <Box modifiers={[fillMaxSize()]}>
      <LazyColumn
        modifiers={[fillMaxSize()]}
        verticalArrangement={{ spacedBy: 8 }}
        contentPadding={{ bottom: 104 }}
      >
        <LargeTitle title="Notes" />
        <DockedSearchBar
          onQueryChange={onSearch}
          modifiers={[padding(16, 0, 16, 0), fillMaxWidth()]}
        >
          <DockedSearchBar.Placeholder>
            <Text>Search notes</Text>
          </DockedSearchBar.Placeholder>
        </DockedSearchBar>
        <SingleChoiceSegmentedButtonRow modifiers={[padding(16, 8, 16, 8), fillMaxWidth()]}>
          {[false, true].map((value) => (
            <SegmentedButton
              key={String(value)}
              selected={archive === value}
              onClick={() => onArchive(value)}
            >
              <SegmentedButton.Label>
                <Text>{value ? "Archive" : "Notes"}</Text>
              </SegmentedButton.Label>
            </SegmentedButton>
          ))}
        </SingleChoiceSegmentedButtonRow>
        {empty && (
          <Text color={material.onSurfaceVariant} modifiers={[padding(16, 8, 16, 8)]}>
            {empty}
          </Text>
        )}
        {notes.map((note) => {
          const { title, items, preview } = summarize(note.text);
          const done = items.filter((item) => item.checked).length;
          const meta = [
            items.length > 0 && `${done} of ${items.length} done`,
            relativeTime(note.updatedAt),
          ]
            .filter(Boolean)
            .join(" · ");
          const tint = tints[note.color];
          const menu = [
            !note.archived && {
              label: note.pinned ? "Unpin" : "Pin",
              run: () => actions.togglePin(note),
            },
            {
              label: note.archived ? "Unarchive" : "Archive",
              run: () => actions.toggleArchive(note),
            },
            { label: "Delete", run: () => actions.confirmDelete(note), destructive: true },
          ].filter((item) => item !== false);

          return (
            <DropdownMenu
              key={note.id}
              expanded={menuFor === note.id}
              onDismissRequest={() => setMenuFor(null)}
            >
              <DropdownMenu.Trigger>
                <Card
                  colors={{ containerColor: tint.background, contentColor: tint.text }}
                  modifiers={[padding(16, 0, 16, 0), fillMaxWidth()]}
                >
                  <Column
                    modifiers={[
                      fillMaxWidth(),
                      combinedClickable({
                        onClick: () => onOpen(note),
                        onLongClick: () => {
                          void Haptics.performAndroidHapticsAsync(
                            Haptics.AndroidHaptics.Long_Press,
                          );
                          setMenuFor(note.id);
                        },
                      }),
                      padding(16, 12, 16, 14),
                    ]}
                  >
                    {(title || note.pinned) && (
                      <Row verticalAlignment="center" horizontalArrangement={{ spacedBy: 6 }}>
                        {note.pinned && <Icon source={icons.pinned} size={16} tint={tint.accent} />}
                        <Text
                          style={{ typography: "titleMedium" }}
                          maxLines={1}
                          overflow="ellipsis"
                          modifiers={[weight(1)]}
                        >
                          {title}
                        </Text>
                      </Row>
                    )}
                    {items.slice(0, PREVIEW_ITEMS).map((item, index) => (
                      <Row
                        key={index}
                        verticalAlignment="center"
                        horizontalArrangement={{ spacedBy: 8 }}
                      >
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
                    {items.length === 0 && preview ? (
                      <Text
                        style={{ typography: "bodyMedium" }}
                        maxLines={title ? 2 : 3}
                        overflow="ellipsis"
                      >
                        {preview}
                      </Text>
                    ) : null}
                    <Text style={{ typography: "labelSmall" }} modifiers={[padding(0, 6, 0, 0)]}>
                      {meta}
                    </Text>
                  </Column>
                </Card>
              </DropdownMenu.Trigger>
              <DropdownMenu.Items>
                {menu.map((item) => (
                  <DropdownMenuItem
                    key={item.label}
                    elementColors={item.destructive ? { textColor: material.error } : undefined}
                    onClick={() => {
                      setMenuFor(null);
                      item.run();
                    }}
                  >
                    <DropdownMenuItem.Text>
                      <Text>{item.label}</Text>
                    </DropdownMenuItem.Text>
                  </DropdownMenuItem>
                ))}
              </DropdownMenu.Items>
            </DropdownMenu>
          );
        })}
      </LazyColumn>
      <FloatingActionButton
        onClick={onCreate}
        modifiers={[align("bottomEnd"), padding(16, 16, 16, 16)]}
      >
        <FloatingActionButton.Icon>
          <Icon source={icons.add} size={24} contentDescription="New note" />
        </FloatingActionButton.Icon>
      </FloatingActionButton>
    </Box>
  );
}
