import { useEffect, useRef, useState } from "react";
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
export function useRecordBar({ busy, onRecorded, onError }: RecordBarProps) {
  const dictation = useDictation();
  const [levels, setLevels] = useState<number[]>([]);
  const heldRef = useRef(false);

  useEffect(() => {
    if (!dictation.recording) return setLevels([]);
    // Exactly 0 means the meter was read twice within one sampling window; skip it.
    if (dictation.level === 0) return;
    setLevels((current) => [...current, loudness(dictation.level)].slice(-BARS));
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

// The recorder spreads -60..0 dBFS over 0..1, but a quiet room already reads about -40 dBFS
// on Android, which left every bar near half height. Speech lives between -40 and -6.
function loudness(level: number) {
  const db = level * 60 - 60;
  return Math.min(1, Math.max(0, (db + 40) / 34));
}
