import { Button, Host, Text } from "@expo/ui/swift-ui";
import { buttonStyle, controlSize, disabled, frame } from "@expo/ui/swift-ui/modifiers";

export function PrimaryButton({
  label,
  disabled: isDisabled,
  onPress,
}: {
  label: string;
  disabled: boolean;
  onPress: () => void;
}) {
  return (
    <Host matchContents={{ vertical: true }}>
      <Button
        onPress={onPress}
        modifiers={[buttonStyle("borderedProminent"), controlSize("large"), disabled(isDisabled)]}
      >
        <Text modifiers={[frame({ maxWidth: Number.POSITIVE_INFINITY })]}>{label}</Text>
      </Button>
    </Host>
  );
}
