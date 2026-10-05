#!/usr/bin/env bash
set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "[!] Please run as root (sudo ./install.sh)"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BIN_PATH="RustCooling"
if [ ! -f "$BIN_PATH" ]; then
  if [ -f "$SCRIPT_DIR/RustCooling" ]; then
    BIN_PATH="$SCRIPT_DIR/RustCooling"
  elif [ -f "$REPO_ROOT/target/release/RustCooling" ]; then
    BIN_PATH="$REPO_ROOT/target/release/RustCooling"
  fi
fi

if [ ! -f "$BIN_PATH" ]; then
  echo "[!] Error: RustCooling binary not found."
  exit 1
fi

echo "[*] Installing RustCooling binary to /usr/local/bin..."
install -m 755 "$BIN_PATH" /usr/local/bin/RustCooling
ln -sf /usr/local/bin/RustCooling /usr/local/bin/rustcooling

RULES_FILE="$SCRIPT_DIR/99-idcooling.rules"
[ -f "$RULES_FILE" ] || RULES_FILE="99-idcooling.rules"

echo "[*] Installing udev rules..."
install -m 644 "$RULES_FILE" /etc/udev/rules.d/99-idcooling.rules
udevadm control --reload-rules && udevadm trigger

LOGO_FILE="$SCRIPT_DIR/logo.png"
[ -f "$LOGO_FILE" ] || LOGO_FILE="$REPO_ROOT/assets/icons/logo_256.png"

DESKTOP_FILE="$SCRIPT_DIR/rustcooling.desktop"
[ -f "$DESKTOP_FILE" ] || DESKTOP_FILE="rustcooling.desktop"

SERVICE_FILE="$SCRIPT_DIR/rustcooling.service"
[ -f "$SERVICE_FILE" ] || SERVICE_FILE="rustcooling.service"

echo "[*] Installing desktop entry, icon, and systemd service..."
install -d /usr/share/icons/hicolor/256x256/apps
install -m 644 "$LOGO_FILE" /usr/share/icons/hicolor/256x256/apps/rustcooling.png
install -m 644 "$DESKTOP_FILE" /usr/share/applications/rustcooling.desktop

if [ -f "$SERVICE_FILE" ]; then
    install -d /usr/lib/systemd/user
    install -m 644 "$SERVICE_FILE" /usr/lib/systemd/user/rustcooling.service
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

echo "[✓] RustCooling installed successfully!"
echo "    - Launch GUI: 'rustcooling' or from application menu"
echo "    - Launch TUI: 'rustcooling --tui'"
echo "    - Check status: 'rustcooling --status'"
echo "    - Background daemon: 'systemctl --user enable --now rustcooling'"