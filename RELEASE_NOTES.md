### What's New in 1.0.0
- **Zero-Bloat Memory Footprint:** Completely refactored Linux backend architecture, eliminated GTK3 runtime overhead, and deferred OpenGL/GPU surface initialization when starting minimized to tray (~20–25 MB RSS in tray, ~2 MB daemon).
- **Interactive TUI Mode:** Added standalone Ratatui terminal dashboard (`rustcooling --tui`) with live sparkline telemetry graphs and interactive settings.
- **Headless Daemon & Systemd Service:** Native background daemon (`rustcooling --daemon`) and user systemd service integration.
- **New Packaging Formats:** Official Debian (`.deb`) and Red Hat / Fedora (`.rpm`) packages alongside portable AppImages and AUR PKGBUILD.
- **Telemetry Optimization:** Cached Linux `hwmon` sensor paths to eliminate redundant disk scanning, and unified high-precision hardware queries.
- **Updated UI & Localizations:** Full localization support for English, Russian, Simplified Chinese, German, and French.

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling-1.0.0.exe`** — Standalone GUI executable for Windows 10/11 x64.
- **`RustCooling-GUI-1.0.0.AppImage`** — Standalone GUI AppImage for Linux x86_64.
- **`RustCooling-TUI-1.0.0.AppImage`** — Standalone terminal TUI & CLI AppImage for Linux x86_64.
- **`RustCooling-GUI-1.0.0.deb`** — Native Debian / Ubuntu GUI package.
- **`RustCooling-TUI-1.0.0.deb`** — Native Debian / Ubuntu TUI/CLI package.
- **`RustCooling-GUI-1.0.0.rpm`** — Native Fedora / RHEL GUI package.
- **`RustCooling-TUI-1.0.0.rpm`** — Native Fedora / RHEL TUI/CLI package.
