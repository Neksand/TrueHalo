## TrueHalo v0.4.0

**Release date:** 2026-11-02 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64 · Linux x64

### Added

- **Token budget** — a live per-request token estimate under the message box and automatic oldest-first trimming of history beyond the budget (1k–32k, default 8k). System instructions and your newest message always survive.

### Changed

- **Clear positioning** — the README now states plainly that TrueHalo is for documents, code, and translations, must not be used for cheating, and no longer references cheating tools. The comparison table now covers Electron overlays instead.

### Install

- macOS (arm64): `TrueHalo-0.4.0-Installer.dmg`
- Windows (x64): `TrueHalo_0.4.0_x64-setup.exe`
- Linux (x64): `TrueHalo_0.4.0_amd64.deb` or `TrueHalo_0.4.0_amd64.AppImage`

### Verification

- `npm run build`, `npx tsc --noEmit`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
