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
| • **< 12 MB** RAM active GUI<br/>• **< 3 MB** in system tray<br/>• **0.0%** idle CPU usage<br/>• **< 50 ms** instant startup | • Direct `/sys/class/hwmon`<br/>• AMD Tctl & Intel Package<br/>• Zero root/Wine required<br/>• Bundled udev permissions | • CPU Temp & CPU Load<br/>• Carousel & smooth modes<br/>• Native dark tray menu<br/>• **5 Languages** (EN, RU, ZH, DE, FR) | • Single monolithic binary<br/>• No .NET or Electron runtime<br/>• Built-in `--daemon` mode<br/>• Zero telemetry or auto-updater |

---

### Benchmark & Comparison

> [!NOTE]
> RustCooling is a complete ground-up rewrite in **100% Rust** of [**idc-lite**](https://github.com/Qyzom/idc-lite) (the author's first C# / .NET project).

| Feature | Original Vendor App | idc-lite (1st Gen) | **RustCooling (Current)** |
| :--- | :---: | :---: | :---: |
| **Technology Stack** | Electron / Node.js + C++ | C# / .NET 8 + WPF | **100% Pure Rust + Slint UI** |
| **RAM (Active GUI)** | ~150 – 300 MB | ~60 – 120 MB | **< 12 MB** (~10.8 MB) |
| **RAM (System Tray)** | ~80 – 150 MB | ~35 – 60 MB | **< 3 MB** (~1.1 MB) |
| **RAM (Headless Daemon)** | None | Separate process (~20 MB) | **~2 – 4 MB** (`--daemon`) |
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
<summary><b>Technical Architecture & Protocol Specification</b></summary>

<br/>

#### Hardware Identification (USB HID)
- **Target Coolers:** ID-COOLING FX Series (FX240, FX280, FX360) and compatible QinHeng / WCH LCD pumps
- **USB VID:** `0x1A86` | **USB PID:** `0xE317`
- **Report Length:** 64 bytes (65 bytes with Windows Report ID `0x00`)

#### HID Protocol Frame Format
| Byte | Field | Description |
| :--- | :--- | :--- |
| `0` | Header 1 | `0x55` |
| `1` | Header 2 | `0xBB` |
| `2` | Length | `0x02` |
| `3` | Command | `0x01` (Temp), `0x02` (Freq), `0x03` (Load), `0x04` (Display Power) |
| `4` | Val High | `(value >> 8) & 0xFF` (Big-Endian) |
| `5` | Val Low | `value & 0xFF` |
| `6` | Checksum | `(byte[0] + ... + byte[5]) & 0xFF` |
| `7..63` | Padding | Zero bytes (`0x00`) |

- **Staggered Dispatch:** 100 ms cadence between metric frames guarantees the microcontroller processes and updates the LCD without dropping packets.
- **Value Deduplication:** Redundant USB transfers are skipped if telemetry values remain unchanged.
- **Working Set Trimming:** Calls `EmptyWorkingSet` at startup and when minimized on Windows to release unused physical RAM pages.
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

### Maintenance Status & Daily Driver Guarantee

> [!IMPORTANT]
> **If you see no commits for several months or a year, the project is NOT abandoned!**  
> The USB protocol for ID-COOLING FX coolers is fixed in hardware and fully reversed. The author uses **RustCooling daily** on their personal workstation. Any kernel or OS compatibility updates will be released immediately.

---

### 📄 License & Credits

- **License:** [MIT License](LICENSE)
- **Typeface:** Embedded *Unbounded* licensed under [SIL Open Font License 1.1](assets/fonts/OFL.txt)
- **Predecessor:** Inspired by lessons learned from [idc-lite](https://github.com/Qyzom/idc-lite)