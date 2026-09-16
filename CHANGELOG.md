# Changelog

All notable changes to the RustCooling project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.3] — 2026-09-16

### Added
- **Direct Physical CPU Sensor Reading via Ring 0:** Integrated LibreHardwareMonitor's proven kernel driver (`WinRing0x64.sys`) into Windows builds, reading real hardware Digital Thermal Sensor (DTS) metrics directly from physical MSR registers (Intel TjMax/DTS, AMD Tctl).
- **First-Run Setup & Activation Screen:** Added a first-run setup wizard allowing one-click UAC administrator permission elevation to register and start the background kernel driver service.
- **Embedded Noto Sans SC Font for Chinese:** Embedded Google Noto Sans SC (Bold) typeface to ensure flawless rendering of Simplified Chinese glyphs without tofu artifacts or clipping.
- **True Portable Storage:** Settings file `config.json` and kernel driver `WinRing0x64.sys` are now stored directly in the folder alongside `RustCooling.exe`, leaving `%APPDATA%` completely clean.

### Fixed
- **Main Window Geometry & Layout:** Window height increased to 360 px with optimized vertical paddings and spacing, eliminating layout clipping and button overlap under Chinese typography.
- **Concise Driver Status Button:** Renamed the driver action button to a concise "Update" across all 5 languages (EN, RU, DE, FR, ZH).
- **ARM Architectural Clarification:** Documented that ARM desktop AIO liquid coolers do not physically exist, clarifying why ARM builds are intentionally omitted.

---

## [0.1.2] — 2026-09-14

### Added
- **German and French Localizations:** Full UI and tray menu translation into German (`de`) and French (`fr`), supporting 5 languages in total.
- **Automated Startup Memory Trimming:** Post-startup working set memory is trimmed via Win32 `EmptyWorkingSet`, dropping RAM usage to < 12 MB right after launch.
- **Embedded Multi-Resolution Icons:** Embedded multi-layer DIB and PNG icon assets into executable PE resources for Explorer and Task Manager.

### Changed
- **Clean Process Identification:** `FileDescription` in PE resources is strictly set to `RustCooling` without suffix clutter.
- **Optimized Transition Animation:** Replaced redundant interpolation with an efficient 50 ms step roller animation.
- **Synchronized System Autostart:** Settings toggle accurately reflects the real Windows Registry and Linux `.desktop` autostart state.

---

## [0.1.1] — 2026-09-14

### Added
- **Instant System Tray Event Handling:** Re-engineered tray event handling with native Win32/X11 message hooks dispatched directly through Slint's event loop (0 ms latency).
- **Guaranteed Window Focus & Restoration:** Opening from system tray calls `SW_RESTORE`, `SetForegroundWindow`, and `BringWindowToTop`.
- **UI Gallery:** Added real application screenshots directly into the documentation.

### Fixed
- **Mouse Pointer Grab Deadlock:** Resolved cursor freeze and pointer release issues when minimizing/restoring from tray.
- **Codebase Cleanup:** Audited code and eliminated unused callbacks and legacy rudiments.

---

## [0.1.0] — 2026-09-13

### Added
- **Initial Release:** Complete rewrite of the ID-COOLING FX Series LCD liquid cooler controller in 100% pure Rust.
- **Monolithic Single Binary:** Self-contained application (< 15 MB RAM active, < 5 MB in tray) with zero external runtime dependencies.
- **Zero-Compromise Linux Support:** Direct hardware monitoring via `/sys/class/hwmon` and `sysfs` without Wine or root privileges.
- **Hardware-Accelerated UI:** High-performance Slint interface with FemtoVG OpenGL rendering.
- **Hardware Compatibility:** Full support for ID-COOLING FX Series coolers (FX240, FX280, FX360; USB VID `0x1A86`, PID `0xE317`).
- **Headless Daemon Mode:** Built-in `--daemon` CLI flag for server and window manager environments.
