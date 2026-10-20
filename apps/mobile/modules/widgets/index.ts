import { requireNativeModule } from "expo";

export type WidgetSnapshot = {
  wordsLeft: number | null;
  notes: {
    id: string;
    title: string;
    preview: string;
    color: string;
    updatedAt: number;
  }[];
};

export const widgets = requireNativeModule<{
  setSnapshot(json: string): Promise<void>;
  clear(): Promise<void>;
}>("Widgets");
