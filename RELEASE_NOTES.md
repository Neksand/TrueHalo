## TrueHalo v0.2.2

**Release date:** 2026-10-29 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64

### Fixed

- **Settings rendering** — the Settings modal now renders. Two React hooks ran after an early return (React error #310) and crashed the whole interface.
- **Transparency** — opening Settings, onboarding or the permission dialog no longer makes the window fully transparent on macOS. The full-screen scrim uses a solid tint; `backdrop-filter` broke compositing of the transparent window.
- **Keychain freeze** — Settings no longer blanks while macOS asks for the Keychain password. Keychain commands run off the main thread and the model catalog is cached, so startup never reads the API key.
- **Visible errors** — a UI render error now shows a fallback screen and writes the details to `~/Library/Logs/TrueHalo/truehalo.log` instead of leaving an empty window.
- **Tray Settings** — opens and focuses the overlay; the global hotkey still shows it without stealing focus.

### Install

- macOS (arm64): `TrueHalo-0.2.2-Installer.dmg`
- Windows (x64): `TrueHalo_0.2.2_x64-setup.exe`

### Verification

- `npm run build`, `npx tsc --noEmit`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
