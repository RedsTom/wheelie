#!/usr/bin/env bash
# Builds an x86_64 AppImage for Wheelie from the release binary.
#
# Required environment:
#   VERSION   release version, e.g. 0.4.1-beta.1
#   BIN       path to the release `wheelie` binary
#   OUT_DIR   output directory for the artifact
#
# The AppImage carries the executable, desktop entry and icon. It still needs a
# host graphics stack (GTK 3, X11/XCB, Wayland, Vulkan); see the README.
set -euo pipefail

: "${VERSION:?}"
: "${BIN:?}"
: "${OUT_DIR:?dist}"
mkdir -p "$OUT_DIR"

APPDIR="$(mktemp -d)/Wheelie.AppDir"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/.diricons"
cp "$BIN" "$APPDIR/usr/bin/wheelie"
chmod +x "$APPDIR/usr/bin/wheelie"

sed "s|^Exec=.*|Exec=AppRun %U|" packaging/linux/wheelie.desktop > "$APPDIR/wheelie.desktop"
cp assets/logo/logo-4x.png "$APPDIR/wheelie.png"

cat > "$APPDIR/AppRun" <<'EOF'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/wheelie" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# Pinned dated appimagetool release plus its published SHA-256. The "continuous"
# tag is rolling and must not be used in a reproducible release pipeline.
TOOL_URL="https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage"
TOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
TOOL_BIN="$(mktemp -d)/appimagetool"
curl -fsSL "$TOOL_URL" -o "$TOOL_BIN"
printf '%s  %s\n' "$TOOL_SHA256" "$TOOL_BIN" | sha256sum -c -
chmod +x "$TOOL_BIN"
if ! "$TOOL_BIN" --help >/dev/null 2>&1; then
  (cd "$(dirname "$TOOL_BIN")" && "$TOOL_BIN" --appimage-extract >/dev/null)
  TOOL_BIN="$(dirname "$TOOL_BIN")/squashfs-root/AppRun"
fi

OUT="$OUT_DIR/Wheelie-$VERSION-x86_64.AppImage"
ARCH=x86_64 "$TOOL_BIN" "$APPDIR" "$OUT"
ls -lh "$OUT_DIR"
