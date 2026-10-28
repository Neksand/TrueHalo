## TrueHalo v0.2.1

**Release date:** 2026-10-28 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64

### Fixed

- **Installer** — the macOS app bundle is re-signed after packaging and the DMG now contains the Applications shortcut, so the app installs and launches.
- **External links** — OpenAI, OpenRouter and Gemini links open the system browser from the packaged app.
- **OpenAI key page** — `platform.openai.com/api-keys` (the old `/account/api-keys` path dead-ends). OpenAI has no OAuth for third-party apps: sign in there, create a key, paste it in Settings.
- **OpenRouter key page** — `openrouter.ai/settings/keys`.
- **Vision guard** — screenshots are no longer sent to text-only models; the app explains the problem before the request leaves.
- **Hide from Dock** — the overlay stays visible after a restart instead of disappearing with no Dock entry.
- **Focus** — showing the overlay no longer pulls keyboard focus from the app underneath; clicking it focuses normally.

### Changed

- One shared provider catalog for Settings and onboarding; one pricing module.
- Removed the placeholder model-compare panel until parallel streaming ships.

### Install

- macOS (arm64): `TrueHalo-0.2.1-Installer.dmg`
- Windows (x64): `TrueHalo_0.2.1_x64-setup.exe`

### Verification

- `npm run build`, `npx tsc --noEmit`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
