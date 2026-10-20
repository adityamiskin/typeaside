import { useNativeState } from "@expo/ui/jetpack-compose";
import {
  Column,
  Host,
  Icon,
  IconButton,
  LazyColumn,
  ListItem,
  OutlinedTextField,
  Text,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import { fillMaxSize, fillMaxWidth, padding } from "@expo/ui/jetpack-compose/modifiers";
import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import { useState } from "react";
import { Alert } from "react-native";
import { icons } from "@/ui/icons";
import { seedColor } from "@/ui/theme";

export default function DictionaryScreen() {
  const settings = useQuery(api.settings.mine);
  const addWord = useMutation(api.settings.addWord);
  const removeWord = useMutation(api.settings.removeWord);
  const value = useNativeState("");
  const [word, setWord] = useState("");
  const [saving, setSaving] = useState(false);
  const material = useMaterialColors({ seedColor });
  const words = [...(settings?.dictionary ?? [])].sort((a, b) => a.localeCompare(b));
  async function add() {
    if (!word.trim() || saving) return;
    setSaving(true);
    try {
      await addWord({ word });
      value.set("");
      setWord("");
    } catch (error) {
      Alert.alert("Could not add word", error instanceof Error ? error.message : undefined);
    } finally {
      setSaving(false);
    }
  }
  return (
    <>
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <LazyColumn modifiers={[fillMaxSize()]}>
          <OutlinedTextField
            value={value}
            onValueChange={setWord}
            maxLength={100}
            singleLine
            keyboardOptions={{ imeAction: "done", autoCorrectEnabled: false }}
            keyboardActions={{ onDone: () => void add() }}
            modifiers={[padding(16, 8, 16, 8), fillMaxWidth()]}
          >
            <OutlinedTextField.Label>
              <Text>Add a word</Text>
            </OutlinedTextField.Label>
            <OutlinedTextField.TrailingIcon>
              <IconButton enabled={!!word.trim() && !saving} onClick={() => void add()}>
                <Icon source={icons.add} size={24} contentDescription="Add word" />
              </IconButton>
            </OutlinedTextField.TrailingIcon>
          </OutlinedTextField>

          {settings !== undefined && words.length === 0 && (
            <Column
              horizontalAlignment="center"
              modifiers={[fillMaxWidth(), padding(24, 32, 24, 0)]}
            >
              <Text
                color={material.onSurfaceVariant}
                style={{ typography: "bodyMedium", textAlign: "center" }}
              >
                Add names and terms TypeAside should spell right.
              </Text>
            </Column>
          )}
          {words.map((word) => (
            <ListItem key={word}>
              <ListItem.HeadlineContent>
                <Text>{word}</Text>
              </ListItem.HeadlineContent>
              <ListItem.TrailingContent>
                <IconButton
                  onClick={() =>
                    void removeWord({ word }).catch(() => Alert.alert("Could not remove word"))
                  }
                >
                  <Icon source={icons.close} size={24} contentDescription={`Remove ${word}`} />
                </IconButton>
              </ListItem.TrailingContent>
            </ListItem>
          ))}
        </LazyColumn>
      </Host>
    </>
  );
}
