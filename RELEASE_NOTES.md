### What's New
- Restored smooth roller transition animation: temperature and load step degree-by-degree at 50 ms intervals with accurate polling compensation.
- Optimized Linux sysfs hwmon scanning: isolated CPU sensors to prevent NVMe/WiFi drives from contaminating core average/max telemetry.
- Streamlined Linux `/proc/stat` CPU load reader using line-buffered streams to reduce memory allocations and latency.
- Added application icon metadata to Linux autostart desktop entry for desktop environment settings.
- CI/CD release workflow optimization: removed duplicate unversioned executable build and improved version tag extraction.

[Full Changelog](https://github.com/Qyzom/RustCooling/commits/main)

### Release Assets
- **`RustCooling-1.0.2.exe`** — Versioned standalone executable for Windows 10/11 x64 (no installation required, ready to run).
- **`RustCooling-1.0.2.AppImage`** — Standalone universal AppImage for all x86_64 Linux distributions.
- **`RustCooling-1.0.2.deb`** — Debian, Ubuntu, and Linux Mint package with automatic udev rules configuration.
- **`RustCooling-1.0.2.tar.gz`** — Universal archive for Arch Linux, Fedora, and other distributions with `install.sh` script.
