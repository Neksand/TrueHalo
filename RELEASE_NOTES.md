## TrueHalo v0.3.1

**Release date:** 2026-10-31 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64

### Fixed

- **Ephemeral history that means it** — chats are deleted the moment the app quits, not on the next start.
- **Send guards** — requests are blocked when no model is selected, and over-limit profiles stay blocked until Unlimited is on or their files are trimmed.

### Install

- macOS (arm64): `TrueHalo-0.3.1-Installer.dmg`
- Windows (x64): `TrueHalo_0.3.1_x64-setup.exe`

### Verification

- `npm run build`, `npx tsc --noEmit`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
