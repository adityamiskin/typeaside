import { api } from "@typeaside/backend/convex/_generated/api";
import { useQuery } from "convex/react";
import { Stack, useRouter } from "expo-router";
import { SymbolView } from "expo-symbols";
import { useState } from "react";
import { FlatList, Pressable, StyleSheet, Text } from "react-native";
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { useNoteActions } from "@/notes/actions";
import { arrange } from "@/notes/model";
import { NoteRow } from "@/notes/note-row";
import { Segments } from "@/notes/segments";
import { colors } from "@/ui/theme";

export default function NotesScreen() {
  const router = useRouter();
  const notes = useQuery(api.notes.list);
  const actions = useNoteActions();
  const [archive, setArchive] = useState(false);
  const [search, setSearch] = useState("");
  const visible = notes ? arrange(notes, archive, search) : [];
  const create = () => router.push({ pathname: "/notes/[id]", params: { id: "new" } });

  return (
    <>
      <Stack.SearchBar
        placeholder="Search"
        textColor={colors.text}
        onChangeText={(event) => setSearch(event.nativeEvent.text)}
        onCancelButtonPress={() => setSearch("")}
      />
      <Stack.Toolbar placement="right">
        {process.env.EXPO_OS === "ios" ? (
          <Stack.Toolbar.Button
            icon="square.and.pencil"
            accessibilityLabel="New note"
            onPress={create}
          />
        ) : (
          <Stack.Toolbar.View>
            <Pressable
              accessibilityLabel="New note"
              hitSlop={8}
              android_ripple={{ color: "rgba(255,255,255,0.12)", borderless: true, radius: 20 }}
              onPress={create}
            >
              <SymbolView name={{ android: "edit_square" }} tintColor={colors.text} size={24} />
            </Pressable>
          </Stack.Toolbar.View>
        )}
      </Stack.Toolbar>
      {/* Swipeable rows need a gesture root; collapsable keeps the large title tracking the list. */}
      <GestureHandlerRootView style={styles.screen} collapsable={false}>
        <FlatList
          data={visible}
          keyExtractor={(note) => note.id}
          contentInsetAdjustmentBehavior="automatic"
          keyboardDismissMode="on-drag"
          ListHeaderComponent={<Segments archive={archive} onChange={setArchive} />}
          renderItem={({ item }) => <NoteRow note={item} actions={actions} />}
          ListEmptyComponent={
            notes ? (
              <Text style={styles.empty}>
                {search.trim() ? "No matches" : archive ? "No archived notes" : "No notes yet"}
              </Text>
            ) : undefined
          }
        />
      </GestureHandlerRootView>
    </>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1, backgroundColor: colors.background },
  empty: { color: colors.tertiary, fontSize: 15, paddingHorizontal: 16, paddingVertical: 12 },
});
