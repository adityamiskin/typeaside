import {
  Button,
  Host,
  List,
  Section,
  SwipeActions,
  Text,
  TextField,
  useNativeState,
} from "@expo/ui/swift-ui";
import { foregroundStyle, onSubmit, submitLabel } from "@expo/ui/swift-ui/modifiers";
import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import { useState } from "react";
import { Alert } from "react-native";

export default function DictionaryScreen() {
  const settings = useQuery(api.settings.mine);
  const addWord = useMutation(api.settings.addWord);
  const removeWord = useMutation(api.settings.removeWord);
  const word = useNativeState("");
  const [saving, setSaving] = useState(false);
  const words = [...(settings?.dictionary ?? [])].sort((a, b) => a.localeCompare(b));
  async function add() {
    const value = word.get().trim();
    if (!value || saving) return;
    setSaving(true);
    try {
      await addWord({ word: value });
      word.set("");
    } catch (error) {
      Alert.alert("Could not add word", error instanceof Error ? error.message : undefined);
    } finally {
      setSaving(false);
    }
  }
  return (
    <>
      <Host style={{ flex: 1 }}>
        <List>
          <Section>
            <TextField
              text={word}
              placeholder="Add a word"
              maxLength={100}
              modifiers={[submitLabel("done"), onSubmit(() => void add())]}
            />
          </Section>
          <Section>
            {settings !== undefined && !words.length && (
              <Text modifiers={[foregroundStyle({ type: "hierarchical", style: "secondary" })]}>
                Add names and terms TypeAside should spell right.
              </Text>
            )}
            {words.map((word) => (
              <SwipeActions key={word}>
                <Text>{word}</Text>
                <SwipeActions.Actions edge="trailing">
                  <Button
                    label="Delete"
                    role="destructive"
                    onPress={() =>
                      void removeWord({ word }).catch(() => Alert.alert("Could not remove word"))
                    }
                  />
                </SwipeActions.Actions>
              </SwipeActions>
            ))}
          </Section>
        </List>
      </Host>
    </>
  );
}
