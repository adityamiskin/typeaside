import {
  Button,
  Circle,
  Form,
  Gauge,
  Host,
  HStack,
  Image,
  Label,
  LabeledContent,
  Section,
  Spacer,
  SwipeActions,
  Text,
  VStack,
  ZStack,
} from "@expo/ui/swift-ui";
import {
  background,
  buttonStyle,
  font,
  foregroundStyle,
  frame,
  gaugeStyle,
  monospacedDigit,
  padding,
  shapes,
  tint,
} from "@expo/ui/swift-ui/modifiers";
import { Alert } from "react-native";
import {
  billingAction,
  DELETE_MESSAGE,
  initial,
  isThisPhone,
  lastUsed,
  planLabel,
  resetsLabel,
  SIGN_OUT_MESSAGE,
  useAccount,
  usageLabel,
  usageProgress,
} from "@/account/account";
import { keyboardStatus, openKeyboardSettings, useKeyboardEnabled } from "@/account/keyboard";

const secondary = foregroundStyle({ type: "hierarchical", style: "secondary" });
const tertiary = foregroundStyle({ type: "hierarchical", style: "tertiary" });
const footnote = font({ textStyle: "footnote" });
const danger = tint("red");

export default function AccountScreen() {
  const { account, devices, signOut, openBilling, confirmRemoveDevice, deleteAccount } =
    useAccount();
  const keyboardEnabled = useKeyboardEnabled();
  const progress = account && usageProgress(account.usage);
  const email = account?.email;

  const confirmSignOut = () =>
    Alert.alert("Sign out?", SIGN_OUT_MESSAGE, [
      { text: "Cancel", style: "cancel" },
      { text: "Sign out", style: "destructive", onPress: signOut },
    ]);

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
    <Host style={{ flex: 1 }}>
      <Form>
        {account && (
          <Section>
            <HStack spacing={14}>
              <ZStack modifiers={[frame({ width: 56, height: 56 })]}>
                <Circle modifiers={[foregroundStyle("gray")]} />
                <Text
                  modifiers={[
                    font({ size: 24, weight: "semibold", design: "rounded" }),
                    foregroundStyle("white"),
                  ]}
                >
                  {initial(account)}
                </Text>
              </ZStack>
              <VStack alignment="leading" spacing={2}>
                <Text modifiers={[font({ textStyle: "title3", weight: "semibold" })]}>
                  {account.name || email || ""}
                </Text>
                {account.name && email ? (
                  <Text modifiers={[font({ textStyle: "subheadline" }), secondary]}>{email}</Text>
                ) : null}
              </VStack>
              <Spacer />
              <Text
                modifiers={[
                  font({ textStyle: "caption", weight: "semibold" }),
                  secondary,
                  padding({ horizontal: 8, vertical: 3 }),
                  background({ type: "hierarchical", style: "quinary" }, shapes.capsule()),
                ]}
              >
                {planLabel(account)}
              </Text>
            </HStack>
          </Section>
        )}
        {account && (
          <Section title="Usage" footer={<Text>{resetsLabel(account.usage)}</Text>}>
            <VStack alignment="leading" spacing={8}>
              <LabeledContent label="This week">
                <Text modifiers={[monospacedDigit()]}>{usageLabel(account.usage)}</Text>
              </LabeledContent>
              {progress != null && (
                <Gauge value={progress} modifiers={[gaugeStyle("linearCapacity")]} />
              )}
            </VStack>
            <Button onPress={openBilling} modifiers={account.billingIssue ? [danger] : []}>
              <Label title={billingAction(account)} systemImage="star.circle" />
            </Button>
          </Section>
        )}
        <Section
          title="Keyboard"
          footer={
            keyboardEnabled === false ? (
              <Text>Turn on TypeAside in Settings to dictate in any app.</Text>
            ) : undefined
          }
        >
          <Button onPress={openKeyboardSettings}>
            <HStack>
              <Label title="TypeAside keyboard" systemImage="keyboard" />
              <Spacer />
              <Text modifiers={[secondary]}>{keyboardStatus(keyboardEnabled)}</Text>
              <Image systemName="chevron.right" size={13} modifiers={[tertiary]} />
            </HStack>
          </Button>
        </Section>
        {devices && devices.length > 0 && (
          <Section title="Devices">
            {devices.map((device) => (
              <SwipeActions key={device._id}>
                <Label systemImage={isThisPhone(device) ? "iphone" : "laptopcomputer"}>
                  <VStack alignment="leading" spacing={2}>
                    <Text>{device.name}</Text>
                    <Text modifiers={[footnote, secondary]}>
                      {isThisPhone(device)
                        ? `This iPhone · ${lastUsed(device.lastUsedAt)}`
                        : lastUsed(device.lastUsedAt)}
                    </Text>
                  </VStack>
                </Label>
                {/* No destructive role: SwiftUI would drop the row before the confirmation. */}
                <SwipeActions.Actions edge="trailing" allowsFullSwipe={false}>
                  <Button
                    label="Remove"
                    systemImage="trash"
                    onPress={() => confirmRemoveDevice(device)}
                    modifiers={[danger]}
                  />
                </SwipeActions.Actions>
              </SwipeActions>
            ))}
          </Section>
        )}
        <Section>
          <Button
            label="Sign out"
            onPress={confirmSignOut}
            modifiers={[buttonStyle("borderless")]}
          />
          {email && <Button label="Delete account" role="destructive" onPress={promptDelete} />}
        </Section>
      </Form>
    </Host>
  );
}
