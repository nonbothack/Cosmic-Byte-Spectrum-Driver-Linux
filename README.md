# Cosmic Byte Spectrum Control for Linux

[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Platform](https://img.shields.io/badge/Platform-Linux%20x86__64-orange.svg)](#)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-green.svg)](#)

Open-source, native Linux control utility and desktop dashboard for the **Cosmic Byte Spectrum RGB Wired Gaming Mouse** (SKU: `TCBP03429`, EAN: `8906107601954`).

Configure DPI resolution, report rate (polling), 16 hardware RGB lighting effects, button remapping, and configuration profiles natively on Linux without relying on the Windows-only OEM software.

---

## Hardware Support Matrix

| Feature | Hardware Status | Linux CLI (`spectrumctl`) | Desktop GUI (`spectrum-gui`) |
|---|:---:|:---:|:---:|
| **Device Detection** | ✅ Supported | ✅ Available | ✅ Live Indicator |
| **DPI Configuration (200–12,800)** | ✅ Supported (23 steps) | ✅ `spectrumctl dpi set` | ✅ Interactive Slider & Chips |
| **DPI Stages Switcher** | ✅ Supported (6 stages) | ✅ Active Queue | ✅ Visual Stage Matrix |
| **USB Polling Rate (125–1000 Hz)** | ✅ Supported | ✅ `spectrumctl polling set` | ✅ Tactile Latency Tiles |
| **16 Hardware RGB Modes** | ✅ Supported | ✅ `spectrumctl rgb set` | ✅ Real-time Mouse Schematic Glow |
| **RGB Custom Color & Brightness** | ✅ Supported | ✅ Hex & 4-bit nibbles | ✅ Palette Picker & Sliders |
| **Programmable Button Remapping** | ✅ Supported (6 buttons) | ✅ `spectrumctl button set` | ✅ Interactive Remap Grid |
| **Host Profiles** | ✅ Supported | ✅ TOML storage | ✅ Quick Switcher |
| **Onboard Flash Commit** | ✅ Supported | ✅ `spectrumctl save` | ✅ Save to Mouse Button |
| **Firmware Update** | ❌ Out of scope (Safety) | ❌ Excluded | ❌ Excluded |

---

## Architecture Overview

```text
       spectrum-gui (Desktop Web Dashboard)      spectrumctl (CLI Harness)
                              │                                │
                              └───────────────┬────────────────┘
                                              ▼
                                 crates/spectrum-core
                             (High-Level Device Service)
                                              │
                              ┌───────────────┴────────────────┐
                              ▼                                ▼
                    crates/spectrum-protocol           crates/spectrum-hid
                    (Packets, Opcodes, Stages)      (Linux /dev/hidraw ioctl)
                                                               │
                                                               ▼
                                                      /dev/hidraw5 (Interface 1)
                                                      Cosmic Byte Spectrum Mouse
```

- **`crates/spectrum-protocol`**: Pure protocol codec. Encodes and decodes 8-byte HID feature reports, converts 23 DPI steps, 4-bit inverted color nibbles, button action codes, and status packets.
- **`crates/spectrum-hid`**: Transport layer. Auto-discovers the vendor interface (Interface 1) via sysfs and issues Linux `HIDIOCSFEATURE` / `HIDIOCGFEATURE` ioctls. Includes a mock backend for offline testing.
- **`crates/spectrum-core`**: High-level device service trait `SpectrumDevice`, state manager, and host profile serialization.
- **`crates/spectrum-cli`**: Command-line tool `spectrumctl`.
- **`crates/spectrum-ui`**: Embedded glassmorphic desktop GUI `spectrum-gui`.

---

## Installation & udev Rules

### 1. Install udev Rules
Grant unprivileged access to the vendor HID node so `root` or `sudo` is not required:
```bash
sudo cp packaging/udev/99-cosmic-byte-spectrum.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```
*Note: If the mouse was already plugged in, unplug and replug the USB cable.*

### 2. Building from Source
Ensure you have the Rust toolchain (1.75+) installed:
```bash
git clone https://github.com/cosmic-byte-spectrum/spectrum-linux.git
cd spectrum-linux
cargo build --release
```
Binaries will be placed in `target/release/`:
- `target/release/spectrumctl`
- `target/release/spectrum-gui`

---

## CLI Usage (`spectrumctl`)

### Detect Device & View Active State
```bash
# Detect connected device
spectrumctl list

# Inspect device hardware info
spectrumctl info

# Read all active settings (DPI, polling, RGB, buttons)
spectrumctl get

# Output structured JSON for automation/scripts
spectrumctl --json get
```

### DPI Configuration
```bash
# Read current active DPI
spectrumctl dpi get

# Set active DPI (200..12800)
spectrumctl dpi set 1600

# View all 23 valid hardware sensor steps
spectrumctl dpi list-steps
```

### USB Polling Rate
```bash
# Read polling rate
spectrumctl polling get

# Set polling rate to 1000 Hz, 500 Hz, 250 Hz, or 125 Hz
spectrumctl polling set 1000
```

### Chroma RGB Lighting
```bash
# View all 16 supported hardware RGB lighting modes
spectrumctl rgb list-modes

# Set lighting mode and color
spectrumctl rgb set --mode neon --color "#00f0ff" --brightness 3 --speed 3

# Static light effect
spectrumctl rgb set --mode static --color "#ff0055" --brightness 4

# Turn LED off
spectrumctl rgb set --mode off
```

### Programmable Button Mapping
```bash
# View button mappings
spectrumctl button list

# Map Button 4 (Side Back) to Rapid Fire
spectrumctl button set 4 rapidfire

# Restore Button 4 to Back
spectrumctl button set 4 back
```

### Configuration Profiles & Onboard Storage
```bash
# List saved profiles
spectrumctl profile list

# Load and apply built-in profile (gaming, office, default)
spectrumctl profile load gaming

# Save current settings to a custom profile
spectrumctl profile save my-fps --description "Custom FPS config"

# Permanently commit current settings to the mouse's internal EEPROM
spectrumctl save
```

---

## Desktop GUI (`spectrum-gui`)

Launch the local GUI control dashboard:
```bash
spectrum-gui
```
This automatically starts the local engine and launches the dashboard in your default browser at `http://127.0.0.1:4567`.

Options:
```bash
# Run without opening browser window (headless/server mode)
spectrum-gui --no-browser

# Bind to a custom port
spectrum-gui --port 8080

# Specify explicit hidraw node
spectrum-gui --device /dev/hidraw5
```

Features of the Desktop GUI:
- **Live Connection Status:** Real-time pulse indicator and device details.
- **Sensor Resolution:** Big digital readout, step slider, quick-preset chips (400, 800, 1200, 1600, 2400, 3200, 6400 DPI), and full 6-stage editor.
- **Report Rate:** Interactive 125 Hz, 250 Hz, 500 Hz, and 1000 Hz tactile tiles.
- **Chroma RGB Studio:** Interactive mouse schematic with dynamic live glow, 16 hardware effects, color picker, hex input, quick neon palette, brightness, speed, direction, and symmetry controls.
- **Button Mapping Matrix:** Visual configuration for all 6 physical mouse buttons.
- **Onboard Commit:** One-click permanent save to mouse flash memory.

---

## Safety & Device Integrity

- **Pointer Protection:** Standard pointer tracking, clicks, and scroll wheel remain completely functional at all times via Linux `hid-generic` on Interface 0.
- **No Guessed Packets:** Every command opcode and payload structure is derived from hardware report descriptors and verified disassembly.
- **Firmware Safety:** Firmware update operations are intentionally excluded to prevent any risk of bricking the physical hardware.

---

## License

Dual-licensed under either:
- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)
