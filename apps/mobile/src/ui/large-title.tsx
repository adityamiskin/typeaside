import { Column, Text, useMaterialColors } from "@expo/ui/jetpack-compose";
import { fillMaxWidth, padding } from "@expo/ui/jetpack-compose/modifiers";

// Material large top app bar title, scrolled with the list it heads.
export function LargeTitle({ title, supporting }: { title: string; supporting?: string }) {
  const { onSurfaceVariant } = useMaterialColors();
  return (
    <Column modifiers={[fillMaxWidth(), padding(16, 36, 16, 12)]}>
      <Text style={{ typography: "headlineMedium" }}>{title}</Text>
      {supporting ? (
        <Text color={onSurfaceVariant} style={{ typography: "bodyMedium" }}>
          {supporting}
        </Text>
      ) : null}
    </Column>
  );
}
