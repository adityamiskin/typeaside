import { useEffect, useState } from "react";
import type { PressableProps } from "react-native";
import { useDictation } from "@/lib/recorder";

export const BARS = 40;

export type RecordBarProps = {
  busy: boolean;
  // With no takes yet the idle button becomes the screen's centered call to action.
  hero: boolean;
  onRecorded: (uri: string) => void;
  onError: (error: Error) => void;
};

// Tap toggles recording; holding records until release. Spread `press` on one Pressable for
// both states so releasing a hold still reaches it after the idle and recording views swap.
// Outside a hold that Pressable is disabled while recording, so only `stop` ends a take.
export function useRecordBar({ busy, onRecorded, onError }: RecordBarProps) {
  const dictation = useDictation();
  const [levels, setLevels] = useState<number[]>([]);
  const [held, setHeld] = useState(false);

  useEffect(() => {
    if (!dictation.recording) return setLevels([]);
    const level = loudness(dictation.metering);
    if (level === undefined) return;
    setLevels((current) => [...current, level].slice(-BARS));
  }, [dictation.recording, dictation.elapsedMs, dictation.metering]);

  async function start() {
    await dictation.start().catch(onError);
  }
  async function stop() {
    const uri = await dictation.stop();
    if (uri) onRecorded(uri);
  }

  const press = {
    disabled: busy || (dictation.recording && !held),
    accessibilityLabel: "Dictate",
    onPress: () => void start(),
    onLongPress: () => {
      setHeld(true);
      void start();
    },
    onPressOut: () => {
      if (!held) return;
      setHeld(false);
      void stop();
    },
  } satisfies PressableProps;

  // Oldest first, padded with `undefined` until the meter has filled the bar.
  const bars = Array.from({ length: BARS }, (_, index) => levels[index - (BARS - levels.length)]);

  return {
    dictation,
    press,
    bars,
    stop: () => void stop(),
    cancel: () => void dictation.cancel(),
  };
}

// A quiet room already reads about -40 dBFS on Android; speech lives between -40 and -6.
// Exactly 0 means the meter was read twice within one sampling window, so it is no reading.
export function loudness(db: number | undefined) {
  if (db === undefined || db === 0) return undefined;
  return Math.min(1, Math.max(0, (db + 40) / 34));
}
