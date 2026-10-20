import type { ImageSourcePropType } from "react-native";

// Material Symbols (outlined, 24dp) as vector drawables for Compose `Icon`.
const sources = {
  chevron: require("./icons/chevron_right.xml"),
  computer: require("./icons/computer.xml"),
  dataUsage: require("./icons/data_usage.xml"),
  deleteForever: require("./icons/delete_forever.xml"),
  keyboard: require("./icons/keyboard.xml"),
  openInNew: require("./icons/open_in_new.xml"),
  phone: require("./icons/smartphone.xml"),
  premium: require("./icons/workspace_premium.xml"),
  signOut: require("./icons/logout.xml"),
};

export const accountIcons: Record<keyof typeof sources, ImageSourcePropType> = sources;
