import { Box, DropdownMenu, DropdownMenuItem, Host, Text } from "@expo/ui/jetpack-compose";
import { size } from "@expo/ui/jetpack-compose/modifiers";
import * as Haptics from "expo-haptics";
import { useRouter } from "expo-router";
import { type ReactNode, useState } from "react";
import { Pressable, type StyleProp, View, type ViewStyle } from "react-native";
import { colors } from "@/ui/theme";
import type { NoteActions } from "./actions";
import type { Note } from "./model";

// Long press opens a Material dropdown anchored where the finger was.
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
  const router = useRouter();
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);
  const items = [
    !note.archived && {
      label: note.pinned ? "Unpin" : "Pin",
      run: () => actions.togglePin(note),
    },
    { label: note.archived ? "Unarchive" : "Archive", run: () => actions.toggleArchive(note) },
    { label: "Delete", run: () => actions.confirmDelete(note), destructive: true },
  ].filter((item) => item !== false);

  return (
    <View>
      <Pressable
        android_ripple={{ color: "rgba(255,255,255,0.08)" }}
        style={style}
        onPress={() => router.push({ pathname: "/notes/[id]", params: { id: note.id } })}
        onLongPress={(event) => {
          void Haptics.performAndroidHapticsAsync(Haptics.AndroidHaptics.Long_Press);
          setAnchor({ x: event.nativeEvent.locationX, y: event.nativeEvent.locationY });
        }}
      >
        {children}
      </Pressable>
      {anchor && (
        <Host
          matchContents
          colorScheme="dark"
          style={{ position: "absolute", left: anchor.x, top: anchor.y }}
        >
          <DropdownMenu expanded color={colors.fill} onDismissRequest={() => setAnchor(null)}>
            <DropdownMenu.Trigger>
              <Box modifiers={[size(1, 1)]} />
            </DropdownMenu.Trigger>
            <DropdownMenu.Items>
              {items.map((item) => (
                <DropdownMenuItem
                  key={item.label}
                  elementColors={{ textColor: item.destructive ? colors.danger : colors.text }}
                  onClick={() => {
                    setAnchor(null);
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
        </Host>
      )}
    </View>
  );
}
