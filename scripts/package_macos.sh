#!/usr/bin/env bash
# Package SlimBrave as a macOS .app bundle.
#
#   ./scripts/package_macos.sh                          # SlimBrave.app in dist/
#   ./scripts/package_macos.sh --dmg                    # plus a compressed .dmg
#   ./scripts/package_macos.sh --arm64 --dmg            # arm64 build (Apple Silicon)
#   ./scripts/package_macos.sh --x86_64 --dmg           # x86_64 build (Intel)
#   ./scripts/package_macos.sh --universal --dmg        # universal (arm64 + x86_64) build
#
# The bundle is ad-hoc code-signed so it launches locally without Gatekeeper
# friction. Distribution builds should replace `-s -` with a Developer ID.

set -euo pipefail
cd "$(dirname "$0")/.."

APP_NAME="SlimBrave"
EXECUTABLE="slimbrave"
VERSION="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
ICON="assets/AppIcon.icns"
TAG=""
DMG=0
for arg in "$@"; do
    case "$arg" in
        --dmg) DMG=1 ;;
        --arm64) TAG="arm64" ;;
        --x86_64) TAG="x86_64" ;;
        --universal) TAG="universal" ;;
        *) echo "unknown argument: $arg" >&2; exit 1 ;;
    esac
done

case "$TAG" in
    universal)
        echo "==> universal build (aarch64 + x86_64)"
        rustup target add aarch64-apple-darwin x86_64-apple-darwin
        cargo build --release --target aarch64-apple-darwin
        cargo build --release --target x86_64-apple-darwin
        mkdir -p dist
        lipo -create -output "dist/${EXECUTABLE}-universal" \
            "target/aarch64-apple-darwin/release/${EXECUTABLE}" \
            "target/x86_64-apple-darwin/release/${EXECUTABLE}"
        BIN="dist/${EXECUTABLE}-universal"
        ;;
    arm64)
        echo "==> arm64 build"
        rustup target add aarch64-apple-darwin
        cargo build --release --target aarch64-apple-darwin
        BIN="target/aarch64-apple-darwin/release/${EXECUTABLE}"
        ;;
    x86_64)
        echo "==> x86_64 build"
        rustup target add x86_64-apple-darwin
        cargo build --release --target x86_64-apple-darwin
        BIN="target/x86_64-apple-darwin/release/${EXECUTABLE}"
        ;;
    *)
        echo "==> native release build"
        cargo build --release
        BIN="target/release/${EXECUTABLE}"
        ;;
esac

if [ -n "$TAG" ]; then
    BUNDLE="dist/${APP_NAME}-${TAG}.app"
else
    BUNDLE="dist/${APP_NAME}.app"
fi

echo "==> assembling ${BUNDLE}"
rm -rf "$BUNDLE"
mkdir -p "${BUNDLE}/Contents/MacOS" "${BUNDLE}/Contents/Resources"
cp "$BIN" "${BUNDLE}/Contents/MacOS/${APP_NAME}"

if [ -f "$ICON" ]; then
    cp "$ICON" "${BUNDLE}/Contents/Resources/AppIcon.icns"
fi

cat > "${BUNDLE}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.slimbrave.app</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHumanReadableCopyright</key>
    <string>GPL-3.0</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
</dict>
</plist>
PLIST

echo "==> ad-hoc code signing"
codesign --force --deep --sign - "${BUNDLE}"

echo "==> verification"
codesign --verify --deep --strict "${BUNDLE}" && echo "signature OK"
plutil -lint "${BUNDLE}/Contents/Info.plist" >/dev/null

if [ "$DMG" = "1" ]; then
    echo "==> building dmg"
    DMG_NAME="dist/${APP_NAME}-${VERSION}${TAG:+-${TAG}}.dmg"
    rm -f "$DMG_NAME"
    hdiutil create -volname "${APP_NAME}" -srcfolder "${BUNDLE}" -ov -format UDZO "$DMG_NAME" >/dev/null
    echo "==> ${DMG_NAME}"
fi

echo "==> done: ${BUNDLE}"
