## TrueHalo v0.4.4

**Release date:** 2026-11-19 · **License:** GPL-3.0-only · **Platforms:** macOS Apple Silicon (arm64) · Windows x64 · Linux x64

This is a maintenance release with no functional changes. It refreshes the dependency tree and closes
the one open RustSec advisory.

### Security

- **SQLite advisory fixed** — `libsqlite3-sys` moved to 0.40 through rusqlite 0.42, closing
  RUSTSEC-2026-0459 / CVE-2026-119375.
- **Audits in CI** — every push and pull request now runs production `npm audit` and `cargo audit`.
  Both report no vulnerabilities; the RustSec scan keeps three warnings for unmaintained or unsound
  transitive crates with no fixed release.

### Changed

- **Dependency refresh** — Tauri 2.12.7, Vite 8.3.8, Vitest 5.1.1, ESLint 10.15, TypeScript 5.9.3,
  react-markdown 10.1, Lucide 1.76, Tailwind Merge 2.6.2, base64 0.23, ashpd 0.14 (zbus 6) and
  security-framework 3.7.
- **Packaging** — release workflow uses `actions/download-artifact@8`.

### Install

- macOS (arm64): `TrueHalo-0.4.4-Installer.dmg`
- Windows (x64): `TrueHalo_0.4.4_x64-setup.exe`
- Linux (x64): `TrueHalo_0.4.4_amd64.deb` or `TrueHalo_0.4.4_amd64.AppImage`

### Verification

- `npm run build`, `npm run lint`, `npx tsc --noEmit`, `npm test` — 12/12
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` — 15/15
- `npm audit --omit=dev --audit-level=high` and `cargo audit` — no vulnerabilities
- CI passed on macOS, Windows and Linux
