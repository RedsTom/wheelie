#!/usr/bin/env bash
# Builds the .deb package for Wheelie from the release binary.
#
# Required environment:
#   VERSION   release version, e.g. 0.1.3-beta.1 (Debian revisions use ~)
#   BIN       path to the release `wheelie` binary
#   OUT_DIR   output directory for the artifact
set -euo pipefail

: "${VERSION:?}"
: "${BIN:?}"
: "${OUT_DIR:?dist}"
mkdir -p "$OUT_DIR"

# Debian/rpm versions reject '-': prerelease suffixes map to '~' (note that a
# bare ${VERSION//-/~} would tilde-expand the replacement to $HOME in bash).
SAFE="$(printf '%s' "$VERSION" | tr '-' '~')"
STAGE="$(mktemp -d)/debroot"

install -D -m 755 "$BIN" "$STAGE/usr/bin/wheelie"
install -D -m 644 packaging/linux/wheelie.desktop "$STAGE/usr/share/applications/wheelie.desktop"
install -D -m 644 assets/logo/logo-4x.png "$STAGE/usr/share/icons/hicolor/256x256/apps/wheelie.png"
install -D -m 644 assets/logo/logo.svg "$STAGE/usr/share/icons/hicolor/scalable/apps/wheelie.svg"
install -D -m 644 LICENSE "$STAGE/usr/share/doc/wheelie/copyright"

mkdir -p "$STAGE/DEBIAN"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: wheelie
Version: $SAFE
Architecture: amd64
Maintainer: RedsTom <contact@redstom.dev>
Section: utils
Priority: optional
Installed-Size: $(du -sk "$STAGE/usr" | cut -f1)
Depends: libgtk-3-0t64 | libgtk-3-0, libayatana-appindicator3-1, libxkbcommon0
Homepage: https://github.com/RedsTom/wheelie
Description: Knob utility with a radial wheel UI
 Turn a keyboard knob into a configurable radial wheel. Wheelie captures
 the keys your keyboard software emits for the knob and runs your own
 actions: volume, brightness, scroll, macros, shortcuts and more.
EOF

OUT="$OUT_DIR/wheelie_${SAFE}_amd64.deb"
dpkg-deb --root-owner-group -Zxz -b "$STAGE" "$OUT"
dpkg-deb -I "$OUT"
ls -lh "$OUT_DIR"
