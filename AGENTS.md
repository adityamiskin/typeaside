# AGENTS Notes

Durable repo-specific facts that are easy to miss.

## Linux Credential Storage

- The app explicitly uses `secret-service` on Linux in `src/credentials.rs`.
- A local vendored copy of `keyring` lives under `vendor/keyring`.
- That vendored patch intentionally disables `keyring`'s SQLite/db-keystore backend because it drags in `turso -> aegis`, which broke Linux desktop builds through Clang/AVX512 compilation.
- Do not remove the `[patch.crates-io] keyring = { path = ... }` entry from `Cargo.toml` unless the upstream dependency chain is verified to build cleanly again on Linux.
