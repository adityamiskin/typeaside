import type { ImageSourcePropType } from "react-native";

// Material Symbols (outlined, 24dp) as vector drawables for Compose `Icon`.
const sources = {
  add: require("@/assets/icons/add.xml"),
  archive: require("@/assets/icons/archive.xml"),
  arrowBack: require("@/assets/icons/arrow_back.xml"),
  check: require("@/assets/icons/check.xml"),
  checkBox: require("@/assets/icons/check_box.xml"),
  checkBoxBlank: require("@/assets/icons/check_box_outline_blank.xml"),
  close: require("@/assets/icons/close.xml"),
  copy: require("@/assets/icons/content_copy.xml"),
  delete: require("@/assets/icons/delete.xml"),
  mic: require("@/assets/icons/mic_fill.xml"),
  noteAdd: require("@/assets/icons/note_add.xml"),
  pin: require("@/assets/icons/push_pin.xml"),
  pinned: require("@/assets/icons/push_pin_fill.xml"),
  refresh: require("@/assets/icons/refresh.xml"),
  share: require("@/assets/icons/share.xml"),
  stop: require("@/assets/icons/stop_fill.xml"),
  unarchive: require("@/assets/icons/unarchive.xml"),
};

export const icons: Record<keyof typeof sources, ImageSourcePropType> = sources;
