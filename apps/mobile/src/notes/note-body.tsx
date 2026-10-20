import { SymbolView } from "expo-symbols";
import { type Ref, useEffect, useImperativeHandle, useRef, useState } from "react";
import {
  type ColorValue,
  Keyboard,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  type TextLayoutEvent,
  type TextInputInstance,
  View,
} from "react-native";
import { useColors } from "@/ui/theme";
import { checklistItem } from "./model";

// A plain TextInput cannot host tappable checkboxes, so the note reads as rendered lines and
// switches to the raw text on tap, with the caret at the end of the tapped line.
export function NoteBody({
  text,
  startEditing,
  onChange,
  onToggle,
  ref,
}: {
  text: string;
  startEditing: boolean;
  onChange: (text: string) => void;
  onToggle: (line: number) => void;
  ref?: Ref<{ edit: () => void }>;
}) {
  const colors = useColors();
  const ink = { color: colors.text };
  const faint = { color: colors.tertiary };
  // Keep dims checked items only to the secondary ink so they stay readable on any note color.
  const done = [styles.checked, { color: colors.secondary, textDecorationColor: colors.secondary }];
  const [editAt, setEditAt] = useState<number | null>(startEditing ? text.length : null);
  const input = useRef<TextInputInstance>(null);
  const lines = text.split("\n");
  useImperativeHandle(ref, () => ({ edit: () => setEditAt(0) }));

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
    return checklistItem(lines[index] ?? "")?.checked ? [styles.body, done] : [styles.body, ink];
  }

  function editLine(index: number) {
    setEditAt(lines.slice(0, index + 1).join("\n").length);
  }

  if (editAt !== null)
    return (
      <TextInput
        ref={input}
        multiline
        style={[styles.content, styles.input, ink]}
        placeholder="Note"
        placeholderTextColor={colors.tertiary}
        selectionColor={colors.accent}
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
              <ChecklistText
                text={item.text || " "}
                checked={item.checked}
                color={item.checked ? colors.secondary : colors.text}
                onPress={() => editLine(index)}
              />
            </View>
          );
        })
      ) : (
        <Text style={[styles.body, faint]} onPress={() => setEditAt(0)}>
          Note
        </Text>
      )}
      <Pressable style={styles.grow} onPress={() => setEditAt(text.length)} />
    </ScrollView>
  );
}

function ChecklistText({
  text,
  checked,
  color,
  onPress,
}: {
  text: string;
  checked: boolean;
  color: ColorValue;
  onPress: () => void;
}) {
  const [lines, setLines] = useState<TextLayoutEvent["nativeEvent"]["lines"]>([]);
  return (
    <View style={styles.itemText}>
      <Text
        style={[styles.body, { color }, checked && styles.checked]}
        onPress={onPress}
        onTextLayout={checked ? (event) => setLines(event.nativeEvent.lines) : undefined}
      >
        {text}
      </Text>
      {checked &&
        process.env.EXPO_OS !== "ios" &&
        lines.map((line, index) => (
          // RN 0.88's Android decoration span does not draw reliably on this renderer.
          <View
            key={index}
            pointerEvents="none"
            style={{
              position: "absolute",
              left: line.x,
              top: line.y + line.height * 0.54,
              width: line.width,
              height: 1,
              backgroundColor: color,
            }}
          />
        ))}
    </View>
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  grow: { flexGrow: 1 },
  content: { paddingHorizontal: 16, paddingTop: 4, paddingBottom: 24 },
  input: { flex: 1, fontSize: 16 },
  body: { fontSize: 16, lineHeight: 24 },
  checked: { textDecorationLine: "line-through", textDecorationStyle: "solid" },
  item: { flexDirection: "row", alignItems: "center", gap: 10, minHeight: 32 },
  itemText: { flex: 1 },
});
