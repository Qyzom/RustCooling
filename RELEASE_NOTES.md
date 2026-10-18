### What's New
- Added dedicated standalone CLI binary (`rustcooling-cli`) that opens interactive TUI directly without requiring `-t`.
- Enforced native Winit + FemtoVG rendering on Linux to properly display embedded `Unbounded` (Latin/Cyrillic) and `Noto Sans SC` (Chinese) typography without system font fallbacks.
- Enhanced system tray integration with instant window restoration and hide-on-close behavior across both Windows and Linux.
- Added automatic desktop shortcut generation on Linux systems upon package installation.
- Windows release artifacts streamlined strictly to portable standalone executable (`RustCooling.exe`).

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling-1.0.4.exe`** — Versioned standalone executable for Windows 10/11 x64 (no installation required, ready to run).
- **`RustCooling-1.0.4.AppImage`** — Standalone universal AppImage for all x86_64 Linux distributions.
- **`RustCooling-1.0.4.deb`** — Debian, Ubuntu, and Linux Mint package with automatic udev rules configuration.
- **`RustCooling-1.0.4.tar.gz`** — Universal archive for Arch Linux, Fedora, and other distributions with `install.sh` script.
