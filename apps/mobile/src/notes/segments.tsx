import {
  Host,
  SegmentedButton,
  SingleChoiceSegmentedButtonRow,
  Text,
} from "@expo/ui/jetpack-compose";
import { fillMaxWidth } from "@expo/ui/jetpack-compose/modifiers";
import { colors } from "@/ui/theme";

const segmentColors = {
  activeContainerColor: colors.fill,
  activeContentColor: colors.text,
  activeBorderColor: colors.separator,
  inactiveContainerColor: colors.background,
  inactiveContentColor: colors.secondary,
  inactiveBorderColor: colors.separator,
};

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
      <SingleChoiceSegmentedButtonRow modifiers={[fillMaxWidth()]}>
        {[false, true].map((value) => (
          <SegmentedButton
            key={String(value)}
            selected={archive === value}
            onClick={() => onChange(value)}
            colors={segmentColors}
          >
            <SegmentedButton.Label>
              <Text>{value ? "Archive" : "Notes"}</Text>
            </SegmentedButton.Label>
          </SegmentedButton>
        ))}
      </SingleChoiceSegmentedButtonRow>
    </Host>
  );
}
