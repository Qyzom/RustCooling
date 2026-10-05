### What's New
- Added rich interactive Terminal User Interface (TUI) mode (`rustcooling tui` / `-t`) with live telemetry gauges, edge-to-edge history sparklines, and full settings editor.
- Ultra-low RAM footprint: TUI mode runs at ~4–7 MB RAM and headless daemon mode runs at ~2–4 MB RAM without loading GUI graphics drivers.
- Seamless background daemon handoff on TUI/GUI exit with SIGHUP handling to keep AIO pump screen continuously active.
- Full runtime localization (EN, RU, ZH, DE, FR) across both TUI and Slint GUI interfaces.
- Added `rustcooling status` CLI command for quick non-blocking status queries in scripts and system bars.
- Added `/usr/local/bin/rustcooling` system binary registration, udev rules, and systemd user service in `.deb`, `.tar.gz`, and installation scripts.

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling-1.0.3.exe`** — Versioned standalone executable for Windows 10/11 x64 (no installation required, ready to run).
- **`RustCooling-1.0.3.AppImage`** — Standalone universal AppImage for all x86_64 Linux distributions.
- **`RustCooling-1.0.3.deb`** — Debian, Ubuntu, and Linux Mint package with automatic udev rules configuration.
- **`RustCooling-1.0.3.tar.gz`** — Universal archive for Arch Linux, Fedora, and other distributions with `install.sh` script.
