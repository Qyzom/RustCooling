# RustCooling

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![Slint](https://img.shields.io/badge/UI-Slint_1.9-blueviolet.svg)](https://slint.dev/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-brightgreen.svg)]()
[![Release](https://img.shields.io/github/v/release/Qyzom/RustCooling?color=teal)](https://github.com/Qyzom/RustCooling/releases)

**Ultra-lightweight, cross-platform LCD controller and telemetry daemon for ID-COOLING FX Series liquid coolers.**  
*Native Linux and Windows support with zero background bloat, hardware-accelerated UI, and < 15 MB RAM usage.*

</div>

---

## Overview & Origin

**RustCooling** is an open-source, high-performance controller for the LCD pump-cap displays found on **ID-COOLING FX Series** All-in-One liquid coolers (FX240, FX280, FX360, and compatible units based on QinHeng Electronics / WCH USB controllers, VID `0x1A86`, PID `0xE317`).

This project is a complete, ground-up rewrite in **100% Rust** of the author's very first project, [**idc-lite**](https://github.com/Qyzom/idc-lite). While `idc-lite` served as a functional alternative to vendor software, it relied on C# / .NET runtimes, had heavy memory usage, and Linux support was experimental and cumbersome. 

**RustCooling** solves all of these compromises: it delivers a native, monolithic binary that runs on both Linux and Windows with zero dependencies, rock-solid hardware telemetry, and an ultra-lean memory footprint.

---

## Interface & Screenshots

<div align="center">

<img src="images/1.png" alt="RustCooling Main Screen" width="300"/>

<br/><br/>

<img src="images/2.png" width="235"/>
<img src="images/3.png" width="235"/>
<img src="images/4.png" width="235"/>

</div>

---

## Architectural Comparison

| Feature | Original Vendor Software | [idc-lite](https://github.com/Qyzom/idc-lite) (1st Project) | **RustCooling** (Current) |
| :--- | :--- | :--- | :--- |
| **Language / Stack** | Electron / Node.js + C++ | C# / .NET 8 + WPF / Tauri | **100% Pure Rust** + Slint UI |
| **RAM (GUI Open)** | ~150 – 300 MB | ~60 – 120 MB | **< 12 MB** (~10.8 MB in Task Manager) |
| **RAM (System Tray)** | ~80 – 150 MB (background bloat) | ~35 – 60 MB | **< 3 MB** (~1.1 MB in Task Manager) |
| **RAM (Headless Daemon)** | ❌ No daemon | ⚠️ Separate process (~20 MB) | **~2 – 4 MB** (`--daemon` mode) |
| **CPU Usage** | 2.0% – 5.0% continuous | ~1.0% | **0.0%** (Ultra-lean event-driven loop) |
| **Linux Support** | ❌ None (Windows only) | ⚠️ Experimental / partial | **Native (hwmon, sysfs, udev)** |
| **Startup Time** | ~3.0 – 6.0 seconds | ~1.5 – 3.0 seconds | **< 50 milliseconds** |
| **Proprietary Bloat** | High (telemetry, auto-updaters) | Moderate (.NET runtime overhead) | **Zero (100% open-source & clean)** |
| **UI Engine** | Chromium WebEngine | WebView2 / WPF | **FemtoVG (Native OpenGL)** |
| **Localization** | EN, ZH | EN, RU, ZH | **EN, RU, ZH, DE, FR** |
| **Headless Daemon** | ❌ No | ⚠️ Separate daemon binary | **Built-in (`--daemon` flag)** |

---

## System Architecture

### 1. Linux Telemetry & Subsystems
On Linux, RustCooling interacts directly with the Linux kernel without requiring third-party libraries, Wine, or root privileges:
- **CPU Temperature:** Directly queries the kernel sysfs interface via `/sys/class/hwmon/hwmon*` (supporting AMD `k10temp`/`zenpower`, Intel `coretemp`, and ACPI `acpitz`). Supports configurable thermal sources:
  - `Package / Tctl` (recommended for modern Ryzen and Core processors)
  - `Core 0`
  - `Average Cores`
  - `Max Core`
  - Automatic fallback to `/sys/class/thermal/thermal_zone*/temp`.
- **CPU Clock Frequency:** Parses active boosted frequencies directly from `/sys/devices/system/cpu/cpufreq/policy*/scaling_cur_freq` and `/proc/cpuinfo`.
- **CPU Load:** Performs non-blocking differential jiffy accounting from `/proc/stat` across polling intervals (`user`, `nice`, `system`, `idle`, `iowait`, `irq`, `softirq`, `steal`).
- **Permissions (udev):** Includes a standalone udev rule (`99-idcooling.rules`) assigning `0666` permissions and `uaccess` to VID `1A86`, PID `E317`, enabling full unprivileged access.
- **Autostart:** Standard FreeDesktop / XDG autostart specification via `~/.config/autostart/RustCooling.desktop`.

### 2. Windows Telemetry & Subsystems
On Windows, RustCooling provides clean, deterministic telemetry without the memory leaks that plague vendor software:
- **Kernel Telemetry:** Combines native OS performance counters and `sysinfo` for CPU clock and utilization.
- **Leak-Free Pipeline:** Eliminates COM / WMI initialization loops and GDI object leaks, guaranteeing completely flat memory consumption over weeks of continuous operation.
- **System Tray Optimization:** When minimized to the notification area, the process invokes `EmptyWorkingSet`, releasing unused physical RAM pages back to the Windows memory manager (< 3 MB RAM footprint).
- **Windows Autostart:** Seamless integration via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

### 3. USB HID Protocol & Display Timing
The ID-COOLING FX Series LCD pump cap communicates via standard USB HID reports:
- **VID:** `0x1A86` (QinHeng Electronics / WCH) | **PID:** `0xE317`
- **Frame Structure:** 64 bytes (65 bytes with Windows Report ID `0x00`):
  - Byte `0`: Header `0x55`
  - Byte `1`: Header `0xBB`
  - Byte `2`: Payload Length (`0x02`)
  - Byte `3`: Command ID (`0x01` Temperature, `0x02` Frequency, `0x03` Utilization, `0x04` Display State)
  - Byte `4`: Value High Byte (Big-Endian `(value >> 8) & 0xFF`)
  - Byte `5`: Value Low Byte (`value & 0xFF`)
  - Byte `6`: Checksum (`(byte[0] + ... + byte[5]) & 0xFF`)
  - Bytes `7..63`: Padding (zeroes `0x00`)
- **Staggered Frame Dispatch:** Command frames are dispatched with a **100 ms cadence** between temperature, frequency, and usage. This ensures the onboard WCH microcontroller processes and refreshes the LCD panel cleanly without dropping packets.
- **Value Deduplication:** Redundant USB HID transfers are skipped if telemetry values have not changed between polling cycles, minimizing USB bus activity.

---

## Project Longevity & Maintenance Status

> [!IMPORTANT]
> **If you notice that the last commit was several months or even a year ago, do NOT assume this project is abandoned!**
> 
> - **Protocol Stability:** The ID-COOLING FX series LCD hardware protocol has been fully reverse-engineered, tested, and finalized. ID-COOLING does not change the hardware firmware or USB protocol for existing coolers.
> - **Zero Known Bugs:** Memory leaks, handle accumulation, and button freeze edge-cases have been systematically identified and resolved. The application is feature-complete and rock-solid.
> - **Daily Driver:** The author uses **RustCooling daily on their personal computer** (this software was built first and foremost for personal everyday use). If any edge cases or OS updates ever require attention, fixes will be released immediately.

---

## Installation & Downloads

Pre-built standalone releases are available on the [**Releases page**](https://github.com/Qyzom/RustCooling/releases):

### Windows
1. Download **`RustCooling-0.1.2.exe`**.
2. Run the executable. It is completely portable — no installer or runtime dependencies required.
3. Open **Settings** within the UI to toggle **Autostart on Boot**.

### Linux (Debian / Ubuntu / Linux Mint)
1. Download **`RustCooling-0.1.2.deb`**.
2. Install the package:
   ```bash
   sudo dpkg -i RustCooling-0.1.2.deb
   sudo apt-get install -f  # resolves dependencies if needed
   ```
3. Launch `RustCooling` from your desktop application launcher or run `RustCooling` from the terminal.

### Linux (Arch Linux / Fedora / Generic Tarball)
1. Download **`RustCooling-0.1.2.tar.gz`**.
2. Extract and run the installer:
   ```bash
   tar -xzf RustCooling-0.1.2.tar.gz
   cd RustCooling-0.1.2
   sudo ./install.sh
   ```

---

## Headless Daemon Mode (CLI / systemd)

For home servers, headless workstations, or minimal Linux desktop configurations (e.g. Hyprland, Sway, i3) where a graphical window is not desired:

```bash
# Run in background daemon mode (no GUI window or tray)
RustCooling --daemon

# Specify a custom update interval (e.g. 500 ms)
RustCooling --daemon --interval 500
```

### systemd Service Example (`~/.config/systemd/user/rustcooling.service`):
```ini
[Unit]
Description=RustCooling ID-COOLING LCD Daemon
After=default.target

[Service]
ExecStart=/usr/local/bin/RustCooling --daemon
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
```

Enable and start the service:
```bash
systemctl --user daemon-reload
systemctl --user enable --now rustcooling.service
```

---

## Building from Source

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (2021 Edition, 1.80+)
- **Linux:** `pkg-config`, `libudev-dev`, `libfontconfig1-dev`, `libgl1-mesa-dev`, `libayatana-appindicator3-dev`

```bash
# Clone the repository
git clone https://github.com/Qyzom/RustCooling.git
cd RustCooling

# Run test suite
cargo test

# Build optimized release binary
cargo build --release
```

The resulting binary will be located in `target/release/RustCooling` (or `RustCooling.exe` on Windows).

---

## Tech Stack & Dependencies

- **Language:** 100% [Rust](https://www.rust-lang.org/) (2021 Edition)
- **GUI Toolkit:** [Slint UI 1.9](https://slint.dev/) (Hardware accelerated FemtoVG OpenGL backend)
- **USB HID:** [hidapi-rs 2.6](https://crates.io/crates/hidapi)
- **Telemetry:** [sysinfo 0.33](https://crates.io/crates/sysinfo) + custom Linux `hwmon`/`procfs` engines
- **System Tray:** [tray-icon](https://crates.io/crates/tray-icon) & [muda](https://crates.io/crates/muda) (Native Win32 & AppIndicator3 with dark theme)
- **Autostart:** [auto-launch](https://crates.io/crates/auto-launch) (Windows Registry & Linux XDG desktop)

---

## License

- **Software:** Distributed under the [MIT License](LICENSE).
- **Typography:** The embedded *Unbounded* typeface is licensed under the [SIL Open Font License 1.1](assets/fonts/OFL.txt).
- **Predecessor:** Inspired by lessons learned from [idc-lite](https://github.com/Qyzom/idc-lite).