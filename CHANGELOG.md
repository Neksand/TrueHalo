# Changelog

All notable changes to TrueHalo are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

