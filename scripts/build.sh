#!/usr/bin/env bash
#
# Build + sign TrueHalo and (optionally) install it into /Applications.
#
# Usage:
#   bash scripts/build.sh              # build + sign only
#   bash scripts/build.sh --install    # build + sign + install + launch
#   bash scripts/build.sh --release    # optimized build (default: debug)
#   bash scripts/build.sh --dmg        # also create an installer .dmg
#   bash scripts/build.sh --reset-tcc  # also clear the ScreenCapture TCC grant
#                                      # (only needed after switching signing identity)
#
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT_DIR}"

IDENTITY_NAME="${SIGN_IDENTITY:-TrueHalo Local Dev}"
BUNDLE_ID="com.truehalo.desktop"

PROFILE="debug"
BUILD_FLAG="--debug"
DO_INSTALL=0
DO_DMG=0
RESET_TCC=0

for arg in "$@"; do
  case "${arg}" in
    --install) DO_INSTALL=1 ;;
    --release) PROFILE="release"; BUILD_FLAG="" ;;
    --dmg) DO_DMG=1 ;;
    --reset-tcc) RESET_TCC=1 ;;
    *) echo "Unknown argument: ${arg}"; exit 1 ;;
  esac
done

info() { printf '\033[1;34m→\033[0m %s\n' "$1"; }
ok() { printf '\033[1;32m✓\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m!\033[0m %s\n' "$1"; }

# ---------------------------------------------------------------------------
# 1. Stable local signing identity (best effort).
# ---------------------------------------------------------------------------
if [ "${USE_ADHOC:-0}" != "1" ]; then
  bash scripts/setup-signing.sh || warn "Continuing with an ad-hoc signature."
fi

SIGN_ARG="-"
if security find-identity -v -p codesigning 2>/dev/null | grep -q "${IDENTITY_NAME}"; then
  SIGN_ARG="${IDENTITY_NAME}"
  ok "Signing: ${IDENTITY_NAME} (screen recording access survives rebuilds)"
else
  warn "Stable signing unavailable — using ad-hoc. macOS may ask for screen recording access again after a rebuild."
fi

# ---------------------------------------------------------------------------
# 2. Build frontend + Tauri bundle.
# ---------------------------------------------------------------------------
info "Building the frontend..."
npm run build

info "Building the app (${PROFILE})..."
APPLE_SIGNING_IDENTITY="${SIGN_ARG}" npx tauri build ${BUILD_FLAG}

APP_PATH="src-tauri/target/${PROFILE}/bundle/macos/TrueHalo.app"
if [ ! -d "${APP_PATH}" ]; then
  echo "Bundle not found: ${APP_PATH}"
  exit 1
fi

# ---------------------------------------------------------------------------
# 3. Re-sign the whole bundle so Info.plist + resources are sealed.
#    This is what makes the TCC (Screen Recording) grant durable.
# ---------------------------------------------------------------------------
info "Signing the bundle..."
codesign --force --deep --options runtime --sign "${SIGN_ARG}" \
  --identifier "${BUNDLE_ID}" "${APP_PATH}"

info "Verifying the signature..."
codesign --verify --deep --strict --verbose=2 "${APP_PATH}"

ok "Current designated requirement:"
codesign -d -r- "${APP_PATH}" 2>&1 | sed 's/^/    /'

# ---------------------------------------------------------------------------
# 4. Optional install.
# ---------------------------------------------------------------------------
if [ "${DO_INSTALL}" = "1" ]; then
  if [ "${DO_DMG}" = "1" ]; then
    info "Creating the DMG..."
    DMG_DIR="$(mktemp -d)"
    cp -R "${APP_PATH}" "${DMG_DIR}/"
    ln -s /Applications "${DMG_DIR}/Applications"
    hdiutil create -volname "TrueHalo" -srcfolder "${DMG_DIR}" -ov -format UDZO \
      "TrueHalo-0.2.1-Installer.dmg" >/dev/null
    rm -rf "${DMG_DIR}"
    ok "DMG: TrueHalo-0.2.1-Installer.dmg"
  fi

  if [ "${RESET_TCC}" = "1" ]; then
    info "Resetting the TCC record for ScreenCapture..."
    tccutil reset ScreenCapture "${BUNDLE_ID}" 2>/dev/null || \
      sudo tccutil reset ScreenCapture "${BUNDLE_ID}" 2>/dev/null || \
      warn "Could not reset TCC — turn the TrueHalo switch off and on in System Settings."
  fi

  info "Installing to /Applications..."
  pkill -f "TrueHalo.app/Contents/MacOS/truehalo" 2>/dev/null || true
  rm -rf "/Applications/TrueHalo.app"
  cp -R "${APP_PATH}" "/Applications/"
  xattr -cr "/Applications/TrueHalo.app" 2>/dev/null || true

  ok "Done. Starting TrueHalo..."
  open "/Applications/TrueHalo.app"
fi

ok "Build complete: ${APP_PATH}"
