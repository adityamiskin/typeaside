import { useNativeState } from "@expo/ui/jetpack-compose";
import {
  Box,
  Button,
  Column,
  ExtendedFloatingActionButton,
  Host,
  Icon,
  LazyColumn,
  ListItem,
  ModalBottomSheet,
  OutlinedTextField,
  Row,
  Text,
  TextButton,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  align,
  clickable,
  fillMaxSize,
  fillMaxWidth,
  padding,
  weight,
} from "@expo/ui/jetpack-compose/modifiers";
import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import { Stack } from "expo-router";
import { useState } from "react";
import { Alert } from "react-native";
import { icons } from "@/ui/icons";
import { seedColor } from "@/ui/theme";

type Snippet = { trigger: string; text: string };
export default function SnippetsScreen() {
  const settings = useQuery(api.settings.mine);
  const [editing, setEditing] = useState<Snippet>();
  const material = useMaterialColors({ seedColor });
  const snippets = [...(settings?.snippets ?? [])].sort((a, b) =>
    a.trigger.localeCompare(b.trigger),
  );
  return (
    <>
      <Stack.Screen options={{ headerShown: true, title: "Snippets" }} />
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <Box modifiers={[fillMaxSize()]}>
          <LazyColumn modifiers={[fillMaxSize()]} contentPadding={{ bottom: 104 }}>
            {settings !== undefined && !snippets.length && (
              <Column
                horizontalAlignment="center"
                modifiers={[fillMaxWidth(), padding(24, 32, 24, 0)]}
              >
                <Text
                  color={material.onSurfaceVariant}
                  style={{ typography: "bodyMedium", textAlign: "center" }}
                >
                  Say a short phrase, get a saved block of text.
                </Text>
              </Column>
            )}
            {snippets.map((snippet) => (
              <ListItem key={snippet.trigger} modifiers={[clickable(() => setEditing(snippet))]}>
                <ListItem.HeadlineContent>
                  <Text>{snippet.trigger}</Text>
                </ListItem.HeadlineContent>
                <ListItem.SupportingContent>
                  <Text maxLines={2} overflow="ellipsis">
                    {snippet.text}
                  </Text>
                </ListItem.SupportingContent>
              </ListItem>
            ))}
          </LazyColumn>
          <ExtendedFloatingActionButton
            onClick={() => setEditing({ trigger: "", text: "" })}
            modifiers={[align("bottomEnd"), padding(16, 16, 16, 24)]}
          >
            <ExtendedFloatingActionButton.Icon>
              <Icon source={icons.add} size={24} contentDescription="New snippet" />
            </ExtendedFloatingActionButton.Icon>
            <ExtendedFloatingActionButton.Text>
              <Text>New snippet</Text>
            </ExtendedFloatingActionButton.Text>
          </ExtendedFloatingActionButton>
        </Box>
        {editing && <SnippetSheet snippet={editing} close={() => setEditing(undefined)} />}
      </Host>
    </>
  );
}

function SnippetSheet({ snippet, close }: { snippet: Snippet; close: () => void }) {
  const triggerValue = useNativeState(snippet.trigger);
  const textValue = useNativeState(snippet.text);
  const [trigger, setTrigger] = useState(snippet.trigger);
  const [text, setText] = useState(snippet.text);
  const [saving, setSaving] = useState(false);
  const save = useMutation(api.settings.saveSnippet);
  const remove = useMutation(api.settings.deleteSnippet);
  const material = useMaterialColors({ seedColor });
  async function submit(deleting = false) {
    if (saving) return;
    setSaving(true);
    try {
      if (deleting) await remove({ trigger: snippet.trigger });
      else await save({ trigger, text, originalTrigger: snippet.trigger || undefined });
      close();
    } catch (error) {
      Alert.alert("Could not save snippet", error instanceof Error ? error.message : undefined);
      setSaving(false);
    }
  }
  return (
    <ModalBottomSheet onDismissRequest={close} initialFullyExpanded>
      <Column
        verticalArrangement={{ spacedBy: 16 }}
        modifiers={[fillMaxWidth(), padding(24, 0, 24, 32)]}
      >
        <OutlinedTextField
          value={triggerValue}
          onValueChange={setTrigger}
          maxLength={100}
          singleLine
          modifiers={[fillMaxWidth()]}
        >
          <OutlinedTextField.Label>
            <Text>Trigger</Text>
          </OutlinedTextField.Label>
          <OutlinedTextField.SupportingText>
            <Text>Say this phrase to insert the text</Text>
          </OutlinedTextField.SupportingText>
        </OutlinedTextField>
        <OutlinedTextField
          value={textValue}
          onValueChange={setText}
          maxLength={4000}
          minLines={4}
          maxLines={8}
          modifiers={[fillMaxWidth()]}
        >
          <OutlinedTextField.Label>
            <Text>Text</Text>
          </OutlinedTextField.Label>
        </OutlinedTextField>
        <Row verticalAlignment="center" modifiers={[fillMaxWidth()]}>
          <Box modifiers={[weight(1)]}>
            {snippet.trigger ? (
              <TextButton enabled={!saving} onClick={() => void submit(true)}>
                <Text color={material.error}>Delete</Text>
              </TextButton>
            ) : null}
          </Box>
          <Button
            enabled={!!trigger.trim() && !!text.trim() && !saving}
            onClick={() => void submit()}
          >
            <Text>Save</Text>
          </Button>
        </Row>
      </Column>
    </ModalBottomSheet>
  );
}
