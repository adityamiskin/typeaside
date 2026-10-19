import { Host, Picker, Text } from "@expo/ui/swift-ui";
import { pickerStyle, tag } from "@expo/ui/swift-ui/modifiers";

export function Segments({
  archive,
  onChange,
}: {
  archive: boolean;
  onChange: (archive: boolean) => void;
}) {
  return (
    <Host
      matchContents={{ vertical: true }}
      colorScheme="dark"
      style={{ marginHorizontal: 16, marginBottom: 10 }}
    >
      <Picker
        modifiers={[pickerStyle("segmented")]}
        selection={archive ? "archive" : "notes"}
        onSelectionChange={(selection) => onChange(selection === "archive")}
      >
        <Text modifiers={[tag("notes")]}>Notes</Text>
        <Text modifiers={[tag("archive")]}>Archive</Text>
      </Picker>
    </Host>
  );
}
