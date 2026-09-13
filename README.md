# RustCooling

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![Slint](https://img.shields.io/badge/UI-Slint_1.9-blueviolet.svg)](https://slint.dev/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-brightgreen.svg)]()

> Lightweight, cross-platform controller and telemetry daemon for **ID-COOLING FX Series** liquid cooler LCD displays. Built in Rust with Material Design 3 and native Slint GUI.

---

## Highlights

- **Ultra-low Memory Footprint:** Consumes only ~**10–14 MB** of RAM with active GUI and ~**4–8 MB** when minimized to system tray (compared to 180–300 MB in official Electron-based software).
- **Single Monolithic Binary:** No separate background daemons, drivers, or auxiliary processes. Everything runs in one native executable.
- **Material Design 3 Interface:** Clean, dark Material You aesthetic featuring the **Unbounded** typeface, live sparkline trend graphs, and active broadcast status.
- **Native Contextual Localization:** Multi-language architecture with separate JSON translation files (English, Russian, Chinese supported out-of-the-box).
- **Rock-solid Hardware Bridge:** Communicates directly with the QinHeng WCH controller (`VID 0x1A86, PID 0xE317`) over USB HID with automatic reconnects and graceful shutdown.
- **Staggered Frame Dispatch:** Smooth 100ms multi-stage packet scheduling and deduplication to prevent screen microcontroller buffer overruns.

---

## Architecture Overview

```
rust-cooling/
├── assets/
│   └── fonts/              # Unbounded font family (OFL-1.1 licensed)
├── i18n/                   # Localized translation files
│   ├── en.json             # English (Default)
│   ├── ru.json             # Russian
│   └── zh.json             # Chinese
├── ui/
│   └── app.slint           # Material 3 UI layout & sparkline components
└── src/
    ├── config/             # Persistent JSON application configuration
    ├── hid/                # USB HID device connector with auto-reconnect
    ├── i18n/               # Embedded translation engine
    ├── protocol/           # 64-byte frame packing and checksum calculation
    ├── service/            # Staggered telemetry dispatcher loop
    ├── telemetry/          # Native Windows (WMI/Sysinfo) & Linux (sysfs/hwmon)
    ├── tray/               # Cross-platform system tray integration
    └── main.rs             # Application entry point & lifecycle manager
```

---

## Protocol Specification

Each communication frame consists of a 64-byte payload (with a 0x00 Report ID prefix on Windows):

| Byte Offset | Field | Description |
| :---: | :--- | :--- |
| `[0]` | Header 1 | `0x55` |
| `[1]` | Header 2 | `0xBB` |
| `[2]` | Data Length | `0x02` (2 bytes payload) |
| `[3]` | Command ID | `0x01` (Temp), `0x02` (Clock), `0x03` (Load), `0x04` (Show) |
| `[4]` | Value High | `(value >> 8) & 0xFF` (Big-Endian) |
| `[5]` | Value Low | `value & 0xFF` |
| `[6]` | Checksum | `(byte[0] + ... + byte[5]) & 0xFF` |
| `[7..63]` | Padding | 57 bytes of zeros (`0x00`) |

---

## Building from Source

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (version 1.80 or newer recommended).

### Build

```bash
# Clone the repository
git clone https://github.com/Qyzom/RustCooling.git
cd RustCooling

# Run unit tests
cargo test

# Build optimized release binary
cargo build --release
```

The resulting standalone executable will be located in `target/release/rust-cooling`.

### Running

```bash
# Normal desktop mode (shows Material 3 GUI)
cargo run --release

# Start minimized directly to system tray
cargo run --release -- --minimized

# Run as headless daemon without GUI (e.g. systemd or background services)
cargo run --release -- --daemon
```

---

## License

This project is licensed under the [MIT License](LICENSE).
Fonts included in `assets/fonts/` are licensed under the [SIL Open Font License 1.1](assets/fonts/OFL.txt).
