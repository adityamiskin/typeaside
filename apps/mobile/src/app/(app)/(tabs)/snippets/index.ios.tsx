import {
  BottomSheet,
  Button,
  Form,
  Host,
  List,
  NavigationStack,
  Section,
  Text,
  TextField,
  Toolbar,
  ToolbarItem,
  VStack,
  useNativeState,
} from "@expo/ui/swift-ui";
import {
  disabled,
  font,
  foregroundStyle,
  lineLimit,
  navigationTitle,
} from "@expo/ui/swift-ui/modifiers";
import { api } from "@typeaside/backend/convex/_generated/api";
import { useMutation, useQuery } from "convex/react";
import { Stack } from "expo-router";
import { useState } from "react";
import { Alert } from "react-native";

type Snippet = { trigger: string; text: string };
export default function SnippetsScreen() {
  const settings = useQuery(api.settings.mine);
  const [editing, setEditing] = useState<Snippet>();
  const snippets = [...(settings?.snippets ?? [])].sort((a, b) =>
    a.trigger.localeCompare(b.trigger),
  );
  return (
    <>
      <Stack.Toolbar placement="right">
        <Stack.Toolbar.Button
          icon="plus"
          accessibilityLabel="New snippet"
          onPress={() => setEditing({ trigger: "", text: "" })}
        />
      </Stack.Toolbar>
      <Host style={{ flex: 1 }}>
        <List>
          {settings !== undefined && !snippets.length && (
            <Text modifiers={[foregroundStyle({ type: "hierarchical", style: "secondary" })]}>
              Say a short phrase, get a saved block of text.
            </Text>
          )}
          {snippets.map((snippet) => (
            <Button key={snippet.trigger} onPress={() => setEditing(snippet)}>
              <VStack alignment="leading" spacing={4}>
                <Text
                  modifiers={[
                    font({ textStyle: "body" }),
                    foregroundStyle({ type: "hierarchical", style: "primary" }),
                  ]}
                >
                  {snippet.trigger}
                </Text>
                <Text
                  modifiers={[
                    foregroundStyle({ type: "hierarchical", style: "secondary" }),
                    lineLimit(2),
                  ]}
                >
                  {snippet.text}
                </Text>
              </VStack>
            </Button>
          ))}
        </List>
        <BottomSheet
          isPresented={!!editing}
          onIsPresentedChange={(shown) => {
            if (!shown) setEditing(undefined);
          }}
        >
          {editing && <SnippetForm snippet={editing} close={() => setEditing(undefined)} />}
        </BottomSheet>
      </Host>
    </>
  );
}

function SnippetForm({ snippet, close }: { snippet: Snippet; close: () => void }) {
  const triggerValue = useNativeState(snippet.trigger);
  const textValue = useNativeState(snippet.text);
  const [trigger, setTrigger] = useState(snippet.trigger);
  const [text, setText] = useState(snippet.text);
  const [saving, setSaving] = useState(false);
  const save = useMutation(api.settings.saveSnippet);
  const remove = useMutation(api.settings.deleteSnippet);
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
    <NavigationStack>
      <Toolbar>
        <Form modifiers={[navigationTitle("Snippets")]}>
          <Section footer={<Text>Say this phrase to insert the text</Text>}>
            <TextField
              text={triggerValue}
              onTextChange={setTrigger}
              placeholder="Trigger"
              maxLength={100}
            />
          </Section>
          <Section>
            <TextField
              text={textValue}
              onTextChange={setText}
              placeholder="Text"
              axis="vertical"
              maxLength={4000}
              modifiers={[lineLimit({ min: 4, max: 12 })]}
            />
          </Section>
          {snippet.trigger && (
            <Section>
              <Button
                label="Delete"
                role="destructive"
                onPress={() => void submit(true)}
                modifiers={[disabled(saving)]}
              />
            </Section>
          )}
        </Form>
        <Toolbar.Content>
          <ToolbarItem placement="cancellationAction">
            <Button label="Cancel" onPress={close} />
          </ToolbarItem>
          <ToolbarItem placement="confirmationAction">
            <Button
              label="Save"
              onPress={() => void submit()}
              modifiers={[disabled(!trigger.trim() || !text.trim() || saving)]}
            />
          </ToolbarItem>
        </Toolbar.Content>
      </Toolbar>
    </NavigationStack>
  );
}
