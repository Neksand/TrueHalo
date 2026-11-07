## TrueHalo v0.4.1

**Release date:** 2026-11-07 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64 · Linux x64

### Added

- **Check for updates** — the Settings footer compares the running version to the latest GitHub release and links to the download when a newer build exists. Manual install, no auto-download.
- **Frontend quality gates** — ESLint and Vitest now run in CI on all three platforms, alongside formatting, Clippy and the Rust tests.

### Changed

- Update checks time out after 8 seconds and show a failure message instead of doing nothing.
- CI moved to Node 24 LTS; Dependabot keeps npm, Cargo and GitHub Actions current.
- Vite 8, Vitest 5 and ESLint 10; `npm audit` is clean.

### Fixed

- AUR metadata now points at the v0.4.0 release instead of v0.3.1, so `truehalo-bin` can build.
- `cargo fmt --check` passes; the README roadmap and architecture doc match the shipped Linux support.

### Install

- macOS (arm64): `TrueHalo-0.4.1-Installer.dmg`
- Windows (x64): `TrueHalo_0.4.1_x64-setup.exe`
- Linux (x64): `TrueHalo_0.4.1_amd64.deb` or `TrueHalo_0.4.1_amd64.AppImage`

### Verification

- `npm run build`, `npm run lint`, `npm test`, `npx tsc --noEmit`
- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
- DMG signature verified with `codesign --verify --deep --strict`
