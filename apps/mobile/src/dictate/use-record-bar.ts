import { useLocalSearchParams, useRouter } from "expo-router";
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
// Outside a hold that Pressable is disabled while recording, so only `stop` ends a take.
export function useRecordBar({ busy, onRecorded, onError }: RecordBarProps) {
  const dictation = useDictation();
  const { start: requestedStart } = useLocalSearchParams<{ start?: string }>();
  const router = useRouter();
  const consumedStart = useRef(false);
  useEffect(() => {
    if (requestedStart !== "1") {
      consumedStart.current = false;
      return;
    }
    if (busy || consumedStart.current) return;
    consumedStart.current = true;
    router.setParams({ start: undefined });
    if (!dictation.recording) void dictation.start().catch(onError);
  }, [requestedStart, busy, dictation.recording, dictation.start, onError, router]);
  const [levels, setLevels] = useState<number[]>([]);
  const [holding, setHolding] = useState(false);
  const held = useRef(false);
  const starting = useRef<Promise<void> | undefined>(undefined);

  useEffect(() => {
    if (!dictation.recording) return setLevels([]);
    const level = loudness(dictation.metering);
    if (level === undefined) return;
    setLevels((current) => [...current, level].slice(-BARS));
  }, [dictation.recording, dictation.elapsedMs, dictation.metering]);

  function start() {
    starting.current ??= dictation
      .start()
      .catch(onError)
      .finally(() => {
        starting.current = undefined;
      });
    return starting.current;
  }
  async function stop() {
    await starting.current;
    const uri = await dictation.stop().catch(onError);
    if (uri) onRecorded(uri);
  }
  async function cancel() {
    await starting.current;
    await dictation.cancel().catch(onError);
  }

  const press = {
    disabled: busy || (dictation.recording && !holding),
    accessibilityLabel: "Dictate",
    onPress: () => void start(),
    onLongPress: () => {
      held.current = true;
      setHolding(true);
      void start();
    },
    onTouchEnd: () => {
      if (!held.current) return;
      held.current = false;
      setHolding(false);
      void stop();
    },
    onTouchCancel: () => {
      if (!held.current) return;
      held.current = false;
      setHolding(false);
      void cancel();
    },
  } satisfies PressableProps;

  // Oldest first, padded with `undefined` until the meter has filled the bar.
  const bars = Array.from({ length: BARS }, (_, index) => levels[index - (BARS - levels.length)]);

  return {
    dictation,
    press,
    bars,
    stop: () => void stop(),
    cancel: () => void cancel(),
  };
}

// A quiet room already reads about -40 dBFS on Android; speech lives between -40 and -6.
// Exactly 0 means the meter was read twice within one sampling window, so it is no reading.
export function loudness(db: number | undefined) {
  if (db === undefined || db === 0) return undefined;
  return Math.min(1, Math.max(0, (db + 40) / 34));
}
