# Release signing

Install `minisign` before packaging. Linux and macOS packaging scripts sign their
installers automatically. After building a Windows installer locally, run
`packaging/sign.sh dist/TypeAside-setup.exe` from Bash.

The default secret key is `~/.config/typeaside-release/minisign.key` (mode `600`).
`TYPEASIDE_SIGNING_KEY_FILE` selects another key file. CI reads the unencrypted
key from the `TYPEASIDE_SIGNING_KEY` repository secret. Only `minisign.pub` belongs
in the repository; back up the secret key separately.

`packaging/sign.sh artifact...` writes and verifies `.minisig` sidecars using the
embedded public key. `packaging/upload.sh` requires valid signatures for every
installer before uploading anything, and uploads sidecars alongside installers.
The app refuses updates without a valid signature, including older unsigned releases.
