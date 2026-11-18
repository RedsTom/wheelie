#!/usr/bin/env bash
# Assembles Wheelie.app, a .dmg and a portable .zip from a universal binary.
#
# Required environment:
#   VERSION   release version, e.g. 0.1.3-beta.1
#   BIN       path to the universal `wheelie` binary
#   OUT_DIR   output directory for the artifacts
set -euo pipefail

: "${VERSION:?}"
: "${BIN:?}"
: "${OUT_DIR:?dist}"
mkdir -p "$OUT_DIR"

APP="$OUT_DIR/Wheelie.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/wheelie"
chmod +x "$APP/Contents/MacOS/wheelie"
sed "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"

# App icon: render the .iconset from the 256 px logo (upscaled where needed).
ICONSET="$(mktemp -d)/appicon.iconset"
mkdir -p "$ICONSET"
SRC="assets/logo/logo-4x.png"
gen() { sips -z "$2" "$2" "$SRC" --out "$ICONSET/$1" >/dev/null; }
gen icon_16x16.png 16
gen icon_16x16@2x.png 32
gen icon_32x32.png 32
gen icon_32x32@2x.png 64
gen icon_128x128.png 128
gen icon_128x128@2x.png 256
gen icon_256x256.png 256
gen icon_256x256@2x.png 512
gen icon_512x512.png 512
gen icon_512x512@2x.png 1024
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$(dirname "$ICONSET")"

# Ad-hoc signature: required on Apple Silicon and quiets Gatekeeper for
# locally downloaded builds that have no developer certificate.
codesign --force --deep --sign - "$APP"

DMG="$OUT_DIR/Wheelie-$VERSION-macos-universal.dmg"
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/Wheelie.app"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Wheelie $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"

ZIP="$OUT_DIR/Wheelie-$VERSION-macos-portable.zip"
ditto -c -k --keepParent "$APP" "$ZIP"

ls -lh "$OUT_DIR"
