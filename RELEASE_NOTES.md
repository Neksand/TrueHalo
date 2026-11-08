## TrueHalo v0.4.2

**Release date:** 2026-11-08 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64 · Linux x64

### Fixed

- **Long answers keep streaming** — cloud requests now use a 120-second inactivity timeout instead of a hard 2-minute total cap, so a long generation is no longer cut off mid-answer. Local servers use 300 seconds.
- **Gemini key handling** — the API key is sent in the `x-goog-api-key` header instead of the URL, so it cannot leak through request errors or logs.
- **Chat switching during generation** — selecting another conversation cancels the active stream, and a slow history load can no longer overwrite a newer selection.
- **No empty assistant turns** — cancelling or hitting a provider error before the first token removes the placeholder instead of saving an empty message. Partial answers are kept.
- **AUR `truehalo-bin`** — the package now extracts `data.tar.*` from the downloaded `.deb`; it previously assumed the payload was already unpacked.
- **SQLite resilience** — history calls recover from a poisoned connection mutex instead of failing for the rest of the session.
- **Linux credential fallback** — the file store creates keys with `0600` permissions from the first byte instead of narrowing permissions after writing.
- **Update retry** — a failed update check is now a real button, not a label that could not be clicked.
- **Local DMG name** — `scripts/build.sh` uses the current package version instead of a hardcoded 0.4.0.

### Changed

- CI cancels superseded builds on the same branch or pull request.

### Install

- macOS (arm64): `TrueHalo-0.4.2-Installer.dmg`
- Windows (x64): `TrueHalo_0.4.2_x64-setup.exe`
- Linux (x64): `TrueHalo_0.4.2_amd64.deb` or `TrueHalo_0.4.2_amd64.AppImage`

### Verification

- `npm run build`, `npm run lint`, `npm test`
- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
- macOS app signature verified with `codesign --verify --deep --strict`
