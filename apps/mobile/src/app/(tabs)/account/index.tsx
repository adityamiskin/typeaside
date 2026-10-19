import {
  AlertDialog,
  Column,
  Host,
  HorizontalDivider,
  LazyColumn,
  LinearProgressIndicator,
  ListItem,
  Text,
  TextButton,
  TextField,
} from "@expo/ui/jetpack-compose";
import {
  background,
  clickable,
  combinedClickable,
  fillMaxSize,
  fillMaxWidth,
  padding,
} from "@expo/ui/jetpack-compose/modifiers";
import { useState } from "react";
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

const itemColors = {
  containerColor: colors.background,
  contentColor: colors.text,
  supportingContentColor: colors.secondary,
  trailingContentColor: colors.secondary,
};

const divider = <HorizontalDivider color={colors.separator} modifiers={[padding(16, 8, 16, 8)]} />;

export default function AccountScreen() {
  const { account, devices, signOut, openBilling, confirmRemoveDevice, deleteAccount } =
    useAccount();
  const keyboardEnabled = useKeyboardEnabled();
  const [deleting, setDeleting] = useState(false);
  const [typed, setTyped] = useState("");
  const progress = account && usageProgress(account.usage);
  const email = account?.email;
  const closeDelete = () => {
    setDeleting(false);
    setTyped("");
  };

  return (
    <Host style={{ flex: 1 }} colorScheme="dark" seedColor={colors.accent}>
      <LazyColumn modifiers={[fillMaxSize(), background(colors.background)]}>
        {account && (
          <ListItem colors={itemColors}>
            <ListItem.HeadlineContent>
              <Text>Email</Text>
            </ListItem.HeadlineContent>
            <ListItem.SupportingContent>
              <Text>{email ?? ""}</Text>
            </ListItem.SupportingContent>
          </ListItem>
        )}
        {account && (
          <ListItem colors={itemColors}>
            <ListItem.HeadlineContent>
              <Text>Plan</Text>
            </ListItem.HeadlineContent>
            <ListItem.SupportingContent>
              <Text>{planLabel(account)}</Text>
            </ListItem.SupportingContent>
            <ListItem.TrailingContent>
              <TextButton onClick={openBilling}>
                <Text color={account.billingIssue ? colors.danger : colors.accent}>
                  {billingAction(account)}
                </Text>
              </TextButton>
            </ListItem.TrailingContent>
          </ListItem>
        )}
        {account && (
          <ListItem colors={itemColors}>
            <ListItem.HeadlineContent>
              <Text>This week</Text>
            </ListItem.HeadlineContent>
            <ListItem.TrailingContent>
              <Text>{usageLabel(account.usage)}</Text>
            </ListItem.TrailingContent>
            {progress != null && (
              <ListItem.SupportingContent>
                <Column modifiers={[fillMaxWidth(), padding(0, 8, 0, 0)]}>
                  <LinearProgressIndicator
                    progress={progress}
                    color={colors.accent}
                    trackColor={colors.fill}
                    modifiers={[fillMaxWidth()]}
                  />
                  <Text modifiers={[padding(0, 6, 0, 0)]}>{resetsLabel(account.usage)}</Text>
                </Column>
              </ListItem.SupportingContent>
            )}
          </ListItem>
        )}
        {account && divider}
        <ListItem colors={itemColors} modifiers={[clickable(openKeyboardSettings)]}>
          <ListItem.HeadlineContent>
            <Text>Keyboard</Text>
          </ListItem.HeadlineContent>
          <ListItem.TrailingContent>
            <Text>{keyboardStatus(keyboardEnabled)}</Text>
          </ListItem.TrailingContent>
        </ListItem>
        {devices && devices.length > 0 && divider}
        {devices?.map((device) => (
          <ListItem
            key={device._id}
            colors={itemColors}
            modifiers={[
              combinedClickable({
                onClick: () => confirmRemoveDevice(device),
                onLongClick: () => confirmRemoveDevice(device),
              }),
            ]}
          >
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
          </ListItem>
        ))}
        {divider}
        <ListItem colors={itemColors} modifiers={[clickable(signOut)]}>
          <ListItem.HeadlineContent>
            <Text>Sign out</Text>
          </ListItem.HeadlineContent>
        </ListItem>
        {email && (
          <ListItem
            colors={{ ...itemColors, contentColor: colors.danger }}
            modifiers={[clickable(() => setDeleting(true))]}
          >
            <ListItem.HeadlineContent>
              <Text>Delete account</Text>
            </ListItem.HeadlineContent>
          </ListItem>
        )}
      </LazyColumn>
      {deleting && email && (
        <AlertDialog colors={{ containerColor: colors.elevated }} onDismissRequest={closeDelete}>
          <AlertDialog.Title>
            <Text>Delete account</Text>
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
                setDeleting(false);
                void deleteAccount(email);
              }}
            >
              <Text color={colors.danger}>Delete</Text>
            </TextButton>
          </AlertDialog.ConfirmButton>
          <AlertDialog.DismissButton>
            <TextButton onClick={closeDelete}>
              <Text>Cancel</Text>
            </TextButton>
          </AlertDialog.DismissButton>
        </AlertDialog>
      )}
    </Host>
  );
}
