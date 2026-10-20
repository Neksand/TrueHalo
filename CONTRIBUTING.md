# Contributing to TrueHalo

Thanks for taking the time to contribute!

## Before you start

- **Bugs / features**: open an issue first for anything larger than a small fix.
- **Security**: do **not** open a public issue for vulnerabilities — see [SECURITY.md](SECURITY.md).
- By contributing you agree that your work is licensed under [GPL-3.0](LICENSE).

## Development setup

```bash
git clone https://github.com/Neksand/TrueHalo.git
cd TrueHalo
npm install
npm run tauri dev
```

Requirements: macOS 13+, Xcode Command Line Tools, Node.js 18+, Rust stable.

> Signing matters. `npm run tauri dev` produces an unsigned binary, and macOS will not remember
> Screen Recording permission for it. To test the capture pipeline, build a signed app with
> `npm run app:install`.

## Project layout

| Path | What lives there |
|---|---|
| `src/` | React UI (chat, providers, capture, settings) |
| `src/lib/` | Typed IPC bridge and shared types |
| `src-tauri/src/ai/` | Provider clients and SSE streaming |
| `src-tauri/src/capture/` | Screen capture, compression, permissions |
| `src-tauri/src/auth/` | macOS Keychain integration |
| `src-tauri/src/commands/` | IPC commands exposed to the UI |
| `src-tauri/src/overlay.rs` | Overlay show/hide/is_visible |
| `scripts/` | Build, signing and packaging |
| `docs/` | Architecture notes and screenshots |

## Rules of the road

1. **AppKit is main-thread only.** Never call AppKit from a worker thread; use
   `overlay::*` helpers or `run_on_main_thread`.
2. **Secrets never leave Rust.** Do not return API keys to the webview or write them to SQLite.
3. **No background capture.** Screenshots happen only after an explicit user action.
4. **Keep the UI English-only.** No localized strings in the interface.
5. **No new telemetry**, analytics or third-party trackers.

## Checks before a pull request

```bash
npm run build                                   # tsc + vite
cargo test --manifest-path src-tauri/Cargo.toml  # unit tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

## Commits and pull requests

- Small, focused commits with an imperative subject (`Fix overlay hide on inactive app`).
- Describe *what* changed and *why*; screenshots help for UI changes.
- One logical change per pull request.

## Release checklist (maintainers)

1. Bump `version` in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. Update `CHANGELOG.md`.
3. `npm run app:build -- --release --dmg --install` to build and sign locally.
4. `git tag vX.Y.Z && git push origin vX.Y.Z`.
5. Attach the DMG to the GitHub release.
