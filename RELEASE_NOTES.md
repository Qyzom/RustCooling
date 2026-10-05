### What's New
- Added rich interactive Terminal User Interface (TUI) mode (`rustcooling --tui` / `-t`) built on `ratatui` with live telemetry gauges, history sparklines, and runtime settings editor.
- Ultra-low RAM footprint: TUI mode runs at ~4–7 MB RAM and headless daemon mode runs at ~2–4 MB RAM without loading GUI graphics drivers.
- Full runtime localization (EN, RU, ZH, DE, FR) supported across TUI and GUI modes.
- Added `rustcooling --status` CLI flag for quick non-blocking status queries in scripts, Waybar, Polybar, or Conky.
- Added `rustcooling.service` systemd user service unit for background daemon execution on Linux.
- Added `/usr/bin/rustcooling` (`/usr/local/bin/rustcooling`) binary symlink in `.deb`, `.tar.gz`, and Arch Linux packaging.

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling-1.0.3.exe`** — Versioned standalone executable for Windows 10/11 x64 (no installation required, ready to run).
- **`RustCooling-1.0.3.AppImage`** — Standalone universal AppImage for all x86_64 Linux distributions.
- **`RustCooling-1.0.3.deb`** — Debian, Ubuntu, and Linux Mint package with automatic udev rules and systemd service configuration.
- **`RustCooling-1.0.3.tar.gz`** — Universal archive for Arch Linux, Fedora, and other distributions with `install.sh` script.
