import { Host, Picker, Text as SwiftText } from "@expo/ui/swift-ui";
import { pickerStyle, tag } from "@expo/ui/swift-ui/modifiers";
import { Stack } from "expo-router";
import { FlatList, PlatformColor, StyleSheet, Text } from "react-native";
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { emptyMessages } from "./model";
import type { NotesViewProps } from "./notes-view";
import { SwipeRow } from "./swipe-row";

export function NotesView({
  onAccount,
  notes,
  empty,
  archive,
  onArchive,
  onSearch,
  onCreate,
  actions,
}: NotesViewProps) {
  return (
    <>
      <Stack.SearchBar
        placeholder="Search"
        onChangeText={(event) => onSearch(event.nativeEvent.text)}
        onCancelButtonPress={() => onSearch("")}
      />
      <Stack.Toolbar placement="left">
        <Stack.Toolbar.Button
          icon="person.crop.circle"
          accessibilityLabel="Account"
          onPress={onAccount}
        />
      </Stack.Toolbar>
      <Stack.Toolbar placement="right">
        <Stack.Toolbar.Button
          icon="square.and.pencil"
          accessibilityLabel="New note"
          onPress={onCreate}
        />
      </Stack.Toolbar>
      {/* Swipeable rows need a gesture root; collapsable keeps the large title tracking the list. */}
      <GestureHandlerRootView style={styles.screen} collapsable={false}>
        <FlatList
          data={notes}
          keyExtractor={(note) => note.id}
          contentInsetAdjustmentBehavior="automatic"
          keyboardDismissMode="on-drag"
          ListHeaderComponent={
            <Host matchContents={{ vertical: true }} style={styles.segments}>
              <Picker
                modifiers={[pickerStyle("segmented")]}
                selection={archive ? "archive" : "notes"}
                onSelectionChange={(selection) => onArchive(selection === "archive")}
              >
                <SwiftText modifiers={[tag("notes")]}>Notes</SwiftText>
                <SwiftText modifiers={[tag("archive")]}>Archive</SwiftText>
              </Picker>
            </Host>
          }
          renderItem={({ item }) => <SwipeRow note={item} actions={actions} />}
          ListEmptyComponent={
            empty ? <Text style={styles.empty}>{emptyMessages[empty]}</Text> : undefined
          }
        />
      </GestureHandlerRootView>
    </>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1, backgroundColor: PlatformColor("systemBackground") },
  segments: { marginHorizontal: 16, marginBottom: 10 },
  empty: {
    color: PlatformColor("secondaryLabel"),
    fontSize: 15,
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
});
