# Dictation latency

Release-to-text latency measured through the real app, base `20fa723`. Host: Linux x86_64, Ryzen 9 8945HS, Radeon 780M, RTX 4060 Laptop, firmware power profile `quiet`.

## Method

`scripts/e2e-latency.py` exports committed source, patches only the delivery callback to write a timestamped JSON file, and drives a release build with `typeaside dictation start/stop`. Each run has its own `TYPEASIDE_HOME`, `XDG_RUNTIME_DIR`, private D-Bus session and instance socket, and records from a silent PipeWire null sink. Physical hotkeys are disabled in the export. The daily-driver app is never contacted.

Latency runs from the CLI stop to the delivery callback: the 200 ms release tail, speech, cleanup and engine overhead. Clipboard ownership and key injection are **not** measured. History's `latency_ms` starts after recording and omits the tail.

Cold means models unloaded at key press, with filesystem and shader caches kept. The existing key-press speech preload still runs. Warm runs drop one initial dictation per clip. Each cell has five retained samples; p90 is nearest rank, so the maximum of five. Fixtures are real LibriSpeech utterances (5 s: row 18, 10 s: rows 19/20, 30 s: rows 25/30/32/36/37) padded with silence. The one-word clip is "True" cropped from row 37.

GPU selection is scoped to the worker with an explicit Vulkan ICD. The app's integrated-UI loader filter is otherwise inherited by workers on this hybrid host, which is a separate existing issue.

## Baseline

Parakeet v3, **median / p90 ms**, 240 samples. The harness waits 600 ms after start before playback.

| Device      | Models | AI editing |        1 word |           5 s |          10 s |          30 s |
| ----------- | ------ | ---------- | ------------: | ------------: | ------------: | ------------: |
| CPU         | warm   | off        |     367 / 388 |     552 / 576 | 1,129 / 1,157 | 3,893 / 3,915 |
| CPU         | cold   | off        |     373 / 381 |     553 / 593 | 1,138 / 1,154 | 3,894 / 4,268 |
| CPU         | warm   | on         |     475 / 479 |   864 / 1,057 | 1,836 / 2,223 | 5,185 / 5,807 |
| CPU         | cold   | on         | 1,124 / 1,270 | 1,978 / 2,040 | 2,556 / 2,596 | 5,650 / 5,681 |
| Radeon 780M | warm   | off        |     349 / 369 |     472 / 481 |     650 / 659 |   857 / 1,214 |
| Radeon 780M | cold   | off        |     386 / 392 |     394 / 546 |     493 / 775 |   888 / 1,066 |
| Radeon 780M | warm   | on         |     533 / 565 |   626 / 1,130 |   835 / 1,834 | 1,808 / 3,467 |
| Radeon 780M | cold   | on         |   783 / 1,136 | 1,184 / 1,466 | 1,151 / 1,765 | 2,579 / 3,827 |
| RTX 4060    | warm   | off        |     407 / 415 |   461 / 1,177 | 1,370 / 1,522 | 1,720 / 1,867 |
| RTX 4060    | cold   | off        |     309 / 325 |     482 / 583 |   804 / 1,377 | 1,791 / 1,835 |
| RTX 4060    | warm   | on         |     619 / 649 |     850 / 980 |   988 / 1,608 | 2,660 / 2,681 |
| RTX 4060    | cold   | on         |     965 / 994 | 1,347 / 1,718 | 1,990 / 2,438 | 2,670 / 3,177 |

Phase medians, warm with AI editing (ms). The tail was 206 to 229 ms in every cell, because release waited for the next 33 ms UI tick after the 200 ms deadline.

| Device      | Clip | Speech | Cleanup |
| ----------- | ---- | -----: | ------: |
| CPU         | word |    103 |     144 |
| CPU         | 5 s  |    267 |     378 |
| CPU         | 10 s |    927 |     691 |
| CPU         | 30 s |  3,498 |   1,439 |
| Radeon 780M | word |    148 |     170 |
| Radeon 780M | 5 s  |    169 |     237 |
| Radeon 780M | 10 s |    256 |     344 |
| Radeon 780M | 30 s |    605 |     955 |
| RTX 4060    | word |    187 |     219 |
| RTX 4060    | 5 s  |    226 |     380 |
| RTX 4060    | 10 s |    569 |     174 |
| RTX 4060    | 30 s |  1,947 |     476 |

A 580 ms one-word crop played as soon as the app reports listening (`--settle-ms 0`) removes the 600 ms preload window and is the better estimate for a quick word:

| Device      | Warm, AI off | Cold, AI off | Warm, AI on | Cold, AI on |
| ----------- | -----------: | -----------: | ----------: | ----------: |
| CPU         |    330 / 346 |    326 / 345 |   480 / 492 |   827 / 887 |
| Radeon 780M |    313 / 316 |    319 / 338 |   407 / 420 |   768 / 842 |
| RTX 4060    |    251 / 275 |    251 / 272 |   394 / 431 |   677 / 735 |

The RTX 4060 is often slower than the iGPU in practice: NVIDIA runtime-suspends while idle or recording. With the same models loaded, a 35 s idle before each request raised speech from 91 / 96 to 967 / 1,014 ms and cleanup from 79 / 83 to 400 / 420 ms. The device was suspended before four of five warm 10 s releases in the matrix.

## Experiments

| Experiment                                           | Result                                                                                                                                                                                                                                                                                                                                                                      | Disposition       |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| Wake on the release deadline                         | Tail 216 → 200 ms median. Same 200 ms of audio.                                                                                                                                                                                                                                                                                                                             | Shipped           |
| Prompt-prefix KV reuse                               | Paired warm workers, varied inputs, alternating order: median saving 22 ms CPU (325 → 310), 21 ms Radeon (159 → 135), 7 ms NVIDIA (87 → 80). GPU outputs byte-identical. CPU changed `off-by-one` to `off by one` in one of 11 cases. CPU first-decode prefill 121 → 94 ms, 67 tokens reused.                                                                               | Shipped           |
| Stop generation once output fails the guard          | Six-repeat 29 s stress clip, cache off: cleanup 3,576 / 3,681 → 2,460 / 2,980 ms. The transcript is still delivered. The same 1.3 length ratio applies, and a growing output prefix cannot pass it later.                                                                                                                                                                   | Shipped           |
| Streaming, Parakeet Unified EN                       | Warm, AI off. Radeon 10 s 378 / 392, 30 s 388 / 400. RTX 4060 352 / 377 and 362 / 379. 24/24 streamed. Streaming keeps the dGPU awake. English only.                                                                                                                                                                                                                        | Recommend, opt-in |
| Preload S1-mini at key press                         | Prototype, cold CPU, AI on: word 1,124 → 472 ms, 5 s 1,978 → 858 ms (600 ms settle). Shipped version, base `b71e119`, cold, n = 10 alternating: CPU word 908 → 472, 5 s 1,229 → 790; Radeon word 797 → 422, 5 s 1,032 → 618 ms. Outputs identical. A slow load now delays speech instead of cleanup.                                                                        | Shipped           |
| VAD-gated release tail                               | When Silero saw trailing silence: 5 s tail 206 → 19 ms, 10 s 223 → 14 ms. In 77 replayed cuts, a 200 ms early cut changed the final word in 10; the gate stayed closed in all 10. Mic buffering, noise and real key release are unvalidated.                                                                                                                                | Not shipped       |
| Skip cleanup when text already looks clean           | Four of five already-punctuated probes still needed edits (`john` → `John`, `He go` → `He goes`).                                                                                                                                                                                                                                                                           | Rejected          |
| Threads 1/2/8, n_batch, flash attention, KV clearing | No stable win over the defaults (four threads, n_batch 1,024, flash off). CPU runs drifted with temperature between identical controls. CPU cleanup with GPU speech was slower.                                                                                                                                                                                             | Unchanged         |
| Prompt-lookup speculative decoding                   | Draft up to 8 tokens copied from the transcript and verify them in one decode. Greedy outputs byte-identical on all 12 cases. About 2.8 tokens per decode, but CPU cost grows with batch size (1 token 13 ms, 9 tokens 121 ms): CPU cleanup about 2x slower. Radeon: 9 tokens 4.7 ms vs 2.2 ms, about 8% faster overall, with first-use shader compile spikes up to 364 ms. | Rejected          |

Repeating the same clip in the real app shows larger cache gains (CPU word 475 → 349 ms, 5 s 864 → 740 ms). That run also reuses the transcript and includes the tail fix, so the paired numbers are the honest estimate.

## Recommendations

1. **Use the iGPU through Vulkan.** The worker already prefers it; do not force `TYPEASIDE_LOCAL_DEVICE=cpu`. Warm 30 s with AI off goes from 3,893 ms on CPU to 857 ms. For English, choosing **Parakeet Unified EN** in Settings streams while recording and keeps release-to-text near 380 ms for any length.
2. **Ship prefix KV reuse, early guard rejection and the release-deadline wake** (this PR). Together they save about 15 ms of tail plus 7 to 22 ms per cleanup on typical dictations, and over a second on runaway output.
3. **Preload S1-mini at key press when AI editing will run.** Shipped: a dictation that may reach S1-mini (editing on, language Auto or English, not a code app) loads it while the key is held, saving about 400 ms on a cold dictation.
4. **Keep the dGPU awake or avoid it for short dictations.** Waking from runtime suspend costs about 900 ms. Prefer the iGPU, or wake the dGPU at key press.
5. **VAD-gated tail**: revisit only with real-microphone release data.

Re-measure streaming on the iGPU:

```bash
python3 scripts/e2e-latency.py run --scratch "$TA_SCRATCH" --binary "$TA_SCRATCH/typeaside-latency" \
  --label igpu-stream --device auto --icd /usr/share/vulkan/icd.d/radeon_icd.json \
  --model parakeet-unified-en "$TA_SCRATCH/clips/10s.wav" "$TA_SCRATCH/clips/30s.wav"
```

## Reproduction

```bash
export CARGO_TARGET_DIR=/absolute/target TA_SCRATCH=/absolute/scratch
export TMPDIR="$TA_SCRATCH"
# Vulkan SDK as in .github/workflows/release.yml
export VULKAN_SDK="$TA_SCRATCH/1.4.309.0/x86_64" CMAKE_PREFIX_PATH="$TA_SCRATCH/1.4.309.0/x86_64"
export PATH="$VULKAN_SDK/bin:$PATH"
python3 scripts/e2e-latency.py prepare --ref 20fa723 --scratch "$TA_SCRATCH"
(cd "$TA_SCRATCH/source" && cargo build --release --locked --bin typeaside --examples -j8)
cp "$CARGO_TARGET_DIR/release/typeaside" "$TA_SCRATCH/typeaside-latency"
export TYPEASIDE_HOME="$TA_SCRATCH/home"
"$CARGO_TARGET_DIR/release/examples/local_models" --download parakeet-v3
"$CARGO_TARGET_DIR/release/examples/local_models" --download parakeet-unified-en
"$CARGO_TARGET_DIR/release/examples/local_cleanup" --download
uv venv "$TA_SCRATCH/venv" && uv pip install --python "$TA_SCRATCH/venv/bin/python" numpy pyarrow soundfile
curl --fail --location -o "$TA_SCRATCH/librispeech.parquet" \
  https://huggingface.co/datasets/hf-internal-testing/librispeech_asr_dummy/resolve/5be91486e11a2d616f4ec5db8d3fd248585ac07a/clean/validation-00000-of-00001.parquet
echo "4e69a06fa5edc90921e5e7e39a7084881f8b3ed9c805c574f4f39c6fde27c603  $TA_SCRATCH/librispeech.parquet" | sha256sum --check
"$TA_SCRATCH/venv/bin/python" scripts/latency-fixtures.py --corpus --stress \
  "$TA_SCRATCH/librispeech.parquet" "$TA_SCRATCH/clips"
python3 scripts/e2e-latency.py run --scratch "$TA_SCRATCH" --binary "$TA_SCRATCH/typeaside-latency" \
  --label cpu-cold-ai --device cpu --icd /usr/share/vulkan/icd.d/radeon_icd.json --cold --editing \
  "$TA_SCRATCH/clips/word.wav" "$TA_SCRATCH/clips/5s.wav" "$TA_SCRATCH/clips/10s.wav" "$TA_SCRATCH/clips/30s.wav"
```

The full matrix repeats `run` for `--device cpu|auto|gpu` (`radeon_icd.json` for CPU and iGPU, `nvidia_icd.json` for the dGPU), with and without `--editing` and `--cold`. Labels must be unique. Use a fresh scratch export: copied CMake caches keep their original absolute path.

Experiments use `prepare --experiments`, which adds scratch-only environment overrides to the export (never to `src/`):

```bash
# Paired prefix-cache comparison; repeat with TYPEASIDE_LOCAL_DEVICE=auto, and gpu + nvidia ICD
TYPEASIDE_LOCAL_DEVICE=cpu VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json \
  python3 scripts/latency-pairs.py "$TA_SCRATCH/candidate/typeaside-latency" \
  --artifacts "$TA_SCRATCH/paired-cpu" --runs 5 > "$TA_SCRATCH/paired-cpu.jsonl"
# Parameter sweep: TA_CACHE=0|1, TA_THREADS, TA_BATCH_THREADS, TA_FLASH=1, TA_BATCH/TA_UBATCH,
# TA_CLEANUP_CPU=1, TA_CLEAR_DATA=0, one at a time
TA_CACHE=0 TA_THREADS=4 python3 scripts/latency-worker.py \
  "$TA_SCRATCH/candidate/typeaside-latency" "$TA_SCRATCH/clips/5s.wav" --cases --probes
# Guard: TA_LENGTH_GUARD=0 versus 1
TA_CACHE=0 TA_LENGTH_GUARD=0 python3 scripts/latency-worker.py \
  "$TA_SCRATCH/candidate/typeaside-latency" "$TA_SCRATCH/clips/repeated.wav" --gap 1
# dGPU idle wake: add --idle 35
TYPEASIDE_LOCAL_DEVICE=gpu VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json TA_CACHE=0 \
  python3 scripts/latency-worker.py "$TA_SCRATCH/candidate/typeaside-latency" "$TA_SCRATCH/clips/5s.wav"
# App prototypes: TA_PRELOAD_CLEANUP=1 with --cold --editing, TA_VAD_TAIL=1 without editing
TA_PRELOAD_CLEANUP=1 TA_CACHE=0 python3 scripts/e2e-latency.py run --scratch "$TA_SCRATCH" \
  --binary "$TA_SCRATCH/candidate/typeaside-latency" --label preload-cpu --device cpu \
  --icd /usr/share/vulkan/icd.d/radeon_icd.json --editing --cold \
  "$TA_SCRATCH/clips/word.wav" "$TA_SCRATCH/clips/5s.wav"
# Release-cut risk replay
TYPEASIDE_LOCAL_DEVICE=auto "$CARGO_TARGET_DIR/release/examples/local_models" \
  --bench "$TA_SCRATCH/clips/release-risk.json" "$TA_SCRATCH/release-risk.jsonl"
```

Raw samples, traces, export patches and binary hashes for this run are kept outside the repo in `/home/aditya/Projects/.ta-targets/e2e-scratch` (`benchmarks/` holds the summarized JSONL).

## Limits

One host in one power profile, n = 5 per cell, one dictation at a time, English with the general context only. No clipboard or key-injection timing. Inference on macOS and Windows is unverified.
