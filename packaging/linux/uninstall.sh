#!/usr/bin/env bash
set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "[!] Please run as root (sudo ./uninstall.sh)"
  exit 1
fi

rm -f /usr/local/bin/RustCooling
rm -f /usr/local/bin/rustcooling-cli
rm -f /usr/local/bin/rustcooling
rm -f /usr/bin/RustCooling
rm -f /usr/bin/rustcooling-cli
rm -f /etc/udev/rules.d/99-idcooling.rules
rm -f /usr/share/applications/rustcooling.desktop
rm -f /usr/share/icons/hicolor/256x256/apps/rustcooling.png
rm -f /usr/lib/systemd/user/rustcooling.service
rm -rf /usr/share/fonts/truetype/rustcooling

# Clean up desktop shortcuts from user desktop directories if present
for u in /home/*; do
  [ -d "$u" ] || continue
  for d in "Desktop" "Рабочий стол"; do
    rm -f "$u/$d/rustcooling.desktop"
  done
done

if command -v fc-cache >/dev/null 2>&1; then
    fc-cache -f >/dev/null 2>&1 || true
fi

udevadm control --reload-rules && udevadm trigger
echo "[✓] RustCooling uninstalled successfully."