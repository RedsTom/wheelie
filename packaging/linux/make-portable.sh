#!/usr/bin/env bash
# Builds the Linux binary archive for Wheelie: the executable, icons and an
# installer for a desktop entry with an absolute Exec path. Shared libraries
# are not bundled; see the README for the required runtime packages.
#
# The historical `portable` filename is kept so the eight-asset release
# manifest stays stable.
#
# Required environment:
#   VERSION   release version, e.g. 0.4.2-beta.1
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

# The placeholder is replaced at install time, when the extraction directory
# is known. A desktop entry built here could only point at a temporary path.
cat > "$ROOT/icons/wheelie.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Wheelie
GenericName=Knob controller
Comment=Turn a keyboard knob into a configurable radial wheel
Exec=__WHEELIE_DIR__/wheelie
Icon=__WHEELIE_DIR__/icons/wheelie.png
Categories=Utility;
Terminal=false
StartupNotify=true
Keywords=knob;wheel;volume;brightness;scroll;
EOF
cp assets/logo/logo-4x.png "$ROOT/icons/wheelie.png"
cp assets/logo/logo.svg "$ROOT/icons/wheelie.svg"
cp LICENSE "$ROOT/LICENSE"

cat > "$ROOT/install-desktop.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
escaped="$(printf '%s' "$here" | sed 's/[&|\\]/\\&/g')"
target="$HOME/.local/share/applications/wheelie.desktop"
mkdir -p "$(dirname "$target")"
sed "s|__WHEELIE_DIR__|$escaped|g" "$here/icons/wheelie.desktop" > "$target"
echo "Installed $target with Exec=$here/wheelie"
EOF
chmod +x "$ROOT/install-desktop.sh"

cat > "$ROOT/README-portable.txt" <<EOF
Wheelie $VERSION — Linux binary archive

Run the app:            ./wheelie
Install a menu entry:   ./install-desktop.sh

This archive does not bundle shared libraries. Install the runtime packages
listed in the README (GTK 3, libayatana-appindicator3, X11/XCB, Wayland,
Vulkan and the other dependencies) before launching Wheelie.

The desktop entry installed by ./install-desktop.sh uses the absolute path of
the extracted directory in its Exec line, so moving the directory means
running the installer again.
EOF

OUT="$OUT_DIR/Wheelie-$VERSION-linux-x64-portable.tar.gz"
tar -C "$(dirname "$ROOT")" -czf "$OUT" "$(basename "$ROOT")"
ls -lh "$OUT_DIR"
