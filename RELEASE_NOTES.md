### What's New
- Dedicated standalone CLI/TUI binary (`rustcooling-cli`) defaulting directly to interactive terminal dashboard without requiring `-t`.
- Completely isolated configuration directories (`RustCooling-gui` and `RustCooling-tui`) for clean separation of GUI and TUI states.
- Removed GUI-specific vestiges (system tray toggle, complex autostart modes) from TUI: simplified to clean On/Off autostart for the background daemon.
- Visual interface labels updated to explicitly distinguish version flavors: `RustCooling v1.0.4 (GUI)` and `RustCooling v1.0.4 (TUI)`.
- Bundled and imported static `Unbounded` Bold, ExtraBold, and Black font instances for crisp and heavy typography on Linux and Windows.
- Automatic application shortcut creation on user Desktop upon Linux package installation.

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling-1.0.4.exe`** — Versioned standalone executable for Windows 10/11 x64 (no installation required, ready to run).
- **`RustCooling-1.0.4.AppImage`** — Standalone universal AppImage for all x86_64 Linux distributions.
- **`RustCooling-1.0.4.deb`** — Debian, Ubuntu, and Linux Mint package with automatic udev rules configuration.
- **`RustCooling-1.0.4.tar.gz`** — Universal archive for Arch Linux, Fedora, and other distributions with `install.sh` script.
