#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

VERSION="${1:-1.0.0}"
VERSION="${VERSION#v}"
OUTPUT_DIR="${2:-$REPO_ROOT}"

cd "$REPO_ROOT"

# --- 1. GUI Package (.rpm) ---
RPM_TOP="$REPO_ROOT/target/rpmbuild-gui"
rm -rf "$RPM_TOP"
mkdir -p "$RPM_TOP"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

cat << EOF > "$RPM_TOP/SPECS/rustcooling-gui.spec"
Name:           RustCooling-GUI
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Ultra-lightweight LCD Display controller for ID-COOLING FX series liquid coolers (GUI)
License:        MIT
URL:            https://github.com/Qyzom/RustCooling
Provides:       rustcooling
Conflicts:      RustCooling-TUI

%description
High-performance, low-overhead native hardware monitor and USB HID LCD display
controller written in Rust with modern Slint graphical interface and system tray.

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/lib/systemd/user
mkdir -p %{buildroot}/usr/lib/udev/rules.d
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps

install -m 755 $REPO_ROOT/target/release/RustCooling %{buildroot}/usr/bin/RustCooling
ln -sf RustCooling %{buildroot}/usr/bin/rustcooling
install -m 644 $REPO_ROOT/packaging/linux/99-idcooling.rules %{buildroot}/usr/lib/udev/rules.d/99-idcooling.rules
install -m 644 $REPO_ROOT/packaging/linux/rustcooling.desktop %{buildroot}/usr/share/applications/rustcooling.desktop
install -m 644 $REPO_ROOT/packaging/linux/rustcooling.service %{buildroot}/usr/lib/systemd/user/rustcooling.service
install -m 644 $REPO_ROOT/assets/icons/logo_256.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/rustcooling.png

%post
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || :
    udevadm trigger || :
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || :
fi

%files
/usr/bin/RustCooling
/usr/bin/rustcooling
/usr/lib/udev/rules.d/99-idcooling.rules
/usr/share/applications/rustcooling.desktop
/usr/lib/systemd/user/rustcooling.service
/usr/share/icons/hicolor/256x256/apps/rustcooling.png
EOF

rpmbuild --define "_topdir $RPM_TOP" -bb "$RPM_TOP/SPECS/rustcooling-gui.spec"
RPM_BUILT=$(ls "$RPM_TOP"/RPMS/*/*.rpm | head -n1)
cp "$RPM_BUILT" "$OUTPUT_DIR/RustCooling-GUI-${VERSION}.rpm"
rm -rf "$RPM_TOP"
echo "[✓] Built $OUTPUT_DIR/RustCooling-GUI-${VERSION}.rpm"

# --- 2. TUI / CLI Package (.rpm) ---
RPM_TOP="$REPO_ROOT/target/rpmbuild-tui"
rm -rf "$RPM_TOP"
mkdir -p "$RPM_TOP"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

cat << EOF > "$RPM_TOP/SPECS/rustcooling-tui.spec"
Name:           RustCooling-TUI
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Ultra-lightweight LCD Display controller for ID-COOLING FX series liquid coolers (TUI/CLI)
License:        MIT
URL:            https://github.com/Qyzom/RustCooling
Provides:       rustcooling
Conflicts:      RustCooling-GUI

%description
High-performance, low-overhead native hardware monitor and USB HID LCD display
controller written in Rust with interactive Ratatui terminal dashboard and headless daemon.

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/lib/systemd/user
mkdir -p %{buildroot}/usr/lib/udev/rules.d
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps

install -m 755 $REPO_ROOT/target/release/rustcooling-cli %{buildroot}/usr/bin/rustcooling-cli
ln -sf rustcooling-cli %{buildroot}/usr/bin/rustcooling
install -m 644 $REPO_ROOT/packaging/linux/99-idcooling.rules %{buildroot}/usr/lib/udev/rules.d/99-idcooling.rules
install -m 644 $REPO_ROOT/packaging/linux/rustcooling-tui.desktop %{buildroot}/usr/share/applications/rustcooling-tui.desktop
install -m 644 $REPO_ROOT/packaging/linux/rustcooling.service %{buildroot}/usr/lib/systemd/user/rustcooling.service
install -m 644 $REPO_ROOT/assets/icons/logo_256.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/rustcooling.png

%post
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || :
    udevadm trigger || :
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || :
fi

%files
/usr/bin/rustcooling-cli
/usr/bin/rustcooling
/usr/lib/udev/rules.d/99-idcooling.rules
/usr/share/applications/rustcooling-tui.desktop
/usr/lib/systemd/user/rustcooling.service
/usr/share/icons/hicolor/256x256/apps/rustcooling.png
EOF

rpmbuild --define "_topdir $RPM_TOP" -bb "$RPM_TOP/SPECS/rustcooling-tui.spec"
RPM_BUILT=$(ls "$RPM_TOP"/RPMS/*/*.rpm | head -n1)
cp "$RPM_BUILT" "$OUTPUT_DIR/RustCooling-TUI-${VERSION}.rpm"
rm -rf "$RPM_TOP"
echo "[✓] Built $OUTPUT_DIR/RustCooling-TUI-${VERSION}.rpm"
