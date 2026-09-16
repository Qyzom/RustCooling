#!/usr/bin/env bash
set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "[!] Please run as root (sudo ./install.sh)"
  exit 1
fi

echo "[*] Installing RustCooling binary to /usr/local/bin..."
install -m 755 RustCooling /usr/local/bin/RustCooling

echo "[*] Installing udev rules..."
install -m 644 99-idcooling.rules /etc/udev/rules.d/99-idcooling.rules
udevadm control --reload-rules && udevadm trigger

echo "[*] Installing desktop entry and icon..."
install -d /usr/share/icons/hicolor/256x256/apps
install -m 644 logo.png /usr/share/icons/hicolor/256x256/apps/rustcooling.png
install -m 644 rustcooling.desktop /usr/share/applications/rustcooling.desktop

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

echo "[✓] RustCooling installed successfully! You can launch it from the app menu or run 'RustCooling'."