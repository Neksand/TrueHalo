# Changelog

All notable changes to TrueHalo are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.2] — 2026-11-08

### Changed
- Streaming providers now use an inactivity timeout (120 s for cloud providers, 300 s for local servers) instead of a hard total timeout, so long answers are not cut off mid-stream.
- A failed update check is now a clickable retry instead of a label that could not be clicked.
- CI cancels superseded runs on the same ref.

### Fixed
- Gemini API keys are sent in the `x-goog-api-key` header instead of the URL, so they cannot leak through request errors or logs.
- Switching conversations during generation cancels the active stream; stale history loads can no longer overwrite a newer selection.
- Cancelled or failed generations no longer leave empty assistant bubbles; partial answers are saved.
- AUR `truehalo-bin` extracts `data.tar.*` from the downloaded `.deb` instead of assuming it is already unpacked.
- SQLite history recovers from a poisoned mutex instead of failing every later call.
- Linux fallback credential files are created with `0600` permissions from the first byte.
- `scripts/build.sh` names DMGs with the current version instead of a hardcoded 0.4.0.

## [0.4.1] — 2026-11-07

### Added
- Check for updates in the Settings footer: compares the running version to the latest GitHub release and links to the download when a newer build exists. Manual install, no auto-download.
- Frontend lint (ESLint) and a Vitest suite covering the updater and token budgeting; CI now gates formatting, lint and frontend tests on macOS, Windows and Linux.

### Changed
- CI uses Node 24 LTS (Node 20 is end-of-life). Dependabot watches npm, Cargo and GitHub Actions weekly.
- Frontend toolchain upgraded to Vite 8, Vitest 5 and ESLint 10; `npm audit` reports no vulnerabilities.
- Update checks time out after 8 seconds and report failure instead of silently doing nothing.
- Removed the unused release-notes payload from the update check.

### Fixed
- AUR metadata pointed at the wrong release tag, so `truehalo-bin` could not build. It now points at 0.4.1 with the matching checksum.
- `cargo fmt --check` now passes (formatting drift in `ai/local.rs` and `auth/linux_store.rs`).
- README roadmap no longer lists already-shipped features as upcoming; ARCHITECTURE.md documents the Linux backend.

## [0.4.0] — 2026-11-02

### Added
- Token budget: a live per-request estimate under the message box and automatic oldest-first trimming of history beyond the budget (1k–32k, default 8k). System instructions and the newest message always survive.

### Changed
- README positioning: TrueHalo is for documents, code, and translations; the cheating comparison is gone, replaced by an Electron-overlays comparison and an explicit no-cheating notice.

## [0.3.1] — 2026-10-31

### Added
- Linux (x64): `.deb` and `.AppImage` packages, Desktop Portal capture (X11 and Wayland), Secret Service credential storage with a locked-down file fallback.

### Fixed
- Ephemeral history now wipes chats the moment the app quits, not on the next start.
- Sending is blocked when no model is selected, and over-limit profiles stay blocked until Unlimited is on or their files are trimmed.

## [0.3.0] — 2026-10-30

### Added
- Assistant profiles: named answering context (instructions plus .txt/.md files) selected in the chat header and sent with every request. ~200 KB cap per profile with a token estimate and a per-profile Unlimited switch; files stay on device except inside your own requests.
- Local models: a Local server provider for Ollama, LM Studio or any OpenAI-compatible endpoint. No API key needed; endpoint configurable in Settings.
- Click-through mode: mouse clicks pass through the overlay; toggle back with the hotkeys or the tray menu.
- History controls: keep screenshots in history on/off (text-only history) and ephemeral mode (delete all chats on quit).
- Stealth: the overlay stays out of Mission Control and the window cycle on macOS, and out of the taskbar, Alt+Tab and Task View on Windows.
- The app version is shown in the Settings footer.

## [0.2.3] — 2026-10-30

### Added
- Per-capture Image / Text choice. Every screenshot keeps both the compressed image and the on-device recognized text; the toggle picks what the next message sends. Settings → Screenshot input is now the default choice (Auto / Text first / Image first).

## [0.2.2] — 2026-10-29

### Fixed
- Settings rendered no content because two React hooks ran after an early return (React error #310). All hooks now run unconditionally, so the Settings modal renders again.
- Settings, onboarding and permission dialogs no longer blank the window on macOS. The full-screen scrim uses a solid tint instead of `backdrop-filter`, which broke compositing of the transparent window and left only the native traffic lights visible.
- Settings no longer freezes or blanks while macOS asks for the Keychain password. Keychain commands run off the main thread, the model catalog is cached in preferences, and catalogs are fetched only from Settings — startup never reads the API key.
- A UI render error now shows a visible fallback and writes the details to the native log instead of leaving an empty window.
- Opening Settings from the tray now activates the window; the hotkey still shows the overlay without stealing focus.

## [0.2.1] — 2026-10-28

### Fixed
- macOS: the installed build now launches — the app bundle is re-signed after packaging, and the DMG includes the Applications shortcut.
- External links (OpenAI, OpenRouter, Gemini) open the system browser from the packaged app.
- OpenAI key page points to `platform.openai.com/api-keys`; OpenRouter to `openrouter.ai/settings/keys`.
- Sending a screenshot to a text-only model is blocked with a clear message instead of a provider 404.
- "Hide from Dock" no longer leaves the overlay invisible after a restart.
- Showing the overlay no longer steals keyboard focus from the app underneath.

### Changed
- Settings and the setup wizard share one provider catalog; pricing helpers live in one module.
- Removed the placeholder model-compare panel until parallel streaming ships.

## [Unreleased]

### Fixed
- Windows: recording protection no longer leaves a black rectangle where the overlay was. While
  protection is on the window becomes fully opaque and drops the DWM transparency composition,
  acrylic blur and the window shadow — all of them are composited outside the excluded surface and
  were painted as a hole in the capture. Transparency and glass are restored when it is off.

### Changed
- Windows: hotkey hints now show `Alt+Space`, `Alt+Shift+2` and `Alt+Shift+3` instead of the
  macOS glyphs (`⌥`, `⇧`).
- Windows: the "hide from the Dock / taskbar" switch reverts when the platform cannot honour it,
  so the UI never claims a change that did not happen.
- README rewritten in plain language and now embeds the 37-second launch film.

### Planned
- Assistant profiles with file-based context injection
- Smart token optimization (same answer quality, fewer tokens)
- Major CPU and memory reduction
- Optional click-through mode

## [0.1.1] — 2026-10-25

### Changed
- Light optimization and clean-up: `clippy`, `tsc`, `eslint`, cargo tests — all green.
- Version bumped to 0.1.1 in `package.json`, `Cargo.toml`, `tauri.conf.json`.
- All fixes from the 0.1.0 follow-up releases (DWM transparency, hotkey labels, accessory/vibrancy revert) rolled into the new tag.

### Added
- README now includes the 37-second launch film (animated preview + MP4 link).
- Settings explain the Windows opacity/glass trade-off when screen-recording protection is enabled.

## [0.1.0] — 2026-10-20

First public release.

### Added
- Native overlay built with Tauri 2, Rust and React on macOS and Windows.
- Three AI providers: OpenRouter, Google Gemini and OpenAI, with a live model catalog.
- Streaming responses with cancellation through `Esc` or the Stop button.
- Multimodal requests: region (`⌥⇧2`) and full-screen (`⌥⇧3`) screenshots.
- Screenshot compression presets (Quality / Balanced / Speed) with token-saving history trimming.
- System credential storage for API keys (macOS Keychain, Windows Credential Manager), with an
  attributes-only existence check and a one-click ACL repair action.
- True transparency with an opacity slider and optional AppKit vibrancy.
- `NSWindowSharingNone` protection: the overlay is excluded from other apps' recordings.
- Accessory mode (hidden from the Dock and `⌘Tab`) with a menu bar icon.
- Window dragging, persisted position/size and global show/hide hotkey (`⌥Space`).
- Local SQLite history and preferences.
- Stable local code signing, so the Screen Recording permission survives rebuilds.
- Windows backend: Credential Manager storage, PowerShell/WinForms screen capture, DWM acrylic,
  `WDA_EXCLUDEFROMCAPTURE` recording protection and an NSIS installer.

### Changed
- Platform-neutral UI copy: the interface no longer hardcodes macOS-specific wording,
  so the same strings fit the upcoming Windows build.

### Fixed
- Windows: screen-recording protection is applied in Rust before the first paint, and is skipped on
  Windows builds older than 2004 where the API degrades to a black rectangle. The settings switch now
  reflects the real state instead of showing protection that is not active.
- Capture from a worker thread no longer traps in AppKit (`Must only be used from the main thread`).
- Hiding the overlay no longer terminates the process: the app is hidden with `NSApp hide:`
  instead of ordering out the last window.
- Duplicate hotkey listeners no longer trigger two parallel captures.
- API key status checks no longer raise the macOS login-password dialog.

[0.1.1]: https://github.com/Neksand/TrueHalo/releases/tag/v0.1.1
[0.1.0]: https://github.com/Neksand/TrueHalo/releases/tag/v0.1.0
[Unreleased]: https://github.com/Neksand/TrueHalo/compare/v0.1.1...HEAD

## [0.2.0] — 2026-10-26

- Cost dashboard: Usage tab (cards, per-model table, 7-day chart).
- Model compare: split-view framework (Side A / Side B) with toggle.
- All phases completed; open-source release v0.2.0.

