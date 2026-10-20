import {
  RecordingPresets,
  requestRecordingPermissionsAsync,
  setAudioModeAsync,
  useAudioRecorder,
  useAudioRecorderState,
} from "expo-audio";
import { File } from "expo-file-system";
import * as Haptics from "expo-haptics";
import { useState } from "react";
import { dictate } from "./dictate";

const options = {
  ...RecordingPresets.HIGH_QUALITY,
  sampleRate: 16_000,
  numberOfChannels: 1,
  bitRate: 32_000,
  isMeteringEnabled: true,
};

export class MicrophoneDenied extends Error {
  constructor() {
    super("Microphone access is off. Turn it on in Settings.");
  }
}

// Records one take; `transcribe` turns it into cleaned text through the hosted API.
export function useDictation() {
  const recorder = useAudioRecorder(options);
  const status = useAudioRecorderState(recorder, 50);
  const [recording, setRecording] = useState(false);

  async function start() {
    if (!(await requestRecordingPermissionsAsync()).granted) throw new MicrophoneDenied();
    await setAudioModeAsync({ allowsRecording: true, playsInSilentMode: true });
    await recorder.prepareToRecordAsync();
    recorder.record();
    setRecording(true);
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Medium);
  }

  // Returns the recording's file; pass it to `transcribe`, which keeps it if dictation fails.
  async function stop() {
    await recorder.stop();
    await setAudioModeAsync({ allowsRecording: false });
    setRecording(false);
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    return recorder.uri;
  }

  async function cancel() {
    const uri = await stop();
    if (uri) new File(uri).delete();
  }

  return {
    recording,
    elapsedMs: status.durationMillis,
    // dBFS; -160 is what the recorder reports before it has a reading.
    metering: status.metering === -160 ? undefined : status.metering,
    start,
    stop,
    cancel,
  };
}

export async function transcribe(uri: string) {
  const result = await dictate(uri);
  new File(uri).delete();
  return result;
}
