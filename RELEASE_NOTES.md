### What's New
- Dedicated standalone CLI/TUI binary (`rustcooling-cli`) defaulting directly to interactive terminal dashboard without requiring `-t`.
- Distinct release packages for Linux: dedicated GUI packages (`RustCooling-GUI`) and minimal zero-dependency CLI/TUI packages (`RustCooling-CLI`).
- Completely isolated configuration directories (`RustCooling-gui` and `RustCooling-tui`) for clean separation of GUI and TUI states.
- Cleaned up TUI settings to remove system tray and complex autostart toggles, keeping an effortless daemon autostart toggle.
- Visual interface labels updated to explicitly distinguish version flavors: `RustCooling v1.0.4 (GUI)` and `RustCooling v1.0.4 (TUI)`.
- Static `Unbounded` Bold, ExtraBold, and Black font variants bundled and imported for crisp typography on Linux and Windows.
- Automatic application shortcut creation on user Desktop upon Linux GUI package installation.

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling-1.0.4.exe`** — Versioned standalone GUI executable for Windows 10/11 x64 (no installation required, ready to run).
- **`RustCooling-GUI-1.0.4.AppImage`** — Standalone universal GUI AppImage for all x86_64 Linux distributions.
- **`RustCooling-GUI-1.0.4.deb`** — Debian, Ubuntu, and Linux Mint GUI package with desktop integration, system tray, and font support.
- **`RustCooling-CLI-1.0.4.deb`** — Minimal headless Debian, Ubuntu, and Linux Mint package with interactive TUI and zero GUI dependencies.
- **`RustCooling-GUI-1.0.4.tar.gz`** — Universal Linux GUI archive with desktop integration, fonts, and `install.sh` script.
- **`RustCooling-CLI-1.0.4.tar.gz`** — Universal Linux TUI & CLI archive for servers and minimal terminal installations.
