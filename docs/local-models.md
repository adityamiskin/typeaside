# On-device models

Settings → Dictation lists the available models. Download several, then use **Use** to switch. The dot marks the active model; its action is **Remove**. To remove another installed model, select it first. Only one download runs at a time. Source → On device appears when a model is installed.

| Model key             | Languages                                          | Assets   | Engine                                   |
| --------------------- | -------------------------------------------------- | -------- | ---------------------------------------- |
| `parakeet-v3`         | 25 European languages                              | 671 MB   | ONNX, int8 TDT                           |
| `parakeet-unified-en` | English                                            | 732 MB   | transcribe.cpp, Q8_0 GGUF, GPU           |
| `bark-3-clean`        | English                                            | 685 MB   | ONNX, int8 TDT, split decoder and joiner |
| `bark-3`              | English                                            | 685 MB   | ONNX, int8 TDT, split decoder and joiner |
| `bark-3-plus`         | English                                            | 685 MB   | ONNX, int8 TDT, split decoder and joiner |
| `cohere`              | 14, including Arabic, Chinese, Japanese and Korean | 2,128 MB | ONNX, q4, float32 activations and caches |
| `whisper-turbo`       | 99                                                 | 620 MB   | transcribe.cpp, Q5_K_M GGUF, GPU         |

Sizes use decimal MB and exclude the shared ONNX runtime, which every model downloads for the VAD. Every model asset is pinned to a Hugging Face commit with its byte size and SHA-256 in `src/local/assets.rs`. All 35 catalog entries were checked against downloaded bytes, including Cohere's external weight files.

Cohere uses the **Spoken language** setting; Auto uses English. Whisper detects the language when Auto is selected. Parakeet v3 detects language itself. Every local model gets the same preprocessing from [Silero VAD](https://github.com/snakers4/silero-vad) v6.2.3 (MIT, `licenses/silero-vad-MIT.txt`), installed into the shared runtime directory with any model and pinned to its upstream commit, byte size and SHA-256. The worker trims non-speech edges, keeping 200 ms of padding so the engine's release tail and optional pre-roll still reach the model, and returns an empty transcript when it finds no speech. That replaces the energy gate's "No speech detected" for local dictation, since the VAD hears quiet speech the gate rejects. Recordings longer than 25 s are split at the longest pause in the 10 s before each boundary, or at the least speech-like frame when there is none, and chunks without speech are skipped. Whisper decodes with segment timestamps so it keeps seeking within a chunk instead of stopping after its first segments.

Existing configurations default to Parakeet v3. The ONNX runtime and VAD live in shared storage and count toward each model's installation marker, so models installed before the VAD shipped show as not installed until downloaded again, which fetches only the 2.3 MB VAD. Removing a model preserves the shared runtime and other models. When Source is On device, the selected model loads in the background at launch and after a model or language change. **Unload when idle** (5 min, 30 min, Never) frees the worker after that long unused. The default, 30 min, keeps a working session warm while returning the roughly 1.3 GB a loaded Parakeet v3 worker holds on 8 to 16 GB laptops. Switching Source away unloads it. ONNX models run no warm-up inference: the first transcription after loading measured the same as later ones. GGUF models transcribe 1.5 s of silence while loading so the GPU compiles its kernels before the first dictation.

## GPU acceleration

`parakeet-unified-en` and `whisper-turbo` run on [transcribe.cpp](https://github.com/handy-computer/transcribe.cpp) 0.2.4 (ggml): Vulkan on Linux x86_64 and Windows x64, Metal on macOS, CPU elsewhere. The worker prefers an integrated GPU, then a discrete one, then the CPU. A discrete laptop GPU powers down about 25 s after use even with the model loaded, so every dictation after a pause pays its wake-up: on this machine a 4.9 s clip took 1,060 to 1,120 ms on the RTX 4060 after 35 s idle, against 170 to 210 ms on the Radeon 780M. `TYPEASIDE_LOCAL_DEVICE=gpu` prefers the discrete GPU (desktops) and `=cpu` forces the CPU. If the GPU fails to load the model or to run the warm-up, the worker reloads it on the CPU. GPU sessions use one CPU thread for the per-token decoder. Each run gets a fresh session so a long recording's scratch memory is released.

ONNX Parakeet v3 stays: its GGUF conversion (`handy-computer/parakeet-tdt-0.6b-v3-gguf`, Q8_0) measured faster on the GPU (`long-en` 1,007 ms on the RTX 4060 and 1,257 ms on the 780M against 1,999 ms ONNX) with WER 0.066 against 0.096, but 3x slower on the CPU (median RTF 0.112 against 0.034), and ONNX Runtime is still needed for Bark, Cohere and the VAD. Whisper moved because whisper-rs and transcribe.cpp each link their own ggml and cannot share a binary; its CPU speed is close to whisper-rs and its GPU speed is new.

Building needs CMake, the Vulkan headers and loader, `glslc` and SPIRV-Headers on Linux and Windows (Arch: `vulkan-headers spirv-headers shaderc vulkan-icd-loader`; Ubuntu 24.04: `libvulkan-dev glslc spirv-headers`; Windows: the LunarG SDK plus `vcpkg install spirv-headers`). The binary links the Vulkan loader; Windows delay-loads it so the app starts without one, but on such a machine the worker cannot load GGUF models.

### Streaming while the shortcut is held

With Source On device, every capture opens a stream to the worker and the recorder feeds it 160 ms blocks, pre-roll first, then everything through the 200 ms release tail. Only a model that advertises streaming on a GPU streams: today `parakeet-unified-en` on Vulkan or Metal, buffered with its default 5.6 s left context, 1.04 s chunk and 1.04 s right context. A CPU stream would re-encode every overlapping window, so CPU workers and other models ignore the audio and transcribe the whole clip after release as before. Streams skip the VAD.

Release finalizes the stream immediately, so it ends before a later capture opens its own. The whole clip is transcribed offline, through the VAD, when the stream fails, returns no text, or finalize runs past 1.5 s (the worker cancels it). A tap, Escape or discarded capture abandons the stream. Loading such a model also runs a 1 s silent stream; if that fails the model stays offline-only. Each dictation logs `typeaside local: stream=… finish=… ms` with `TYPEASIDE_TRACE=1`.

## Repeatable verification

The example requires an explicit isolated home. It never reads the daily-driver config or model directory. Its child workers inherit that home. It asserts that a pre-selection config still selects v3, transcribes the fixed clips with every installed model, prints cold-load-inclusive timings, and exercises switching models/languages within one parent process.

```bash
export TYPEASIDE_HOME="$(mktemp -d /tmp/typeaside-models.XXXXXX)"
cargo build --example local_models
for model in parakeet-v3 cohere whisper-turbo; do
  target/debug/examples/local_models --download "$model"
done
target/debug/examples/local_models --list
target/debug/examples/local_models tests/fixtures/local-models-en.wav tests/fixtures/local-models-fr.wav
# Optional focused Whisper language detection check:
target/debug/examples/local_models tests/fixtures/local-models-en.wav tests/fixtures/local-models-fr.wav --model whisper-turbo --auto
```

To exercise cancellation in a fresh isolated home, `--download cohere 500` cancels after 500 ms. It should return `Download cancelled`; `--list` must show it uninstalled. Retry without the delay to install it.

The clips were generated with eSpeak NG 1.52.0, voices `en-us` and `fr-fr`, speed 145, then converted with ffmpeg to 16 kHz mono signed 16-bit PCM. They contain original verification text:

- English: “The meeting is tomorrow at three in the afternoon. Please bring your notes.”
- French: “Bonjour, la réunion est demain à trois heures. Merci de préparer vos notes.”

```bash
espeak-ng -v en-us -s 145 -w /tmp/local-en.wav 'The meeting is tomorrow at three in the afternoon. Please bring your notes.'
espeak-ng -v fr-fr -s 145 -w /tmp/local-fr.wav 'Bonjour, la réunion est demain à trois heures. Merci de préparer vos notes.'
ffmpeg -i /tmp/local-en.wav -ar 16000 -ac 1 tests/fixtures/local-models-en.wav
ffmpeg -i /tmp/local-fr.wav -ar 16000 -ac 1 tests/fixtures/local-models-fr.wav
```

## Benchmark

`--bench MANIFEST OUT.jsonl` builds each clip in `tests/fixtures/local-models-bench.json` from the two fixtures (trimmed, repeated with silence gaps) and, per model and clip, runs one cold load plus `iterations` warm runs. Each run writes a JSONL record with `audio_ms`, `phase`, `elapsed_ms`, `rtf` and `wer`. WER is word Levenshtein distance over lowercase words with punctuation removed, so `3` for "three" counts as an error. Release replay trims the clip to its last non-silent sample, then cuts 0, 100 and 200 ms more, and records whether the reference's final word is still the transcript's final word. A summary record per model gives warm median RTF, p95 warm latency, mean warm WER and release survival.

Clips: `short-en` 2.9 s, `medium-en` 10.3 s, `medium-fr` 10.5 s, `long-en` 44.3 s.

```bash
export TYPEASIDE_HOME="$(mktemp -d -p ~/.cache)"
cargo build --release --example local_models
for model in parakeet-v3 parakeet-unified-en whisper-turbo; do
  target/release/examples/local_models --download "$model"
done
target/release/examples/local_models --bench tests/fixtures/local-models-bench.json "$TYPEASIDE_HOME/bench.jsonl"
```

Linux x86_64, AMD Ryzen 9 8945HS (16 threads), release build, CPU only, 5 warm iterations:

| Model           | Median RTF | p95 ms | Mean WER | Median cold ms | Final word kept at 0/100/200 ms cut |
| --------------- | ---------- | ------ | -------- | -------------- | ----------------------------------- |
| `parakeet-v3`   | 0.040      | 2,319  | 0.096    | 2,295          | 4/3/3 of 4                          |
| `whisper-turbo` | 1.223      | 24,195 | 0.278    | 16,058         | 4/4/4 of 4                          |

Warm medians per clip, ms (WER):

| Model           | short-en      | medium-en     | medium-fr     | long-en       |
| --------------- | ------------- | ------------- | ------------- | ------------- |
| `parakeet-v3`   | 114 (0.00)    | 353 (0.08)    | 400 (0.31)    | 2,259 (0.00)  |
| `whisper-turbo` | 15,276 (0.11) | 12,861 (0.08) | 11,836 (0.15) | 23,737 (0.77) |

Parakeet's French errors are "de ma toine" for "demain à trois", and "nôtes"/"eaux" for "notes" once 100 ms is cut. Whisper pads every window to 30 s, so short clips cost as much as long ones on CPU, and on `long-en` it transcribed only two of the eight repeated sentences. The final-word check still passes there, so it measures release truncation, not completeness.

### Silero VAD before/after

Same machine, release build, `--bench` with `iterations` lowered to 2, models `parakeet-v3` and `whisper-turbo`. Before is `238e16d`; after adds the VAD and turns on Whisper timestamps.

| Model           | Median RTF    | Mean WER      | `long-en` WER | `medium-fr` WER | Final word kept at 0/100/200 ms cut |
| --------------- | ------------- | ------------- | ------------- | --------------- | ----------------------------------- |
| `parakeet-v3`   | 0.034 → 0.059 | 0.096 → 0.096 | 0.00 → 0.00   | 0.31 → 0.31     | 4/3/3 → 4/3/3 of 4                  |
| `whisper-turbo` | 1.227 → 1.130 | 0.278 → 0.259 | 0.77 → 0.77   | 0.15 → 0.08     | 4/4/4 → 4/4/4 of 4                  |

The Parakeet RTF gap is run-to-run noise: two interleaved Parakeet-only reruns with 5 iterations measured 0.038 and 0.033 before, 0.034 and 0.045 after. The VAD itself took 14 ms on 4.9 s and 148 ms on 52 s of audio. `long-en` was split into two chunks at a 750 ms gap, yet Whisper still returns two of its eight sentences. Its sentences are sample-identical, so this clip does not separate Whisper's handling of repetition from long audio.

A 52.3 s clip of eight different English fixtures with 0.8 to 3 s of digital silence between them, and the same clip under pink noise at amplitude 0.003, measure long audio with varied speech. Sentences kept out of 8, cold load included:

| Model           | Clean before | Clean after | Noisy before | Noisy after |
| --------------- | ------------ | ----------- | ------------ | ----------- |
| `parakeet-v3`   | 8 (5.7 s)    | 8 (5.0 s)   | 2 (3.0 s)    | 7 (5.5 s)   |
| `whisper-turbo` | 5 (43.0 s)   | 8 (68.7 s)  | 5 (47.9 s)   | 8 (82.2 s)  |

VAD chunks alone left Whisper at 5 of 8. It dropped sentences inside a 23 s chunk until `no_timestamps` was removed, which lets whisper.cpp seek per segment; that costs more encoder passes on CPU. The first cut, searching only the last 3 s before the 25 s boundary, missed a 3 s pause and split "Open source slash config.rs", which Parakeet then lost; searching the last 10 s fixed it. On 6 s of digital silence, Whisper returned "Sous-titrage Société Radio-Canada" before and the VAD now returns no speech in 15 ms. Parakeet returned nothing on silence or noise either way.

### transcribe.cpp on the GPU

After the VAD (`a519f00`), same machine, release build, 5 warm iterations. iGPU is the default device (Radeon 780M, RADV), dGPU is `TYPEASIDE_LOCAL_DEVICE=gpu` (RTX 4060 Laptop, NVIDIA 615.78), CPU is `=cpu`. ONNX Parakeet v3 was rerun in the same session as the baseline; whisper-rs is the post-VAD row above. Back-to-back warm runs keep the dGPU awake, so its numbers exclude the wake-up described in GPU acceleration. Unified English's WER and final-word columns exclude `medium-fr`, which an English model cannot transcribe (WER 0.85).

| Model                        | Device | Median RTF | p95 ms | Mean WER | Median cold ms | Final word kept at 0/100/200 ms cut |
| ---------------------------- | ------ | ---------- | ------ | -------- | -------------- | ----------------------------------- |
| `parakeet-v3` (ONNX)         | CPU    | 0.037      | 1,652  | 0.096    | 1,907          | 4/3/3 of 4                          |
| `parakeet-unified-en`        | iGPU   | 0.027      | 1,098  | 0.090    | 823            | 3/3/3 of 3                          |
| `parakeet-unified-en`        | dGPU   | 0.015      | 583    | 0.090    | 686            | 3/3/3 of 3                          |
| `parakeet-unified-en`        | CPU    | 0.051      | 2,269  | 0.090    | 1,019          | 3/3/3 of 3                          |
| `whisper-turbo` (whisper-rs) | CPU    | 1.130      |        | 0.259    |                | 4/4/4 of 4                          |
| `whisper-turbo`              | iGPU   | 0.119      | 2,989  | 0.259    | 2,388          | 4/4/4 of 4                          |
| `whisper-turbo`              | dGPU   | 0.025      | 527    | 0.259    | 888            | 4/4/4 of 4                          |
| `whisper-turbo`              | CPU    | 0.699      | 14,597 | 0.259    | 14,665         | 4/4/4 of 4                          |

Warm medians per clip, ms:

| Model                 | Device | short-en | medium-en | medium-fr | long-en |
| --------------------- | ------ | -------- | --------- | --------- | ------- |
| `parakeet-v3` (ONNX)  | CPU    | 123      | 355       | 397       | 1,635   |
| `parakeet-unified-en` | iGPU   | 119      | 273       |           | 943     |
| `parakeet-unified-en` | dGPU   | 65       | 155       |           | 576     |
| `parakeet-unified-en` | CPU    | 174      | 632       |           | 2,126   |
| `whisper-turbo`       | iGPU   | 790      | 1,373     | 1,245     | 2,088   |
| `whisper-turbo`       | dGPU   | 204      | 256       | 254       | 525     |
| `whisper-turbo`       | CPU    | 7,237    | 7,230     | 7,187     | 14,370  |

Unified English's errors are all "3" for "three". Whisper GGUF scored the same per-clip WER as whisper-rs, including two of eight sentences on `long-en`. On the 52.3 s varied clip from the VAD section, clean and under pink noise, both GGUF models kept 8 of 8 sentences; cold-load-inclusive Whisper took 6.0 s on the iGPU and 33.6 s on the CPU (whisper-rs: 68.7 s), Unified English 1.5 s and 2.9 s.

The silent warm-up at load matters on the dGPU: the first Whisper transcription after loading took 1,012 ms without it and 232 ms with it (later runs 255 to 261 ms). On the iGPU the first run matched later ones either way. Mesa and NVIDIA keep their compiled shaders in the user's cache; the very first load on a machine took 4.1 s on the 780M and 11.7 s on the RTX 4060.

### Streaming release-to-text

`--stream-bench MANIFEST OUT.jsonl [SPEED]` replays each clip into a live stream in 160 ms blocks at SPEED times real time, releases, and times the text against a warm offline run of the same clip. Each iteration first abandons a 1 s stream, as a tap does. Real time (SPEED 1), `parakeet-unified-en`, 3 iterations, medians in ms:

| Device | `medium-en` 10.3 s stream | offline | `long-en` 44.3 s stream | offline | Used the stream | WER stream / offline |
| ------ | ------------------------- | ------- | ----------------------- | ------- | --------------- | -------------------- |
| iGPU   | 191                       | 249     | 192                     | 840     | 6 of 6          | 0.08 / 0.08          |
| dGPU   | 161                       | 160     | 151                     | 543     | 6 of 6          | 0.08 / 0.08          |
| CPU    | 397                       | 372     | 1,604                   | 1,606   | 0 of 6          | 0.08 / 0.08          |

Release-to-text no longer grows with the recording on a GPU; at 10 s the offline run is already short, so streaming only helps the iGPU. On the CPU the stream is declined and the offline fallback adds nothing measurable. Feeding a stream should also keep the dGPU awake while recording, so a streamed dictation would not pay its idle wake-up at release; that was not measured. Whisper, which cannot stream, declined in 0 ms at release and fell back. HEX's 8 s warm-up stream loaded in 3,013 ms on the iGPU against 503 ms for 1 s, with the same first-dictation latency (short-en 139 against 126 ms), so the warm-up is 1 s. The finalize-deadline fallback was not triggered in these runs.

## Verification on Linux x86_64

Debug build, CPU ONNX Runtime 1.24.4, four inference threads. English audio: 4,875 ms; French audio: 4,987 ms. These are single-run timings, including model loading, not a comparative accuracy benchmark.

```text
parakeet-v3 en: "The meeting is tomorrow at 3 in the afternoon. Please bring your notes." | audio=4875ms elapsed=3625ms (cold load included)
parakeet-v3 fr: "Bonjour, la réunion est demain à 3h. Merci de préparer vos notes." | audio=4987ms elapsed=1761ms (cold load included)
cohere en: "The meeting is tomorrow at 3 in the afternoon. Please bring your notes." | audio=4875ms elapsed=3847ms (cold load included)
cohere fr: "Bonjour, la réunion est demain à trois heures. Merci de préparer vos notes." | audio=4987ms elapsed=3234ms (cold load included)
```

Cancellation after 500 ms returned `Download cancelled` and left all models uninstalled in a separate fresh home.

Checks: `cargo build`, `cargo clippy`, `cargo test`, `bun run build`, and `bun run check` passed. Cargo currently discovers zero tests; the transcription example supplies the exercised runtime verification. `bun run codegen` could not run because this checkout has no `CONVEX_DEPLOYMENT`. No backend functions changed and no deployment was configured. No E2E suite ran.

The real GPUI app was launched with `--demo`, `TYPEASIDE_HOME=/tmp/typeaside-model-verification`, `TYPEASIDE_DEMO_PAGE=settings`, `TYPEASIDE_DEMO_SETTINGS_SCROLL=Dictation`, `TYPEASIDE_DEMO_MODEL=installed`, and `TYPEASIDE_THEME=dark`. Demo actions are read-only and its instance socket is separate. The screenshot shows v3 installed and the other download actions using the demo fixture.

![On-device model settings in the real app](assets/settings.png)

Unverified: macOS/Windows runtime execution, other supported languages, VAD chunking on real (non-synthetic) long recordings, and live UI download/selection/removal interactions.

## Attribution

The decoder/cache handling, from its Moonshine and Cohere engines, and the Cohere feature-extractor structure were adapted from [voxtype](https://github.com/peteonrails/voxtype), commit `dc499ad47444ba6ea80d8750aa3581c23120821c`, MIT, Copyright 2025 Peter Jackson. The license is retained in `licenses/voxtype-MIT.txt` and ported files carry credit. The Cohere port corrects framing, Slaney filter normalization and per-feature variance to match the upstream Hugging Face processor. Whisper and Parakeet Unified English run on [transcribe.cpp](https://github.com/handy-computer/transcribe.cpp) (MIT, built on MIT ggml), built for a portable x86 baseline (AVX2, FMA, F16C) without system BLAS via `.cargo/config.toml`. One decoder thread on GPU sessions, per-run session rotation and the 1.5 s minimum input were adapted from [HEX](https://github.com/anomalyco/hex), commit `7160492a4d415c24184f57012ce3a6dae1af4e99`, MIT, Copyright 2026 Christopher Langton, retained in `licenses/hex-MIT.txt`.

Parakeet Unified English is NVIDIA's [parakeet-unified-en-0.6b](https://huggingface.co/nvidia/parakeet-unified-en-0.6b), CC-BY-4.0, used as handy-computer's [Q8_0 GGUF conversion](https://huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf) at commit `7e948f21`. Whisper large-v3-turbo is OpenAI's, MIT, used as handy-computer's [Q5_K_M GGUF conversion](https://huggingface.co/handy-computer/whisper-large-v3-turbo-gguf) at commit `18ada752`.

The Underdog Bark 3.0 family is by Conway Research ([Standard](https://huggingface.co/ConwayResearch/Underdog-Bark-3.0), [Clean](https://huggingface.co/ConwayResearch/Underdog-Bark-3.0-Clean), [Plus](https://huggingface.co/ConwayResearch/Underdog-Bark-3.0-Plus)), derived from NVIDIA's parakeet-tdt-0.6b-v3 and Moondream's parakeet-ultra, CC-BY-4.0. Clean drops fillers and false starts; Standard and Plus keep every word, and Plus makes fewer mistakes. Each was converted to int8 sherpa-onnx from its `model.bark` at the pinned Hugging Face revision, with the converter that produced velddev's [Standard conversion](https://huggingface.co/velddev/Underdog-Bark-3.0-sherpa-onnx) (Standard's int8 encoder is byte-identical). The encoder's weights are split into three external data files of at most 212 MB, served from `typeaside.app/models` (R2), since no upstream host publishes Clean or Plus as ONNX. All three reuse v3's `nemo128.onnx` preprocessor and get the leading silence they were evaluated with: 0.25 s for Standard and Plus, none for Clean. NOTICE files are retained in `licenses/underdog-bark-3.0*-NOTICE.txt`. Standard often omits capitals and punctuation; Clean and Plus usually include them.

Implemented by Codex GPT-6.1-Sol via T3 Code.
