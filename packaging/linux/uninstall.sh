#!/usr/bin/env bash
set -e

if [ "\" -ne 0 ]; then
  echo "[!] Please run as root (sudo ./uninstall.sh)"
  exit 1
fi

rm -f /usr/local/bin/RustCooling
rm -f /etc/udev/rules.d/99-idcooling.rules
rm -f /usr/share/applications/rustcooling.desktop
rm -f /usr/share/icons/hicolor/256x256/apps/rustcooling.png

udevadm control --reload-rules && udevadm trigger
echo "[✓] RustCooling uninstalled successfully."