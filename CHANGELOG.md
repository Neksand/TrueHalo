# Changelog

All notable changes to TrueHalo are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Windows build (Credential Manager + Win32 capture backend)
- Universal binary (Apple Silicon + Intel)
- Assistant profiles (interview, meeting, sales)
- Optional click-through mode

## [0.1.0] — 2026-10-20

First public release.

### Added
- Native macOS overlay built with Tauri 2, Rust and React.
- Three AI providers: OpenRouter, Google Gemini and OpenAI, with a live model catalog.
- Streaming responses with cancellation through `Esc` or the Stop button.
- Multimodal requests: region (`⌥⇧2`) and full-screen (`⌥⇧3`) screenshots.
- Screenshot compression presets (Quality / Balanced / Speed) with token-saving history trimming.
- macOS Keychain storage for API keys, with an attributes-only existence check and a one-click
  ACL repair action.
- True transparency with an opacity slider and optional AppKit vibrancy.
- `NSWindowSharingNone` protection: the overlay is excluded from other apps' recordings.
- Accessory mode (hidden from the Dock and `⌘Tab`) with a menu bar icon.
- Window dragging, persisted position/size and global show/hide hotkey (`⌥Space`).
- Local SQLite history and preferences.
- Stable local code signing, so the Screen Recording permission survives rebuilds.

### Changed
- Platform-neutral UI copy: the interface no longer hardcodes macOS-specific wording,
  so the same strings fit the upcoming Windows build.

### Fixed
- Capture from a worker thread no longer traps in AppKit (`Must only be used from the main thread`).
- Hiding the overlay no longer terminates the process: the app is hidden with `NSApp hide:`
  instead of ordering out the last window.
- Duplicate hotkey listeners no longer trigger two parallel captures.
- API key status checks no longer raise the macOS login-password dialog.

[Unreleased]: https://github.com/Neksand/TrueHalo/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Neksand/TrueHalo/releases/tag/v0.1.0
