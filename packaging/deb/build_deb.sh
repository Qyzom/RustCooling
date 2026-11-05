#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

VERSION="${1:-1.0.0}"
VERSION="${VERSION#v}"
OUTPUT_DIR="${2:-$REPO_ROOT}"

cd "$REPO_ROOT"

# --- 1. GUI Package (.deb) ---
DEB_DIR="deb_gui_build"
rm -rf "$DEB_DIR"
mkdir -p "$DEB_DIR/DEBIAN"
mkdir -p "$DEB_DIR/usr/bin"
mkdir -p "$DEB_DIR/usr/lib/systemd/user"
mkdir -p "$DEB_DIR/lib/udev/rules.d"
mkdir -p "$DEB_DIR/usr/share/applications"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/RustCooling "$DEB_DIR/usr/bin/RustCooling"
ln -sf RustCooling "$DEB_DIR/usr/bin/rustcooling"
cp packaging/linux/99-idcooling.rules "$DEB_DIR/lib/udev/rules.d/99-idcooling.rules"
cp packaging/linux/rustcooling.desktop "$DEB_DIR/usr/share/applications/rustcooling.desktop"
cp packaging/linux/rustcooling.service "$DEB_DIR/usr/lib/systemd/user/rustcooling.service"
cp assets/icons/logo_256.png "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/rustcooling.png"

cat << EOF > "$DEB_DIR/DEBIAN/control"
Package: rustcooling-gui
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Qyzom <t0lotsmail@gmail.com>
Depends: libc6, libudev1, libfontconfig1, libgl1, libxkbcommon0
Provides: rustcooling
Conflicts: rustcooling-tui
Description: Ultra-lightweight LCD Display controller for ID-COOLING FX series liquid coolers (GUI)
 High-performance, low-overhead native hardware monitor and USB HID LCD display
 controller written in Rust with modern Slint graphical interface and system tray.
EOF

cat << 'EOF' > "$DEB_DIR/DEBIAN/postinst"
#!/bin/sh
set -e
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
exit 0
EOF
chmod 755 "$DEB_DIR/DEBIAN/postinst"

dpkg-deb --build --root-owner-group "$DEB_DIR" "$OUTPUT_DIR/RustCooling-GUI-${VERSION}.deb"
rm -rf "$DEB_DIR"
echo "[✓] Built $OUTPUT_DIR/RustCooling-GUI-${VERSION}.deb"

# --- 2. TUI / CLI Package (.deb) ---
DEB_DIR="deb_tui_build"
rm -rf "$DEB_DIR"
mkdir -p "$DEB_DIR/DEBIAN"
mkdir -p "$DEB_DIR/usr/bin"
mkdir -p "$DEB_DIR/usr/lib/systemd/user"
mkdir -p "$DEB_DIR/lib/udev/rules.d"
mkdir -p "$DEB_DIR/usr/share/applications"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/rustcooling-cli "$DEB_DIR/usr/bin/rustcooling-cli"
ln -sf rustcooling-cli "$DEB_DIR/usr/bin/rustcooling"
cp packaging/linux/99-idcooling.rules "$DEB_DIR/lib/udev/rules.d/99-idcooling.rules"
cp packaging/linux/rustcooling-tui.desktop "$DEB_DIR/usr/share/applications/rustcooling-tui.desktop"
cp packaging/linux/rustcooling.service "$DEB_DIR/usr/lib/systemd/user/rustcooling.service"
cp assets/icons/logo_256.png "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/rustcooling.png"

cat << EOF > "$DEB_DIR/DEBIAN/control"
Package: rustcooling-tui
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Qyzom <t0lotsmail@gmail.com>
Depends: libc6, libudev1
Provides: rustcooling
Conflicts: rustcooling-gui
Description: Ultra-lightweight LCD Display controller for ID-COOLING FX series liquid coolers (TUI/CLI)
 High-performance, low-overhead native hardware monitor and USB HID LCD display
 controller written in Rust with interactive Ratatui terminal dashboard and headless daemon.
EOF

cat << 'EOF' > "$DEB_DIR/DEBIAN/postinst"
#!/bin/sh
set -e
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
exit 0
EOF
chmod 755 "$DEB_DIR/DEBIAN/postinst"

dpkg-deb --build --root-owner-group "$DEB_DIR" "$OUTPUT_DIR/RustCooling-TUI-${VERSION}.deb"
rm -rf "$DEB_DIR"
echo "[✓] Built $OUTPUT_DIR/RustCooling-TUI-${VERSION}.deb"
