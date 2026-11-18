#!/usr/bin/env bash
# Builds the portable .tar.gz for Wheelie: binary plus desktop entry and
# icons, ready to run from any directory.
#
# Required environment:
#   VERSION   release version, e.g. 0.1.3-beta.1
#   BIN       path to the release `wheelie` binary
#   OUT_DIR   output directory for the artifact
set -euo pipefail

: "${VERSION:?}"
: "${BIN:?}"
: "${OUT_DIR:?dist}"
mkdir -p "$OUT_DIR"

ROOT="$(mktemp -d)/Wheelie-$VERSION"
mkdir -p "$ROOT/icons"
cp "$BIN" "$ROOT/wheelie"
chmod +x "$ROOT/wheelie"
cp packaging/linux/wheelie.desktop "$ROOT/icons/"
cp assets/logo/logo-4x.png "$ROOT/icons/wheelie.png"
cp assets/logo/logo.svg "$ROOT/icons/wheelie.svg"
cp LICENSE "$ROOT/LICENSE"

cat > "$ROOT/README-portable.txt" <<EOF
Wheelie $VERSION — portable Linux build

Run the app:            ./wheelie
To get a menu entry, copy icons/wheelie.desktop to ~/.local/share/applications
and icons/wheelie.png to ~/.local/share/icons/hicolor/256x256/apps/wheelie.png.
EOF

OUT="$OUT_DIR/Wheelie-$VERSION-linux-x64-portable.tar.gz"
tar -C "$(dirname "$ROOT")" -czf "$OUT" "$(basename "$ROOT")"
ls -lh "$OUT_DIR"
