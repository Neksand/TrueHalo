## TrueHalo v0.2.2

**Release date:** 2026-10-29 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64

### Fixed

- **Settings window** — opening Settings, onboarding or the permission dialog no longer makes the window fully transparent on macOS. The full-screen scrim now uses a solid tint instead of `backdrop-filter`, which broke compositing of the transparent window and left only the native traffic lights visible.

### Install

- macOS (arm64): `TrueHalo-0.2.2-Installer.dmg`
- Windows (x64): `TrueHalo_0.2.2_x64-setup.exe`

### Verification

- `npm run build`, `npx tsc --noEmit`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
