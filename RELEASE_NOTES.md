## TrueHalo v0.3.0

**Release date:** 2026-10-30 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64

### Added

- **Assistant profiles** — named answering context (instructions plus .txt/.md files), picked in the chat header and sent with every request. Capped at ~200 KB per profile with a token estimate and a per-profile Unlimited switch. Files stay on your device and only leave it inside your own requests to the selected provider.
- **Local models** — a Local server provider for Ollama (`http://localhost:11434/v1` out of the box), LM Studio, llama.cpp or any OpenAI-compatible endpoint, configurable in Settings. No API key needed.
- **Click-through mode** — mouse clicks pass through the overlay; toggle back with the hotkeys or the tray menu.
- **History controls** — keep screenshots in history on/off (text-only history keeps the database small) and ephemeral mode (delete all chats on quit).
- **Stealth** — the overlay stays out of Mission Control and the window cycle on macOS, and out of the taskbar, Alt+Tab and Task View on Windows.
- **Version in Settings** — the app version is shown in the Settings footer.

### Install

- macOS (arm64): `TrueHalo-0.3.0-Installer.dmg`
- Windows (x64): `TrueHalo_0.3.0_x64-setup.exe`

### Verification

- `npm run build`, `npx tsc --noEmit`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
