#!/usr/bin/env bash
set -e

APP_NAME="TrueHalo.app"
BUNDLE_ID="com.truehalo.desktop"
SOURCE_APP="$(cd "$(dirname "$0")" && pwd)/src-tauri/target/debug/bundle/macos/${APP_NAME}"
TARGET_DIR="/Applications"
TARGET_APP="${TARGET_DIR}/${APP_NAME}"

echo "=================================================="
echo "         TrueHalo Installer for macOS             "
echo "=================================================="

if [ ! -d "$SOURCE_APP" ]; then
    echo "❌ Error: Source application not found at:"
    echo "   $SOURCE_APP"
    echo "Run 'bash scripts/build.sh' first (it also signs the app)."
    exit 1
fi

# Verify the bundle signature: an unsigned bundle makes macOS lose the Screen
# Recording permission on every launch ("granted, but asks again" loop).
echo "🔏 Verifying code signature..."
if ! codesign --verify --deep --strict "$SOURCE_APP" 2>/dev/null; then
    echo "⚠️  The app signature is invalid. Rebuild with 'bash scripts/build.sh'."
    echo "   Continuing, but macOS may ask for screen recording access again."
else
    echo "✅ Signature is valid."
    codesign -d -r- "$SOURCE_APP" 2>&1 | sed 's/^/   /'
fi

echo "📦 Installing TrueHalo to $TARGET_DIR..."

if [ -d "$TARGET_APP" ]; then
    echo "ℹ️  Removing existing installation at $TARGET_APP..."
    pkill -f "TrueHalo.app/Contents/MacOS/truehalo" 2>/dev/null || true
    rm -rf "$TARGET_APP"
fi

cp -R "$SOURCE_APP" "$TARGET_DIR/"

echo "🛡️  Clearing macOS quarantine attributes..."
xattr -cr "$TARGET_APP" 2>/dev/null || true

if [ "${RESET_TCC:-0}" = "1" ]; then
    echo "🧹 Resetting stale Screen Recording permission record..."
    tccutil reset ScreenCapture "$BUNDLE_ID" 2>/dev/null || true
fi

echo "✅ TrueHalo installed successfully into /Applications!"
echo ""
echo "ℹ️  If screen recording access was already granted to an older build and"
echo "   screenshots ask for permission again — turn the TrueHalo switch OFF and"
echo "   ON in «System Settings → Privacy & Security → Screen Recording»."
echo ""
echo "🚀 Starting TrueHalo..."

open "$TARGET_APP"
