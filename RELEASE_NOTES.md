## TrueHalo v0.4.3

**Release date:** 2026-11-09 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64 · Linux x64

### Fixed

- **Windows connection diagnostics** — a failed model list or stream now reports the real DNS, TCP, TLS or proxy cause instead of just `Failed to connect`. The same chain is written to `%LOCALAPPDATA%\TrueHalo\logs\truehalo.log`.
- **Antivirus / corporate TLS** — Windows builds now trust the system certificate store in addition to bundled roots, so HTTPS inspection no longer rejects provider requests.
- **Credential save verification** — the key is read back after saving; if the credential store did not persist it, the app says so instead of claiming success.
- **No silent unauthenticated requests** — OpenRouter and OpenAI reject keys containing control characters instead of dropping the `Authorization` header.
- **Windows log file** — logs are written under `%LOCALAPPDATA%\TrueHalo\logs` instead of a macOS-only path.

### Changed

- Settings uses platform-neutral credential-store wording on Windows.

### Install

- macOS (arm64): `TrueHalo-0.4.3-Installer.dmg`
- Windows (x64): `TrueHalo_0.4.3_x64-setup.exe`
- Linux (x64): `TrueHalo_0.4.3_amd64.deb` or `TrueHalo_0.4.3_amd64.AppImage`

### Verification

- `npm run build`, `npm run lint`, `npm test`, `npx tsc --noEmit`
- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
- CI passed on macOS, Windows and Linux
