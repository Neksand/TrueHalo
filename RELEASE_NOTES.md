## TrueHalo v0.2.3

**Release date:** 2026-10-30 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64

### Added

- **Per-capture Image / Text choice** — every screenshot now keeps both the compressed image and the on-device recognized text. An Image / Text toggle above the preview picks what the next message sends. The default follows Settings → Screenshot input (Auto / Text first / Image first).

### Install

- macOS (arm64): `TrueHalo-0.2.3-Installer.dmg`
- Windows (x64): `TrueHalo_0.2.3_x64-setup.exe`

### Verification

- `npm run build`, `npx tsc --noEmit`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
