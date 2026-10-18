#!/usr/bin/env bash
set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "[!] Please run as root (sudo ./install.sh)"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

INSTALL_CLI_DEFAULT=false
if [ "$1" = "--cli" ]; then
  INSTALL_CLI_DEFAULT=true
fi

BIN_PATH="RustCooling"
if [ ! -f "$BIN_PATH" ]; then
  if [ -f "$SCRIPT_DIR/RustCooling" ]; then
    BIN_PATH="$SCRIPT_DIR/RustCooling"
  elif [ -f "$REPO_ROOT/target/release/RustCooling" ]; then
    BIN_PATH="$REPO_ROOT/target/release/RustCooling"
  fi
fi

CLI_BIN_PATH="rustcooling-cli"
if [ ! -f "$CLI_BIN_PATH" ]; then
  if [ -f "$SCRIPT_DIR/rustcooling-cli" ]; then
    CLI_BIN_PATH="$SCRIPT_DIR/rustcooling-cli"
  elif [ -f "$REPO_ROOT/target/release/rustcooling-cli" ]; then
    CLI_BIN_PATH="$REPO_ROOT/target/release/rustcooling-cli"
  fi
fi

if [ -f "$BIN_PATH" ]; then
  echo "[*] Installing RustCooling GUI binary to /usr/local/bin..."
  install -m 755 "$BIN_PATH" /usr/local/bin/RustCooling
fi

if [ -f "$CLI_BIN_PATH" ]; then
  echo "[*] Installing rustcooling-cli binary to /usr/local/bin..."
  install -m 755 "$CLI_BIN_PATH" /usr/local/bin/rustcooling-cli
fi

if [ "$INSTALL_CLI_DEFAULT" = true ] && [ -f "$CLI_BIN_PATH" ]; then
  echo "[*] Pointing /usr/local/bin/rustcooling symlink to CLI version..."
  ln -sf /usr/local/bin/rustcooling-cli /usr/local/bin/rustcooling
elif [ -f "$BIN_PATH" ]; then
  echo "[*] Pointing /usr/local/bin/rustcooling symlink to GUI version..."
  ln -sf /usr/local/bin/RustCooling /usr/local/bin/rustcooling
elif [ -f "$CLI_BIN_PATH" ]; then
  ln -sf /usr/local/bin/rustcooling-cli /usr/local/bin/rustcooling
fi

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

# Create Desktop shortcut for the invoking user
REAL_USER="${SUDO_USER:-$USER}"
if [ -n "$REAL_USER" ] && [ "$REAL_USER" != "root" ]; then
    USER_HOME=$(getent passwd "$REAL_USER" | cut -d: -f6)
    DESK_FOUND=false
    for DESK_NAME in "Desktop" "Рабочий стол"; do
        DESK_DIR="$USER_HOME/$DESK_NAME"
        if [ -d "$DESK_DIR" ]; then
            echo "[*] Creating Desktop shortcut in $DESK_DIR..."
            install -m 755 -o "$REAL_USER" "$DESKTOP_FILE" "$DESK_DIR/rustcooling.desktop"
            DESK_FOUND=true
            break
        fi
    done
    if [ "$DESK_FOUND" = false ] && [ -d "$USER_HOME" ]; then
        TARGET_DESK="$USER_HOME/Desktop"
        echo "[*] Creating $TARGET_DESK and Desktop shortcut..."
        install -d -o "$REAL_USER" "$TARGET_DESK"
        install -m 755 -o "$REAL_USER" "$DESKTOP_FILE" "$TARGET_DESK/rustcooling.desktop"
    fi
fi

if [ -f "$SERVICE_FILE" ]; then
    install -d /usr/lib/systemd/user
    install -m 644 "$SERVICE_FILE" /usr/lib/systemd/user/rustcooling.service
fi

echo "[*] Installing application fonts (Unbounded & Noto Sans SC)..."
FONTS_DIR="$SCRIPT_DIR/fonts"
[ -d "$FONTS_DIR" ] || FONTS_DIR="$REPO_ROOT/assets/fonts"
if [ -d "$FONTS_DIR" ]; then
    install -d /usr/share/fonts/truetype/rustcooling
    install -m 644 "$FONTS_DIR"/*.ttf /usr/share/fonts/truetype/rustcooling/ 2>/dev/null || true
    if command -v fc-cache >/dev/null 2>&1; then
        fc-cache -f /usr/share/fonts/truetype/rustcooling >/dev/null 2>&1 || true
    fi
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

echo "[✓] RustCooling installed successfully!"
echo "    - Launch GUI: 'rustcooling' or from application menu / desktop shortcut"
echo "    - Launch TUI: 'rustcooling --tui' (or 'rustcooling-cli')"
echo "    - Check status: 'rustcooling --status'"
echo "    - Background daemon: 'systemctl --user enable --now rustcooling'"