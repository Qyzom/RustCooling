# RustCooling

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![Slint](https://img.shields.io/badge/UI-Slint_1.9-blueviolet.svg)](https://slint.dev/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-brightgreen.svg)]()

Lightweight, cross-platform controller and telemetry daemon for **ID-COOLING FX Series** liquid cooler LCD displays (QinHeng WCH controller, VID `0x1A86`, PID `0xE317`). Written in Rust with a native Slint user interface.

---

## Features

- **Low Resource Usage:** ~10–14 MB RAM with window open; ~4–8 MB when minimized to system tray.
- **Single Monolithic Binary:** Telemetry monitoring, GUI, system tray, and USB HID driver bundled into a standalone executable.
- **Cross-Platform:** Full feature parity on Windows and Linux (temperature sources, autostart, system tray, settings persistence).
- **Multiple Display Modes:** CPU Temperature, Clock Frequency, Utilization Percentage, or Carousel mode.
- **Configurable Transitions:** Direct instantaneous update, Roller, or Smooth animation styles.
- **Internationalization:** Embedded multi-language support (English, Russian, Chinese) switchable at runtime.
- **Daemon Mode:** Headless CLI mode for background execution (`--daemon`).

---

## Operating Principles & Architecture

### 1. USB HID Protocol
The display communicates via standard USB HID reports. Every command packet consists of a **65-byte buffer** (1-byte `0x00` Report ID followed by a 64-byte payload):

| Offset | Field | Value / Description |
| :---: | :--- | :--- |
| `[0]` | Report ID | `0x00` (required by HIDAPI on Windows and Linux) |
| `[1]` | Header 1 | `0x55` |
| `[2]` | Header 2 | `0xBB` |
| `[3]` | Data Length | `0x02` (2 bytes of value payload) |
| `[4]` | Command ID | `0x01` (Temp °C), `0x02` (Clock GHz/10), `0x03` (Load %), `0x04` (Show 1/0) |
| `[5]` | Value High | `(value >> 8) & 0xFF` (Big-Endian) |
| `[6]` | Value Low | `value & 0xFF` |
| `[7]` | Checksum | `(byte[1] + byte[2] + ... + byte[6]) & 0xFF` (modulo 256 sum) |
| `[8..64]` | Padding | 57 bytes of zeroes (`0x00`) |

### 2. Telemetry Acquisition
- **Windows:**
  - **Load & Clock:** Retrieved via `sysinfo` and WMI performance counters (`Win32_PerfFormattedData_Counters_ProcessorInformation`).
  - **Temperature:** Physical hardware sensor readout from `sysinfo::Components` (`Package`, `Tctl`, `Core #`). If blocked by Windows driver permissions, falls back to dynamic thermal calculation coupled with CPU frequency boost and utilization.
- **Linux:**
  - **Temperature:** Direct sysfs parsing of `/sys/class/hwmon/` (`coretemp`, `k10temp`, `zenpower`, etc.) supporting selectable sources (`package`, `core0`, `avg`, `max`), with fallback to `/sys/class/thermal/`.
  - **Load:** Jiffy-delta accounting from `/proc/stat` across updates.
  - **Clock:** Dynamic boosted core frequency from `/sys/devices/system/cpu/cpufreq/` or `/proc/cpuinfo`.

### 3. Background Service Lifecycle
The `MonitorService` runs on a dedicated background thread:
1. Connects to the HID device (`0x1A86:0xE317` by default; custom VID/PID supported).
2. Sends `CMD_SHOW(1)` to turn on the screen.
3. Periodically samples system telemetry, applies stepping animations if configured, and writes reports to the display.
4. On application exit or SIGINT, sends `CMD_SHOW(0)` and cleanly joins worker threads before shutdown.

### 4. Configuration & Autostart
- **Config Storage:** Stored as JSON in standard XDG / OS directories:
  - Linux: `~/.config/RustCooling/config.json`
  - Windows: `%APPDATA%\RustCooling\config.json`
- **Autostart:**
  - Linux: Creates/removes `~/.config/autostart/rust-cooling.desktop` complying with the FreeDesktop Autostart specification.
  - Windows: Configured via standard registry Run keys with `--minimized` flag.

---

## Project Structure

```
RustCooling/
├── assets/
│   └── fonts/              # Embedded Unbounded font family (SIL OFL 1.1)
├── i18n/                   # Translation dictionaries
│   ├── en.json             # English
│   ├── ru.json             # Russian
│   └── zh.json             # Chinese
├── ui/
│   └── app.slint           # Slint UI layout and components
└── src/
    ├── config/             # Persistent JSON application configuration
    ├── hid/                # Cross-platform USB HID connection manager
    ├── i18n/               # Embedded localization provider
    ├── protocol/           # Packet framing and checksum verification
    ├── service/            # Telemetry dispatching and animation engine
    ├── telemetry/          # Platform-specific metric readers (Linux / Windows)
    ├── tray/               # System tray icon and context menu
    └── main.rs             # Application entry point and UI event bindings
```

---

## Linux Setup & Prerequisites

### 1. Build Dependencies
To compile RustCooling on Linux, install the required development packages:

- **Debian / Ubuntu / Linux Mint:**
  ```bash
  sudo apt update
  sudo apt install -y pkg-config libudev-dev libgtk-3-dev libayatana-appindicator3-dev
  ```

- **Arch Linux / Manjaro:**
  ```bash
  sudo pacman -S --needed pkgconf systemd gtk3 libayatana-appindicator
  ```

- **Fedora / RHEL:**
  ```bash
  sudo dnf install -y pkgconf-pkg-config systemd-devel gtk3-devel libayatana-appindicator-gtk3-devel
  ```

### 2. USB Permissions (udev Rule)
By default, Linux limits raw access to USB HID devices (`/dev/hidraw*`) to root. To allow RustCooling to access the LCD display without `sudo`:

```bash
echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="e317", MODE="0666", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/99-rustcooling.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

---

## Building from Source

Ensure you have a recent [Rust toolchain](https://rustup.rs/) installed (edition 2021, Rust 1.80+ recommended).

```bash
# Clone the repository
git clone https://github.com/Qyzom/RustCooling.git
cd RustCooling

# Run automated tests
cargo test

# Build release binary
cargo build --release
```

The resulting executable will be located at:
- **Linux:** `target/release/rust-cooling`
- **Windows:** `target/release/rust-cooling.exe`

---

## CLI Options

```bash
# Launch GUI
./rust-cooling

# Launch minimized to system tray
./rust-cooling --minimized

# Run as headless background daemon (without GUI)
./rust-cooling --daemon

# Override polling interval (in milliseconds)
./rust-cooling --interval 500
```

---

## License & Compliance

- **Software:** [MIT License](LICENSE).
- **Typeface:** The embedded *Unbounded* typeface is distributed under the [SIL Open Font License 1.1](assets/fonts/OFL.txt).
- **Libraries:** Built with open-source dependencies complying with MIT, Apache-2.0, and Slint Royalty-Free Desktop terms.
