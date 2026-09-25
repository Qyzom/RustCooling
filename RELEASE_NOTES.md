### What's New
- Native Arch Linux packaging: fixed missing dependencies, eliminated build conflicts with GCC LTO (`options=('!lto')`), and added automated `pacman` build support (`makepkg -si`).
- Fixed system tray on Linux: added explicit GTK3 initialization and event loop pumping in the UI timer, restoring tray menus and minimize/restore actions on Wayland and X11.
- Native Wayland window dragging: replaced Win32 `GetCursorPos` delta math with Winit's native `drag_window()`, enabling flawless window movement in Hyprland, Sway, and other Wayland compositors.
- UI cleanup on Linux: hid the Windows-only WinRing0 kernel driver settings card when running under Linux.
- Linux memory optimization: added `malloc_trim(0)` when hiding to system tray to release freed heap pages back to the kernel.

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling.exe`** — Portable standalone executable for Windows 10/11 x64 (no installation required, ready to run).
- **`RustCooling-1.0.1.exe`** — Versioned standalone executable for Windows 10/11 x64.
- **`RustCooling-1.0.1.AppImage`** — Standalone universal AppImage for all x86_64 Linux distributions.
- **`RustCooling-1.0.1.deb`** — Debian, Ubuntu, and Linux Mint package with automatic udev rules configuration.
- **`RustCooling-1.0.1.tar.gz`** — Universal archive for Arch Linux, Fedora, and other distributions with `install.sh` script.
