# Dictation history limits

History keeps at most 2,000 entries and 4 MiB of serialized JSONL, evicting the oldest timestamps first. Each entry stores at most 16 KiB of transcript text across raw and cleaned text, truncated at UTF-8 boundaries. Delivery still uses the full transcript. Failed audio keeps the newest 20 recordings within 200 MiB; evicted files and interrupted temporary files are removed.

The default retention is unchanged, labeled **Until full**. **Off** stops saving new transcripts and preserves existing history within the hard caps. **Clear all** requires confirmation. Individual transcript deletion and timed retention remain available.

History file reads and mutations share a process mutex. Deleting a transcript reads the current file under that lock, preserving transcripts concurrently added by the engine. Writes atomically replace the bounded file. Startup skips corrupt JSONL lines and applies the same caps as writes.
