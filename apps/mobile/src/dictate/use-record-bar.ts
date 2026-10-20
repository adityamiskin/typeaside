import { useEffect, useRef, useState } from "react";
import type { PressableProps } from "react-native";
import { useDictation } from "@/lib/recorder";

export const BARS = 28;

export type RecordBarProps = {
  busy: boolean;
  onRecorded: (uri: string) => void;
  onError: (error: Error) => void;
};

// Tap toggles recording; holding records until release. Spread `press` on one Pressable for
// both states so releasing a hold still reaches it after the idle and recording views swap.
export function useRecordBar({ busy, onRecorded, onError }: RecordBarProps) {
  const dictation = useDictation();
  const [levels, setLevels] = useState<number[]>([]);
  const heldRef = useRef(false);

  useEffect(() => {
    if (dictation.recording) setLevels((current) => [...current, dictation.level].slice(-BARS));
    else setLevels([]);
  }, [dictation.recording, dictation.elapsedMs, dictation.level]);

  async function start() {
    await dictation.start().catch(onError);
  }
  async function stop() {
    const uri = await dictation.stop();
    if (uri) onRecorded(uri);
  }

  const press = {
    disabled: busy,
    accessibilityLabel: dictation.recording ? "Stop dictating" : "Dictate",
    onPress: () => void (dictation.recording ? stop() : start()),
    onLongPress: () => {
      if (dictation.recording) return;
      heldRef.current = true;
      void start();
    },
    onPressOut: () => {
      if (!heldRef.current) return;
      heldRef.current = false;
      void stop();
    },
  } satisfies PressableProps;

  // Oldest first, padded with `undefined` until the meter has filled the bar.
  const bars = Array.from({ length: BARS }, (_, index) => levels[index - (BARS - levels.length)]);

  return { dictation, press, bars, cancel: () => void dictation.cancel() };
}
