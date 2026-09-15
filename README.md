<div align="center">

<img src="logo.png" alt="RustCooling Logo" width="100" />

# RustCooling

**High-performance, ultra-lean LCD pump display controller for ID-COOLING FX Series liquid coolers.**  
*100% Pure Rust • Native Linux & Windows • Zero Bloat • < 12 MB RAM*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![Slint](https://img.shields.io/badge/UI-Slint_1.9-blueviolet.svg)](https://slint.dev/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-brightgreen.svg)]()
[![Release](https://img.shields.io/github/v/release/Qyzom/RustCooling?color=teal)](https://github.com/Qyzom/RustCooling/releases)

<br/>

<img src="images/1.png" alt="RustCooling Main Screen" width="280"/>

<br/><br/>

<img src="images/2.png" width="220"/>
<img src="images/3.png" width="220"/>
<img src="images/4.png" width="220"/>

</div>

---

### Key Features

| Extreme Efficiency | Native Linux | Total Control | Pure & Portable |
| :--- | :--- | :--- | :--- |
| • **< 12 MB** RAM active GUI<br/>• **< 3 MB** in system tray<br/>• **0.0%** idle CPU usage<br/>• **< 50 ms** instant startup | • Direct `/sys/class/hwmon`<br/>• AMD Tctl & Intel Package<br/>• Zero root/Wine required<br/>• Bundled udev permissions | • CPU Temp & CPU Load<br/>• 50 ms roller transition<br/>• Native dark tray menu<br/>• **5 Languages** (EN, RU, ZH, DE, FR) | • Single monolithic binary<br/>• No .NET or Electron runtime<br/>• Built-in `--daemon` mode<br/>• Zero telemetry or bloat |

---

### Benchmark & Comparison

> [!NOTE]
> I created RustCooling as a complete rewrite in **100% Rust** of my previous C# / .NET project, [**idc-lite**](https://github.com/Qyzom/idc-lite). My goal was to completely eliminate runtime dependencies, slash memory usage below 12 MB, and provide first-class native Linux support.

| Feature | Official Vendor App | idc-lite (My Previous C# App) | **RustCooling (Current)** |
| :--- | :---: | :---: | :---: |
| **Technology Stack** | Electron / Node.js + C++ | C# / .NET 8 + WPF | **100% Pure Rust + Slint UI** |
| **RAM (Active GUI)** | ~150 – 300 MB | ~60 – 120 MB | **< 12 MB** (~10.8 MB) |
| **RAM (System Tray)** | ~80 – 150 MB | ~35 – 60 MB | **< 3 MB** (~1.1 MB) |
| **RAM (Headless Daemon)** | Not available | Separate process (~20 MB) | **~2 – 4 MB** (`--daemon`) |
| **CPU Utilization** | 2.0% – 5.0% continuous | ~1.0% | **0.0%** (Event-driven) |
| **Linux Support** | None (Windows only) | Experimental | **Native (hwmon / sysfs / udev)** |
| **Startup Time** | 3.0 – 6.0 sec | 1.5 – 3.0 sec | **< 50 ms** |
| **Runtime Dependencies** | Chromium WebEngine | .NET Runtime | **Zero (Native machine code)** |
| **Interface Languages** | EN, ZH | EN, RU, ZH | **EN, RU, ZH, DE, FR** |
| **Headless Daemon** | No | Separate binary | **Built-in (`--daemon` flag)** |

---

### Downloads & Quick Start

Pre-built releases are available on the [**Releases Page**](https://github.com/Qyzom/RustCooling/releases/latest).

#### Windows (Portable)
1. Download **[`RustCooling-0.1.2.exe`](https://github.com/Qyzom/RustCooling/releases/latest)**.
2. Run the executable — completely portable, zero installer or runtime dependencies needed.
3. *(Optional)* Toggle **"Launch at Startup"** in the Settings panel.

#### Linux (Debian / Ubuntu / Linux Mint)
```bash
sudo dpkg -i RustCooling-0.1.2.deb
```
*Desktop entry and udev rules (`0666` for VID `1A86`, PID `E317`) are installed automatically.*

#### Linux (Arch / Fedora / Generic Tarball)
```bash
tar -xzf RustCooling-0.1.2.tar.gz
cd RustCooling-0.1.2
sudo ./install.sh
```

#### Headless Daemon Mode (CLI / systemd)
For minimal window managers (Hyprland, Sway, i3) or home servers:
```bash
# Run headless background telemetry
RustCooling --daemon

# Specify custom update interval in ms (default: 300)
RustCooling --daemon --interval 500
```

<details>
<summary><b>Click to view systemd user service setup</b></summary>

Create `~/.config/systemd/user/rustcooling.service`:
```ini
[Unit]
Description=RustCooling LCD Daemon
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
</details>

---

<details>
<summary><b>Hardware Architecture & Protocol Specification</b></summary>

<br/>

#### Hardware Identification (USB HID)
- **Target Hardware:** ID-COOLING FX Series AIO coolers (FX240, FX280, FX360) and compatible pumps based on QinHeng Electronics (WCH) USB HID microcontrollers.
- **Vendor ID (VID):** `0x1A86` (QinHeng Electronics / WCH)
- **Product ID (PID):** `0xE317`
- **Interface:** USB HID Class, Endpoint 1 (Interrupt OUT).
- **Report Length:** 64 bytes (65 bytes with the OS Report ID `0x00` prepended by Win32 HID API / HIDAPI).

#### Frame Structure (64 Bytes)
Every control and telemetry packet sent to the pump microcontroller consists of 64 bytes:

| Byte Offset | Field Name | Type | Value / Description |
| :--- | :--- | :--- | :--- |
| `0` | Header 1 | `u8` | `0x55` (Magic byte 1) |
| `1` | Header 2 | `u8` | `0xBB` (Magic byte 2) |
| `2` | Data Length | `u8` | `0x02` (2 payload bytes: value high and low) |
| `3` | Command Code (`cmd`) | `u8` | Target register / display target (see table below) |
| `4` | Value High (`valHi`) | `u8` | `(value >> 8) & 0xFF` (MSB, Big-Endian) |
| `5` | Value Low (`valLo`) | `u8` | `value & 0xFF` (LSB) |
| `6` | Checksum (`cksum`) | `u8` | `(byte[0] + byte[1] + byte[2] + byte[3] + byte[4] + byte[5]) & 0xFF` |
| `7..63` | Padding | `[u8; 57]` | 57 zero bytes (`0x00`) |

#### Command Codes (`cmd`)
| Command | Hex | Target | Value Encoding |
| :--- | :---: | :--- | :--- |
| `CMD_TEMPERATURE` | `0x01` | Central 7-segment digital readout | Temperature in °C (integer, e.g. `45` -> `0x002D`). Also used to display numerical CPU load. |
| `CMD_FREQUENCY` | `0x02` | Frequency readout | Clock frequency (e.g. `46` for 4.6 GHz or raw MHz depending on firmware revision). |
| `CMD_USAGE` | `0x03` | Outer circular LED gauge ring | Utilization percentage from `0` to `100` (e.g. `35` -> `0x0023`). |
| `CMD_SHOW` | `0x04` | Display power / sleep mode | `0x0001` = Display ON / Active<br/>`0x0000` = Display OFF / Sleep |

#### Hardware Nuances & Implementation Notes
1. **Central Display vs. Outer Gauge Ring:**
   - The pump hardware uses command `0x01` (`CMD_TEMPERATURE`) to render digits on the central 7-segment display and command `0x03` (`CMD_USAGE`) to fill the perimeter LED ring.
   - When in **CPU Load** mode, RustCooling dispatches both `0x01` (to show the load percentage as a number on the central display) and `0x03` (to fill the circular ring proportionally), matching physical cooler behavior.
2. **Packet Staggering:**
   - The WCH microcontroller does not implement a deep packet queue. Sending multiple back-to-back HID packets without a pause can cause packet drops or MCU lockups.
   - RustCooling staggers multi-packet dispatches with a **100 ms** delay between commands.
3. **50 ms Roller Transition Animation:**
   - When enabled, value changes step through intermediate numbers at a 50 ms cadence (with automatic acceleration if the step delta exceeds the update interval). Smooth interpolation modes were eliminated to avoid display lag and ensure real-time accuracy.
4. **Graceful Shutdown:**
   - The display retains the last sent frame indefinitely even when USB communication stops. On application shutdown, RustCooling explicitly sends `CMD_SHOW(0)` to turn off the LCD screen instead of leaving stale frozen values.
5. **Checksum Algorithm:**
   - Sum of the first 6 bytes modulo 256:
     ```rust
     fn calculate_checksum(header1: u8, header2: u8, len: u8, cmd: u8, val_hi: u8, val_lo: u8) -> u8 {
         (header1 as u16 + header2 as u16 + len as u16 + cmd as u16 + val_hi as u16 + val_lo as u16) as u8
     }
     ```
</details>

---

### Building from Source

```bash
# Prerequisites: Rust 1.80+ (on Linux: libudev-dev, libfontconfig1-dev, libgl1-mesa-dev, libayatana-appindicator3-dev)
git clone https://github.com/Qyzom/RustCooling.git
cd RustCooling
cargo test
cargo build --release
```
The compiled binary will be located in `target/release/RustCooling` (`.exe` on Windows).

---

### Maintenance & Personal Guarantee

> [!IMPORTANT]
> **If you see periods of no new commits, the project is NOT abandoned.**  
> The USB protocol for ID-COOLING FX coolers is fixed in hardware and fully reversed. I use **RustCooling daily** on my personal PC. If any Windows or Linux kernel updates require compatibility adjustments, I will release fixes immediately.

---

### License & Credits

- **License:** [MIT License](LICENSE)
- **Author:** [Qyzom](https://github.com/Qyzom)
- **Typeface:** Embedded *Unbounded* licensed under [SIL Open Font License 1.1](assets/fonts/OFL.txt)
- **Predecessor:** Inspired by my previous work on [idc-lite](https://github.com/Qyzom/idc-lite)