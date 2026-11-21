#!/usr/bin/env bash
# Builds the .rpm package for Wheelie from the release binary.
#
# Required environment:
#   VERSION   release version, e.g. 0.4.1-beta.1 (rpm versions use ~)
#   BIN       path to the release `wheelie` binary
#   OUT_DIR   output directory for the artifact
set -euo pipefail

: "${VERSION:?}"
: "${BIN:?}"
: "${OUT_DIR:?dist}"
mkdir -p "$OUT_DIR"

# rpm versions reject '-': prerelease suffixes map to '~' (a bare
# ${VERSION//-/~} would tilde-expand the replacement to $HOME in bash).
SAFE="$(printf '%s' "$VERSION" | tr '-' '~')"

# Staged install tree that the spec copies verbatim.
STAGE="$(mktemp -d)/stage"
install -D -m 755 "$BIN" "$STAGE/usr/bin/wheelie"
install -D -m 644 packaging/linux/wheelie.desktop "$STAGE/usr/share/applications/wheelie.desktop"
install -D -m 644 assets/logo/logo-4x.png "$STAGE/usr/share/icons/hicolor/256x256/apps/wheelie.png"
install -D -m 644 assets/logo/logo.svg "$STAGE/usr/share/icons/hicolor/scalable/apps/wheelie.svg"
install -D -m 644 LICENSE "$STAGE/usr/share/doc/wheelie/LICENSE"

ROOT="$(mktemp -d)/rpmbuild"
mkdir -p "$ROOT"/{BUILD,RPMS,SPECS,SOURCES,SRPMS}
cp -R "$STAGE" "$ROOT/SOURCES/stage"
sed "s/@VERSION@/$SAFE/g" packaging/linux/wheelie.spec > "$ROOT/SPECS/wheelie.spec"

rpmbuild -bb --define "_topdir $ROOT" --nodeps "$ROOT/SPECS/wheelie.spec"

RPM="$(find "$ROOT/RPMS" -name 'wheelie-*.rpm' | head -n1)"
mv "$RPM" "$OUT_DIR/"
ls -lh "$OUT_DIR"
