import { SymbolView } from "expo-symbols";
import { useEffect, useRef, useState } from "react";
import {
  Keyboard,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  type TextInputInstance,
  View,
} from "react-native";
import { colors } from "@/ui/theme";
import { checklistItem, titleLine } from "./model";

// A plain TextInput cannot host tappable checkboxes, so the note reads as rendered lines and
// switches to the raw text on tap, with the caret at the end of the tapped line.
export function NoteBody({
  text,
  startEditing,
  onChange,
  onToggle,
}: {
  text: string;
  startEditing: boolean;
  onChange: (text: string) => void;
  onToggle: (line: number) => void;
}) {
  const [editAt, setEditAt] = useState<number | null>(startEditing ? text.length : null);
  const input = useRef<TextInputInstance>(null);
  const lines = text.split("\n");
  const title = titleLine(lines);

  useEffect(() => {
    if (editAt === null) return;
    input.current?.focus();
    input.current?.setSelection(editAt, editAt);
  }, [editAt]);

  // Android keeps focus after the keyboard is dismissed with the back gesture.
  useEffect(() => {
    const subscription = Keyboard.addListener("keyboardDidHide", () => input.current?.blur());
    return () => subscription.remove();
  }, []);

  function lineStyle(index: number) {
    if (index === title) return styles.title;
    return checklistItem(lines[index] ?? "")?.checked ? [styles.body, styles.checked] : styles.body;
  }

  function editLine(index: number) {
    setEditAt(lines.slice(0, index + 1).join("\n").length);
  }

  if (editAt !== null)
    return (
      <TextInput
        ref={input}
        multiline
        style={[styles.content, styles.input]}
        placeholder="Note"
        placeholderTextColor={colors.tertiary}
        selectionColor={colors.accent}
        keyboardAppearance="dark"
        textAlignVertical="top"
        maxLength={20_000}
        onChangeText={onChange}
        onBlur={() => setEditAt(null)}
      >
        {lines.map((line, index) => (
          <Text key={index} style={lineStyle(index)}>
            {index > 0 ? `\n${line}` : line}
          </Text>
        ))}
      </TextInput>
    );

  return (
    <ScrollView style={styles.fill} contentContainerStyle={[styles.content, styles.grow]}>
      {text ? (
        lines.map((line, index) => {
          const item = checklistItem(line);
          if (!item)
            return (
              <Text key={index} style={lineStyle(index)} onPress={() => editLine(index)}>
                {line || " "}
              </Text>
            );
          return (
            <View key={index} style={styles.item}>
              <Pressable
                hitSlop={10}
                accessibilityRole="checkbox"
                accessibilityState={{ checked: item.checked }}
                onPress={() => onToggle(index)}
              >
                <SymbolView
                  name={{
                    ios: item.checked ? "checkmark.circle.fill" : "circle",
                    android: item.checked ? "check_box" : "check_box_outline_blank",
                  }}
                  tintColor={item.checked ? colors.accent : colors.tertiary}
                  size={20}
                />
              </Pressable>
              <Text
                style={[styles.body, styles.itemText, item.checked && styles.checked]}
                onPress={() => editLine(index)}
              >
                {item.text || " "}
              </Text>
            </View>
          );
        })
      ) : (
        <Text style={[styles.body, styles.placeholder]} onPress={() => setEditAt(0)}>
          Note
        </Text>
      )}
      <Pressable style={styles.grow} onPress={() => setEditAt(text.length)} />
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  grow: { flexGrow: 1 },
  content: { paddingHorizontal: 20, paddingTop: 8, paddingBottom: 24 },
  input: { flex: 1, color: colors.text, fontSize: 17 },
  title: { color: colors.text, fontSize: 24, fontWeight: "700", lineHeight: 30 },
  body: { color: colors.text, fontSize: 17, lineHeight: 24 },
  checked: { color: colors.tertiary, textDecorationLine: "line-through" },
  placeholder: { color: colors.tertiary },
  item: { flexDirection: "row", alignItems: "center", gap: 10, minHeight: 32 },
  itemText: { flex: 1 },
});
