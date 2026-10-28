# Cache-aware Nemotron streaming experiment

**Verdict:** Nemotron EN is a useful opt-in for faster release on CPU and Radeon. Keep Parakeet v3 as the English default; both Nemotron candidates lose corpus accuracy and punctuation quality. Keep 3.5 experimental pending multilingual validation. Use 1040 ms lookahead.

Nemotron English ships as an opt-in model from this benchmark. Nemotron 3.5 was measured but is not in the catalog. The raw artifacts and the full transcript diff stayed on the benchmark machine.

## Implementation

`nemotron-en` and `nemotron-3-5` use Q4_K_M GGUFs through transcribe.cpp 0.4.1. Its Rust API already exposes `StreamExtension::ParakeetStream(ParakeetStreamOptions { att_context_right })`; no vendoring or FFI patch is necessary. The loader probes the public `PKST` extension kind, then allows cache-aware streams on CPU as well as GPU. Buffered Unified English remains GPU-only. Each dictation retains encoder/decoder state across 160 ms feeds and gets a fresh session afterward. Whole-clip VAD fallback and the 1.5 s cooperative finalize cancellation remain in place.

The catalog pins model, LICENSE and NOTICE downloads to HF commits and verifies their sizes and SHA-256. The selected language passes through per-request `RunOptions`. Exact advertised languages take priority, then a short code resolves to the first advertised matching locale: `en` becomes `en-US` for 3.5. Its prompt dictionary supports the short alias, but its advertised `general.languages` list contains full locales only. The old exact-only filter silently selected Auto; those 3.5 measurements are excluded from the English results. Parakeet v3 remains the configuration default.

| Asset | Q4_K_M bytes | Q8_0 bytes, not benchmarked | HF commit |
| --- | ---: | ---: | --- |
| Nemotron EN | 475,436,032 | 729,650,176 | `7df420cc3fb915a3b334560dac712af44fc41ef5` |
| Nemotron 3.5 | 495,831,520 | 751,094,240 | `53ce288f69326b415aeef030cfcb02a08a8096dd` |

The exact Q4 hashes live in `src/local/assets.rs`. Q8 hashes from the same HF API snapshots: EN `90d8c89714cd31efc88be62a40c6b2bea57e0cc2063af1ffe2c28f1a228ca110`; 3.5 `b94545b313b3223fda7b2857a52681da813935c2127643d1e9ff0c23d988089c`.

## Measurement contract

Linux x86_64, Ryzen 9 8945HS, 16 logical CPUs, 14 GiB RAM; eight CPU inference threads, one decoder thread on GPU. All builds use at most `-j6`, the specified Vulkan SDK, and a reflink copy of `spec/target` under `/home/aditya/Projects/.ta-targets/nemotron/target`. The app home, runtime/socket, models, logs and exported source are isolated under the same `nemotron` directory. Existing models and LibriSpeech clips were reused; only the two new model packages were downloaded.

The real-app measurements use `scripts/e2e-latency.py` with an uncommitted Nemotron adapter around its exported harness. A private Pulse sink feeds the GPUI recorder; the CLI drives the isolated instance socket; instrumented delivery writes JSON rather than pasting. AI editing is off, language is `en`, capture settle is 600 ms, release tail remains 200 ms. Each clip has one excluded warmup and five scored warm dictations. Median is the middle sample; p90 is nearest rank, hence the maximum with n=5. Clipboard/key injection is excluded. Models load before a ten-second idle probe. Compilation finishes before scored runs.

The shortened-quiet control is the unchanged 30 s speech cropped to 28.757875 s, ending 50 ms after its last sample above amplitude 0.01, exactly as in the segmentation experiment. The regular clips have about 592 / 1,101 / 1,292 ms of trailing quiet. Capture settling, playback drain and release tail add further audio, so these are not measurements of releasing at the final phoneme.

CPU-seconds use `/proc/<pid>/stat` user+system deltas for the app and descendants, as in the segmentation experiment. Recording CPU includes capture settle and playback drain. Worker PSS is sampled from `smaps_rollup` every 500 ms and after delivery; sampled peaks can miss shorter transients and exclude GPU allocations outside process mappings. Held CPU percentage is aggregate recording CPU-seconds / recording wall-seconds, so 100% means one logical core. Feed wall RTF is the sum of synchronous `feed` durations / fed audio duration, excluding waits between messages; it is distinct from CPU-seconds/audio. Finalize duration is logged separately.

Accuracy replay uses the same 77-entry release-risk manifest (73 utterances and four derived fixtures), the same case/punctuation-insensitive word Levenshtein scorer, and 10x accelerated feed timing. Replay latency is excluded from the real-app table. Every replay first runs offline, abandons a one-second stream, then streams the clip. Both offline and streamed transcripts are retained. Micro-WER is total edits / total reference words, not the mean of per-clip WERs. Punctuation and casing must be assessed separately.

Only the scratch export receives lookahead overrides and feed/finalize instrumentation through `--experiments`. No experiment environment switches are added to `src/` in the worktree. Delivery instrumentation leaves unused-variable warnings in the exported source; strict clippy runs on the actual worktree.

## Licenses

The two models do not share a license. [NVIDIA Open Model License](https://www.nvidia.com/en-us/agreements/enterprise-software/nvidia-open-model-license/) covers EN. Sections 2.2 and 3 permit distribution subject to its terms, including the agreement and NVIDIA attribution notice, plus its use conditions. The catalog downloads both LICENSE and NOTICE with the weights. This permits keeping application code under MIT while distributing model weights under their own license; the weights do not become MIT licensed.

The pinned [3.5 model card](https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf) and downloaded LICENSE specify OpenMDW 1.1, which permits distribution while retaining the agreement and applicable notices. It is not NVIDIA Open Model License. Source API snapshots and exact license/notice files are retained in the scratch artifacts. These are checks against the published grants and download packaging, not a claim that all downstream uses satisfy every license condition.

## Real-app latency and per-clip accuracy

All cells have n=5 scored warm runs. Release-to-delivery ms, median / p90; 1040 ms lookahead for both Nemotron models.

| Model / device | 5 s | 10 s | 30 s | 30 s shortened quiet |
| --- | ---: | ---: | ---: | ---: |
| Parakeet v3, CPU offline | 385 / 392 | 536 / 548 | 1,231 / 1,262 | 1,203 / 1,217 |
| Nemotron EN, CPU stream | 263 / 264 | 279 / 282 | 282 / 295 | 281 / 296 |
| Nemotron 3.5 en, CPU stream | 261 / 262 | 289 / 292 | 312 / 325 | 269 / 291 |
| Nemotron EN, Radeon stream | 288 / 367 | 308 / 320 | 314 / 344 | 310 / 459 |

Word error rate, mean across five scored transcripts (min–max), in percent. The reference word counts are 15 / 28 / 72 / 72.

| Model / device | 5 s | 10 s | 30 s | 30 s shortened quiet |
| --- | ---: | ---: | ---: | ---: |
| Parakeet v3, CPU offline | 0.00 (0.00–0.00) | 3.57 (3.57–3.57) | 5.28 (4.17–5.56) | 5.00 (4.17–5.56) |
| Nemotron EN, CPU stream | 0.00 (0.00–0.00) | 0.00 (0.00–0.00) | 6.11 (4.17–6.94) | 6.39 (4.17–8.33) |
| Nemotron 3.5 en, CPU stream | 0.00 (0.00–0.00) | 6.43 (3.57–7.14) | 6.94 (6.94–6.94) | 6.94 (6.94–6.94) |
| Nemotron EN, Radeon stream | 1.33 (0.00–6.67) | 0.00 (0.00–0.00) | 5.00 (4.17–5.56) | 5.28 (4.17–6.94) |

## CPU and memory

Total app-family CPU-seconds per clip, median / p90. This includes capture and release; GPU execution time is not CPU time.

| Model / device | 5 s | 10 s | 30 s | 30 s shortened quiet |
| --- | ---: | ---: | ---: | ---: |
| Parakeet v3, CPU offline | 1.81 / 1.85 | 3.25 / 3.36 | 10.13 / 10.22 | 10.00 / 10.03 |
| Nemotron EN, CPU stream | 3.44 / 3.64 | 5.85 / 5.96 | 15.83 / 16.12 | 15.38 / 15.52 |
| Nemotron 3.5 en, CPU stream | 4.19 / 4.24 | 7.45 / 7.89 | 24.70 / 25.04 | 15.86 / 16.53 |
| Nemotron EN, Radeon stream | 0.87 / 0.94 | 1.52 / 1.57 | 4.05 / 4.14 | 1.92 / 2.05 |

30 s resource detail. PSS values are MiB. CPU percentages refer to one logical core, not all 16 CPUs.

| Model / device | Recording CPU s, median | Release CPU s, median | Held CPU %, median | Feed wall RTF, median / p90 | Worker PSS loaded / sampled peak / after, median |
| --- | ---: | ---: | ---: | ---: | ---: |
| Parakeet v3, CPU offline | 2.50 | 7.59 | 8.1 | declined | 875 / 968 / 901 |
| Nemotron EN, CPU stream | 15.23 | 0.59 | 49.6 | 0.059 / 0.061 | 817 / 885 / 885 |
| Nemotron 3.5 en, CPU stream | 23.86 | 0.81 | 77.7 | 0.092 / 0.094 | 935 / 953 / 953 |
| Nemotron EN, Radeon stream | 3.95 | 0.10 | 12.9 | 0.093 / 0.095 | 111 / 116 / 108 |

Every model recorded **0.00 worker CPU-seconds over ten seconds loaded idle**, at 10 ms kernel tick resolution. The app-family idle deltas were 0.02–0.06 CPU-seconds over ten seconds, mostly UI/housekeeping. Both cache-aware CPU models keep up with real-time capture on this machine. For 30 s, EN moves 15.23 CPU-seconds before release; 3.5 moves 23.86 CPU-seconds before release. Total CPU rises about 56% / 144% over the offline baseline. CPU-seconds are not a battery-energy measurement.

Vulkan PSS excludes substantial driver-managed allocations. A separate live worker DRM snapshot reported 472,524 KiB resident GTT plus 23,260 KiB resident VRAM (about 484 MiB combined), in addition to process PSS. It is one snapshot, not a peak or a complete GPU-memory accounting. The low Vulkan PSS must not be interpreted as the model using only 116 MiB of machine memory.

Runs are blocked by model/device in the table order, not interleaved. Thermal drift and other desktop activity remain possible confounds. No compiler ran alongside scored latency samples. With n=5, p90 is only the largest observation.

## Lookahead choice

**Use 13 frames, 1040 ms lookahead**, for both models. This corresponds to 1120 ms encoder chunks; six frames corresponds to 560 ms chunks. More frequent smaller encoder calls increased measured CPU without a reliable release-time benefit. The app finalizes by flushing cached state; it does not wait a full lookahead interval after release.

The shortened-quiet control has one excluded warmup and **n=1 scored** run per setting. These are tuning probes, not five-sample latency estimates:

| CPU model | Lookahead | Release-to-text ms | Total CPU s | WER |
| --- | ---: | ---: | ---: | ---: |
| EN | 1040 ms | 260 | 15.29 | 6.94% |
| EN | 480 ms | 284 | 19.95 | 4.17% |
| 3.5, explicit English | 1040 ms | 288 | 18.79 | 6.94% |
| 3.5, explicit English | 480 ms | 294 | 23.86 | 6.94% |

The CPU increases are 30.5% / 27.0%. EN's 480 ms corpus replay improves only one word edit; 3.5's worsens eleven. EN's 1040 ms control also appended a trailing “and”; its absence at 480 ms is not a guarantee, since capture alignment varies and the five-sample main controls also expose occasional additions. The 480 ms 3.5 control loses sentence punctuation. Keep the selected 1040 ms setting rather than spending more CPU for these mixed quality results.

## Accuracy over all 77 replays

All rows use the same 1,266 reference words. One replay per manifest entry, Q4_K_M, CPU. “Offline” includes the existing VAD/chunking path. Parakeet's stream request deliberately declines and falls back offline. Each Nemotron streamed row uses streamed text for **77/77 entries**, without offline fallback.

| Model / path | Lookahead | Word edits | Micro-WER | Macro-WER |
| --- | ---: | ---: | ---: | ---: |
| Parakeet v3 offline | n/a | 45 | 3.55% | 4.17% |
| Nemotron EN stream | 1040 ms | 62 | 4.90% | 5.46% |
| Nemotron EN stream | 480 ms | 61 | 4.82% | 5.26% |
| Nemotron EN offline | model default | 64 | 5.06% | 5.63% |
| Nemotron 3.5 English stream | 1040 ms | 66 | 5.21% | 5.39% |
| Nemotron 3.5 English stream | 480 ms | 77 | 6.08% | 5.76% |
| Nemotron 3.5 English offline | model default | 71 | 5.61% | 6.10% |

The baseline exactly reproduces the segmentation report's 45 edits / 1,266 words, 3.55% micro-WER. At the chosen lookahead, EN improves 7 entries, worsens 17 and ties 53 against Parakeet; 3.5 improves 4, worsens 18 and ties 55. Among comparisons with identical normalized words, 33 EN / 28 3.5 outputs differ in punctuation or casing. Entries are not all independent; do not interpret these counts as a broad population accuracy estimate.

Examples that matter for dictation:

- On 10 s, Parakeet retains the sentence break at `you. I begged` but writes `Rugado`. EN gets `Ruggedo` right while omitting that sentence boundary and the final period in all five CPU runs. 3.5 changes `fled in disgrace` to `fled a disgrace` and usually misses Ruggedo too.
- On 30 s, Nemotron gets `metal forest` where Parakeet often says `meadow forest`. Both families miss the proper-name spellings `nomes` / `Kaliko`. `there is` becoming `there's` costs two normalized word edits despite preserving meaning; the WER gap is not solely acoustic omissions.
- One of five shortened-quiet EN CPU samples adds a final `and`. This is a real unwanted insertion, regardless of whether it is near a cache boundary. Model punctuation/casing also varies with capture alignment.

There is no segmentation/join/deduplication path. Neither cache-aware model emitted an adjacent duplicated word in either complete 77-entry replay, including `018` and the derived `5s` case that produced `but But` in the VAD experiment. That removes the observed join bug from this corpus; it does **not** establish error-free transcription or preservation at every possible chunk alignment. Both models have more corpus errors than Parakeet v3, and their own offline paths also score worse than Parakeet. Streaming itself is not the main source of the quality gap here.

## Edge checks and limits

- **Tap and cancellation:** the worker harness abandons 100 ms audio and separately queues cancellation 10 ms after a two-second feed, then immediately dictates the one-word fixture. Eight assertions pass through EN → 3.5 English → Parakeet v3 → EN without unloading between model selections. The next transcript normalizes to `true`; only Parakeet uses offline fallback. Recovery includes the new word's inference, so its duration is not a pure cancellation latency.
- **Real app cancellation:** for both models, a 40 ms tap and cancellation two seconds into the 30 s playback produce no delivery during the 300 ms observation, followed by successful warm and scored one-word dictations. CLI socket-send times are about 5–7 ms, not an acknowledgement that inference stopped instantly. Actual cancellation is observed between worker feeds, not by interrupting every in-flight encoder operation.
- **Very short speech:** the source word WAV is 850 ms. The scored real-app result is `True.` for both models, around 281 ms EN / 291 ms 3.5 after release; n=1 plus one warmup each. The worker also retains the word without capture settling or the app release tail.
- **Model switch:** the worker harness switches model identities through the production worker-selection path, including returning to EN, and preserves the next word. Interactive settings changes during an active capture were not exercised.
- **Deadline:** every measured real-app cache finalize succeeds; the maximum is 233.444 ms, below 1,500 ms. No finalize timeout/fallback occurs. The existing deadline is cooperative and starts once the worker consumes END; it does not bound time waiting behind queued feeds, and a native call may observe cancellation later. No forced-timeout fault injection was performed. These measurements do not prove a hard wall-clock limit under arbitrary load or on a slower CPU.
- **Idle:** both Nemotron CPU workers also consume 0.00 CPU-seconds in ten seconds after the cancellation/word sequence. This checks post-inference idle, as well as the initial loaded-idle probes.
- **GPU:** EN has the full five-sample Radeon matrix. 3.5 English additionally streams the 5 s clip on Radeon in a feature probe (n=1 plus warmup), 324 ms scored release, zero WER, effective `en-US`, `stream=text`. It is not a five-sample performance estimate.

The corrected 3.5 CPU run has a substantial regular-30 s versus shortened-quiet CPU gap (24.70 versus 15.86 CPU-seconds). It is retained rather than discarded; the cause was not isolated. Blocked run order, capture alignment and desktop contention prevent attributing all resource differences to model architecture. No compiler overlapped scored runs. Real microphones, noise, multilingual accuracy, language switching, slower CPUs, discrete GPU execution for Nemotron, macOS/Metal and Windows remain unverified.

**Shipping recommendation:** offer Nemotron EN as an explicit latency-oriented option after the usual platform checks; retain Parakeet v3 as default, including for English. Keep 3.5 experimental until its multilingual advantage is measured. The prototype proves useful CPU cache-aware streaming and eliminates the observed segmented-join duplication in this corpus, but neither candidate earns the English default on accuracy or punctuation, and release latency is not a universal hard guarantee. Do not rely on AI editing to repair speech-model mistakes; it was off throughout this benchmark.
