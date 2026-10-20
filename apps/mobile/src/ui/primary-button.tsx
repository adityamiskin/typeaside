import { Button, Host, Text } from "@expo/ui/jetpack-compose";
import { fillMaxWidth, height } from "@expo/ui/jetpack-compose/modifiers";
import { seedColor } from "./theme";

export function PrimaryButton({
  label,
  disabled,
  onPress,
}: {
  label: string;
  disabled: boolean;
  onPress: () => void;
}) {
  return (
    <Host matchContents={{ vertical: true }} seedColor={seedColor}>
      <Button enabled={!disabled} onClick={onPress} modifiers={[fillMaxWidth(), height(56)]}>
        <Text style={{ typography: "titleMedium" }}>{label}</Text>
      </Button>
    </Host>
  );
}
