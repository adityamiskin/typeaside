import {
  Button,
  Form,
  Gauge,
  Host,
  HStack,
  Image,
  LabeledContent,
  Section,
  Spacer,
  SwipeActions,
  Text,
  VStack,
} from "@expo/ui/swift-ui";
import {
  background,
  buttonStyle,
  font,
  foregroundStyle,
  gaugeStyle,
  monospacedDigit,
  scrollContentBackground,
  tint,
} from "@expo/ui/swift-ui/modifiers";
import { Alert } from "react-native";
import {
  billingAction,
  DELETE_MESSAGE,
  isThisPhone,
  lastUsed,
  planLabel,
  resetsLabel,
  useAccount,
  usageLabel,
  usageProgress,
} from "@/account/account";
import { keyboardStatus, openKeyboardSettings, useKeyboardEnabled } from "@/account/keyboard";
import { colors } from "@/ui/theme";

const secondary = foregroundStyle({ type: "hierarchical", style: "secondary" });
const footnote = font({ textStyle: "footnote" });

export default function AccountScreen() {
  const { account, devices, signOut, openBilling, confirmRemoveDevice, deleteAccount } =
    useAccount();
  const keyboardEnabled = useKeyboardEnabled();
  const progress = account && usageProgress(account.usage);
  const email = account?.email;

  const promptDelete = () => {
    if (!email) return;
    Alert.prompt(
      "Delete account",
      DELETE_MESSAGE,
      [
        { text: "Cancel", style: "cancel" },
        {
          text: "Delete",
          style: "destructive",
          onPress: (typed) => {
            if (typed?.trim() === email) void deleteAccount(email);
            else Alert.alert("Email does not match your account.");
          },
        },
      ],
      "plain-text",
      "",
      "email-address",
    );
  };

  return (
    <Host style={{ flex: 1 }} colorScheme="dark" seedColor={colors.accent}>
      <Form modifiers={[scrollContentBackground("hidden"), background(colors.background)]}>
        {account && (
          <Section>
            <LabeledContent label="Email">
              <Text>{email ?? ""}</Text>
            </LabeledContent>
            <LabeledContent label="Plan">
              <HStack spacing={12}>
                <Text>{planLabel(account)}</Text>
                <Button
                  label={billingAction(account)}
                  onPress={openBilling}
                  modifiers={[
                    buttonStyle("borderless"),
                    ...(account.billingIssue ? [tint(colors.danger)] : []),
                  ]}
                />
              </HStack>
            </LabeledContent>
            <VStack alignment="leading" spacing={8}>
              <LabeledContent label="This week">
                <Text modifiers={[monospacedDigit()]}>{usageLabel(account.usage)}</Text>
              </LabeledContent>
              {progress != null && (
                <>
                  <Gauge value={progress} modifiers={[gaugeStyle("linearCapacity")]} />
                  <Text modifiers={[footnote, secondary]}>{resetsLabel(account.usage)}</Text>
                </>
              )}
            </VStack>
          </Section>
        )}
        <Section>
          <Button onPress={openKeyboardSettings}>
            <HStack>
              <Text modifiers={[foregroundStyle(colors.text)]}>Keyboard</Text>
              <Spacer />
              <Text modifiers={[secondary]}>{keyboardStatus(keyboardEnabled)}</Text>
              <Image systemName="chevron.right" size={13} color={colors.tertiary} />
            </HStack>
          </Button>
        </Section>
        {devices && devices.length > 0 && (
          <Section>
            <LabeledContent label="Devices">
              <Text>{devices.length}</Text>
            </LabeledContent>
            {devices.map((device) => (
              <SwipeActions key={device._id}>
                <LabeledContent
                  label={
                    <HStack spacing={6}>
                      <Text>{device.name}</Text>
                      {isThisPhone(device) && (
                        <Text modifiers={[footnote, secondary]}>This phone</Text>
                      )}
                    </HStack>
                  }
                >
                  <Text>{lastUsed(device.lastUsedAt)}</Text>
                </LabeledContent>
                {/* No destructive role: SwiftUI would drop the row before the confirmation. */}
                <SwipeActions.Actions edge="trailing" allowsFullSwipe={false}>
                  <Button
                    label="Remove"
                    systemImage="trash"
                    onPress={() => confirmRemoveDevice(device)}
                    modifiers={[tint(colors.danger)]}
                  />
                </SwipeActions.Actions>
              </SwipeActions>
            ))}
          </Section>
        )}
        <Section>
          <Button label="Sign out" onPress={signOut} />
          {email && <Button label="Delete account" role="destructive" onPress={promptDelete} />}
        </Section>
      </Form>
    </Host>
  );
}
