#!/usr/bin/env bash
set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "[!] Please run as root (sudo ./install.sh)"
  exit 1
fi

echo "[*] Installing RustCooling binary to /usr/local/bin..."
install -m 755 RustCooling /usr/local/bin/RustCooling
ln -sf /usr/local/bin/RustCooling /usr/local/bin/rustcooling

echo "[*] Installing udev rules..."
install -m 644 99-idcooling.rules /etc/udev/rules.d/99-idcooling.rules
udevadm control --reload-rules && udevadm trigger

echo "[*] Installing desktop entry, icon, and systemd service..."
install -d /usr/share/icons/hicolor/256x256/apps
install -m 644 logo.png /usr/share/icons/hicolor/256x256/apps/rustcooling.png
install -m 644 rustcooling.desktop /usr/share/applications/rustcooling.desktop

if [ -f rustcooling.service ]; then
    install -d /usr/lib/systemd/user
    install -m 644 rustcooling.service /usr/lib/systemd/user/rustcooling.service
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

echo "[✓] RustCooling installed successfully!"
echo "    - Launch GUI: 'rustcooling' or from application menu"
echo "    - Launch TUI: 'rustcooling --tui'"
echo "    - Check status: 'rustcooling --status'"
echo "    - Background daemon: 'systemctl --user enable --now rustcooling'"