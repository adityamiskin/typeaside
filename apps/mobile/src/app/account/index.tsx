import {
  AlertDialog,
  Box,
  Column,
  Host,
  Icon,
  IconButton,
  LazyColumn,
  LinearProgressIndicator,
  ListItem,
  Row,
  Text,
  TextButton,
  TextField,
  useMaterialColors,
} from "@expo/ui/jetpack-compose";
import {
  background,
  clickable,
  clip,
  fillMaxSize,
  fillMaxWidth,
  height,
  padding,
  Shapes,
  size,
  weight,
} from "@expo/ui/jetpack-compose/modifiers";
import { useRouter } from "expo-router";
import { useState } from "react";
import { type ImageSourcePropType, useColorScheme, View } from "react-native";
import {
  billingAction,
  DELETE_MESSAGE,
  initial,
  isThisPhone,
  lastUsed,
  resetsLabel,
  SIGN_OUT_MESSAGE,
  useAccount,
  usageLabel,
  usageProgress,
} from "@/account/account";
import { useKeyboardEnabled } from "@/account/keyboard";
import { icons } from "@/ui/icons";
import { seedColor } from "@/ui/theme";

// Android 16 Settings groups: rounded outer corners, tight inner corners, 2dp gaps.
const segment = (index: number, count: number) => [
  padding(16, 0, 16, index === count - 1 ? 0 : 2),
  clip(
    Shapes.RoundedCorner({
      topStart: index === 0 ? 24 : 4,
      topEnd: index === 0 ? 24 : 4,
      bottomStart: index === count - 1 ? 24 : 4,
      bottomEnd: index === count - 1 ? 24 : 4,
    }),
  ),
];

type Dialog = "signOut" | "delete";

export default function AccountScreen() {
  const { account, devices, signOut, openBilling, confirmRemoveDevice, deleteAccount } =
    useAccount();
  const keyboardEnabled = useKeyboardEnabled();
  const router = useRouter();
  const material = useMaterialColors({ seedColor });
  const dark = useColorScheme() === "dark";
  const [dialog, setDialog] = useState<Dialog>();
  const [typed, setTyped] = useState("");
  const progress = account && usageProgress(account.usage);
  const email = account?.email;
  const item = { containerColor: material.surfaceContainer };
  const closeDialog = () => {
    setDialog(undefined);
    setTyped("");
  };

  const header = (title: string) => (
    <Text
      color={material.primary}
      style={{ typography: "titleSmall" }}
      modifiers={[padding(32, 24, 32, 8)]}
    >
      {title}
    </Text>
  );
  const leading = (source: ImageSourcePropType, tint = material.onSurfaceVariant) => (
    <ListItem.LeadingContent>
      <Icon source={source} tint={tint} size={24} />
    </ListItem.LeadingContent>
  );

  return (
    <View style={{ flex: 1, backgroundColor: dark ? "#000" : material.surface }}>
      <Host style={{ flex: 1 }} seedColor={seedColor}>
        <LazyColumn modifiers={[fillMaxSize()]}>
          {account && (
            <Row
              verticalAlignment="center"
              horizontalArrangement={{ spacedBy: 16 }}
              modifiers={[fillMaxWidth(), padding(24, 24, 24, 8)]}
            >
              <Box
                contentAlignment="center"
                modifiers={[
                  size(48, 48),
                  clip(Shapes.Circle),
                  background(material.primaryContainer),
                ]}
              >
                <Text color={material.onPrimaryContainer} style={{ typography: "titleLarge" }}>
                  {initial(account)}
                </Text>
              </Box>
              <Column verticalArrangement={{ spacedBy: 4 }} modifiers={[weight(1)]}>
                <Text style={{ typography: "titleLarge" }}>{account.name || "Your account"}</Text>
                <Text color={material.onSurfaceVariant} style={{ typography: "bodyMedium" }}>
                  {email}
                </Text>
              </Column>
            </Row>
          )}

          {account && header(account.plan === "pro" ? "Pro membership" : "Free plan")}
          {account && (
            <ListItem colors={item} modifiers={segment(0, 2)}>
              {leading(icons.barChart)}
              <ListItem.HeadlineContent>
                <Text>{`${usageLabel(account.usage)} this week`}</Text>
              </ListItem.HeadlineContent>
              <ListItem.SupportingContent>
                <Column verticalArrangement={{ spacedBy: 8 }} modifiers={[padding(0, 8, 0, 0)]}>
                  {progress != null && (
                    <LinearProgressIndicator
                      progress={progress}
                      modifiers={[fillMaxWidth(), height(8)]}
                    />
                  )}
                  <Text>{resetsLabel(account.usage)}</Text>
                </Column>
              </ListItem.SupportingContent>
            </ListItem>
          )}
          {account && (
            <ListItem
              colors={{
                ...item,
                ...(account.billingIssue ? { contentColor: material.error } : {}),
              }}
              modifiers={[...segment(1, 2), clickable(openBilling)]}
            >
              {leading(
                icons.premium,
                account.billingIssue ? material.error : material.onSurfaceVariant,
              )}
              <ListItem.HeadlineContent>
                <Text>{billingAction(account)}</Text>
              </ListItem.HeadlineContent>
              <ListItem.TrailingContent>
                <Icon source={icons.openInNew} tint={material.onSurfaceVariant} size={20} />
              </ListItem.TrailingContent>
            </ListItem>
          )}

          {header("Preferences")}
          <ListItem
            colors={item}
            modifiers={[...segment(0, 1), clickable(() => router.push("/floating-mic"))]}
          >
            {leading(icons.mic)}
            <ListItem.HeadlineContent>
              <Text>Floating mic</Text>
            </ListItem.HeadlineContent>
            <ListItem.SupportingContent>
              <Text>Dictate into any app</Text>
            </ListItem.SupportingContent>
            {keyboardEnabled !== undefined && (
              <ListItem.TrailingContent>
                <Text color={material.onSurfaceVariant}>{keyboardEnabled ? "On" : "Off"}</Text>
              </ListItem.TrailingContent>
            )}
          </ListItem>

          {devices && devices.length > 0 && header("Devices")}
          {devices?.map((device, index) => (
            <ListItem key={device._id} colors={item} modifiers={segment(index, devices.length)}>
              {leading(isThisPhone(device) ? icons.phone : icons.computer)}
              <ListItem.HeadlineContent>
                <Text>{device.name}</Text>
              </ListItem.HeadlineContent>
              <ListItem.SupportingContent>
                <Text>
                  {isThisPhone(device)
                    ? `This phone · ${lastUsed(device.lastUsedAt)}`
                    : lastUsed(device.lastUsedAt)}
                </Text>
              </ListItem.SupportingContent>
              <ListItem.TrailingContent>
                <IconButton onClick={() => confirmRemoveDevice(device)}>
                  <Icon
                    source={icons.delete}
                    tint={material.onSurfaceVariant}
                    size={24}
                    contentDescription={`Remove ${device.name}`}
                  />
                </IconButton>
              </ListItem.TrailingContent>
            </ListItem>
          ))}

          <Column modifiers={[padding(0, 24, 0, 24)]}>
            <ListItem
              colors={item}
              modifiers={[...segment(0, email ? 2 : 1), clickable(() => setDialog("signOut"))]}
            >
              {leading(icons.signOut)}
              <ListItem.HeadlineContent>
                <Text>Sign out</Text>
              </ListItem.HeadlineContent>
            </ListItem>
            {email && (
              <ListItem
                colors={{ ...item, contentColor: material.error }}
                modifiers={[...segment(1, 2), clickable(() => setDialog("delete"))]}
              >
                {leading(icons.deleteForever, material.error)}
                <ListItem.HeadlineContent>
                  <Text>Delete account</Text>
                </ListItem.HeadlineContent>
              </ListItem>
            )}
          </Column>
        </LazyColumn>

        {dialog === "signOut" && (
          <AlertDialog onDismissRequest={closeDialog}>
            <AlertDialog.Icon>
              <Icon source={icons.signOut} size={24} />
            </AlertDialog.Icon>
            <AlertDialog.Title>
              <Text>Sign out?</Text>
            </AlertDialog.Title>
            <AlertDialog.Text>
              <Text>{SIGN_OUT_MESSAGE}</Text>
            </AlertDialog.Text>
            <AlertDialog.ConfirmButton>
              <TextButton
                onClick={() => {
                  closeDialog();
                  signOut();
                }}
              >
                <Text>Sign out</Text>
              </TextButton>
            </AlertDialog.ConfirmButton>
            <AlertDialog.DismissButton>
              <TextButton onClick={closeDialog}>
                <Text>Cancel</Text>
              </TextButton>
            </AlertDialog.DismissButton>
          </AlertDialog>
        )}
        {dialog === "delete" && email && (
          <AlertDialog onDismissRequest={closeDialog}>
            <AlertDialog.Icon>
              <Icon source={icons.deleteForever} tint={material.error} size={24} />
            </AlertDialog.Icon>
            <AlertDialog.Title>
              <Text>Delete account?</Text>
            </AlertDialog.Title>
            <AlertDialog.Text>
              <Column>
                <Text>{DELETE_MESSAGE}</Text>
                <TextField
                  singleLine
                  autoFocus
                  keyboardOptions={{ keyboardType: "email", autoCorrectEnabled: false }}
                  onValueChange={setTyped}
                  modifiers={[fillMaxWidth(), padding(0, 16, 0, 0)]}
                >
                  <TextField.Placeholder>
                    <Text>{email}</Text>
                  </TextField.Placeholder>
                </TextField>
              </Column>
            </AlertDialog.Text>
            <AlertDialog.ConfirmButton>
              <TextButton
                enabled={typed.trim() === email}
                onClick={() => {
                  closeDialog();
                  void deleteAccount(email);
                }}
              >
                <Text color={typed.trim() === email ? material.error : undefined}>Delete</Text>
              </TextButton>
            </AlertDialog.ConfirmButton>
            <AlertDialog.DismissButton>
              <TextButton onClick={closeDialog}>
                <Text>Cancel</Text>
              </TextButton>
            </AlertDialog.DismissButton>
          </AlertDialog>
        )}
      </Host>
    </View>
  );
}
