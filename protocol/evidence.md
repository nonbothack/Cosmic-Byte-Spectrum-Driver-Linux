# Cosmic Byte Spectrum — Protocol Reverse Engineering Evidence

## 1. Executive Summary

Static analysis of the OEM Windows application (`Gaming Mouse 3.0.exe` from `CB Spectrum Gaming Mouse.[20230330].exe`) combined with live Linux HID descriptor inspection on the physical Cosmic Byte Spectrum mouse (`VID: 0x30fa, PID: 0x1440`) reveals the exact communication protocol.

Communication uses **HID Feature Reports** on **Report ID 0x07** over USB Interface 1 (`/dev/hidraw5`).
Every command is sent as an 8-byte buffer:
```
[0x07, COMMAND_BYTE, DATA0, DATA1, DATA2, DATA3, DATA4, DATA5]
```

---

## 2. Transport Layer

- **Bus:** USB Full Speed
- **Control Interface:** Interface 1 (Keyboard & Vendor HID Collection)
- **Linux Device Node:** `/dev/hidraw5` (Interface 1)
- **Usage Page:** `0xFF01` (Vendor-defined)
- **Usage:** `0x01`
- **Report Type:** Feature Report (`Report ID: 0x07`, Length: 8 bytes total = 1 byte ID + 7 payload bytes)
- **HID Report Descriptor Reference:**
  ```
  0x06, 0x01, 0xFF,  // Usage Page (Vendor-defined 0xFF01)
  0x09, 0x01,        // Usage (0x01)
  0xA1, 0x01,        // Collection (Application)
  0x85, 0x07,        //   Report ID (7)
  0x15, 0x00,        //   Logical Minimum (0)
  0x26, 0xFF, 0x00,  //   Logical Maximum (255)
  0x09, 0x20,        //   Usage (0x20)
  0x95, 0x07,        //   Report Count (7)
  0x75, 0x08,        //   Report Size (8 bits)
  0xB1, 0x02,        //   Feature (Data,Var,Abs)
  0xC0               // End Collection
  ```
- **Linux Host Ioctls:**
  - `HIDIOCSFEATURE(8)` = `0xC0084806` (Write Feature Report)
  - `HIDIOCGFEATURE(8)` = `0xC0084807` (Read Feature Report)

---

## 3. Protocol Commands

### 3.1. Command `0x13`: RGB Lighting Control
- **Opcode:** `0x13`
- **Call site in PE:** `VA 0x429de0`
- **Buffer layout:**
  ```
  Byte 0: 0x07 (Report ID)
  Byte 1: 0x13 (Command)
  Byte 2: (user_color_en << 7) | (mode & 0x7F)
  Byte 3: (direction << 7) | (symmetry << 4) | (scolor << 3) | (speed & 0x07)
  Byte 4: (flag1 << 7) | (flag2 << 6) | (brightness_hi << 4) | (color_index & 0x0F)
  Byte 5: brightness & 0x07
  Byte 6: (custom_color >> 8) & 0x03
  Byte 7: custom_color & 0xFF
  ```
- **Lighting Modes (from `LanguageText.ini` `[UniqueName_LED]`):**
  - `1`: DPI Breathing Effect
  - `2`: Cycle Breathing Effect
  - `3`: Light On (Static)
  - `4`: Flowing Water Effect
  - `5`: Mono Water Effect
  - `6`: Comet Streak
  - `7`: Neon
  - `8`: Ambilight
  - `9`: Flicker
  - `10`: Star Trek
  - `11`: Ripple
  - `12`: Enraptured
  - `13`: Button Response
  - `14`: LED Off
  - `15`: Single Breath
  - `16`: Cycle Color

### 3.2. Command `0x14`: DPI Stage Configuration
- **Opcode:** `0x14`
- **Call site in PE:** `VA 0x429f00` / `VA 0x42a3a0`
- **Buffer layout:**
  ```
  Byte 0: 0x07 (Report ID)
  Byte 1: 0x14 (Command)
  Byte 2: (stage_index << 5) | ((dpi_step_index & 0x01) << 4) | (green_nibble & 0x0F)
  Byte 3: ((red_nibble & 0x0F) << 4) | (blue_nibble & 0x0F)
  Byte 4..7: 0x00
  ```
- **DPI Step Mapping Table (from disassembly at `VA 0x417d95..0x417f15`):**
  | Step Index | Value in Binary | Target DPI |
  |---|---|---|
  | 1 | `0x00c8` | 200 |
  | 2 | `0x0190` | 400 |
  | 3 | `0x0258` | 600 |
  | 4 | `0x0320` | 800 |
  | 5 | `0x03e8` | 1000 |
  | 6 | `0x04b0` | 1200 |
  | 7 | `0x0578` | 1400 |
  | 8 | `0x0640` | 1600 |
  | 9 | `0x0708` | 1800 |
  | 10 | `0x07d0` | 2000 |
  | 12 | `0x0960` | 2400 |
  | 13 | `0x0c80` | 3200 |
  | 14 | `0x0fa0` | 4000 |
  | 15 | `0x12c0` | 4800 |
  | 16 | `0x15e0` | 5600 |
  | 17 | `0x1900` | 6400 |
  | 18 | `0x1c20` | 7200 |
  | 19 | `0x1f40` | 8000 |
  | 20 | `0x2260` | 8800 |
  | 21 | `0x2580` | 9600 |
  | 22 | `0x28a0` | 10400 |
  | 23 | `0x2bc0` | 11200 |
  | 24 | `0x3200` | 12800 |

### 3.3. Command `0x15`: Active DPI Stages Queue
- **Opcode:** `0x15`
- **Call site in PE:** `VA 0x427ed5` / `VA 0x428086`
- **Buffer layout:**
  ```
  Byte 0: 0x07 (Report ID)
  Byte 1: 0x15 (Command)
  Byte 2: Stage 1 index (or 0 if disabled)
  Byte 3: Stage 2 index (or 0 if disabled)
  Byte 4: Stage 3 index (or 0 if disabled)
  Byte 5: Stage 4 index (or 0 if disabled)
  Byte 6: Stage 5 index (or 0 if disabled)
  Byte 7: Stage 6 index (or 0 if disabled)
  ```

### 3.4. Command `0x10`: Button / Key Mapping
- **Opcode:** `0x10`
- **Call site in PE:** `VA 0x42f8f0`
- **Buffer layout:**
  ```
  Byte 0: 0x07 (Report ID)
  Byte 1: 0x10 (Command)
  Byte 2: Hardware Key Code
  Byte 3: Action Code
  Byte 4..7: 0x00
  ```
- **Hardware Key Codes (from `config.ini` `[KEYMAP]`):**
  - Button 1 (Left): `1`
  - Button 2 (Middle): `2`
  - Button 3 (Right): `3`
  - Button 4 (Forward): `5`
  - Button 5 (Back): `4`
  - Button 6 (DPI Loop): `6`
- **Action Codes (from `config.ini` `[KEY_UIFC]` and `LanguageText.ini`):**
  - `1`: Left Click
  - `2`: Middle Click
  - `3`: Right Click
  - `4`: Back
  - `5`: Forward
  - `6`: DPI Loop
  - `14`: Rapid Fire
  - `15`: LED Loop
  - `16`: DPI+
  - `17`: DPI-
  - `41`: Mode Loop
  - `0`: Disabled / No function

### 3.5. Polling Rate Mapping (USB Speed)
- **Call site in PE:** `VA 0x423a60`
  - 1000 Hz: Hardware value `0x00`
  - 500 Hz: Hardware value `0x01`
  - 250 Hz: Hardware value `0x03`
  - 125 Hz: Hardware value `0x07`

### 3.6. Command `0x20`: Save / Commit Settings
- **Opcode:** `0x20`
- **Call site in PE:** `VA 0x43fd30`
- **Buffer layout:**
  ```
  Byte 0: 0x07 (Report ID)
  Byte 1: 0x20 (Command)
  Byte 2..7: 0x00
  ```
  This command instructs the MCU to save all written settings into onboard non-volatile memory.

---

## 4. Device Status Readback

- Reading via `HIDIOCGFEATURE(8)` with buffer `[0x07, 0, 0, 0, 0, 0, 0, 0]` returns the current device status.
- Verified on live mouse:
  ```
  Response: [0x07, 0x00, 0x68, 0x09, 0x00, 0x00, 0x03, 0x3b]
  ```
- Byte 2-3 contains current DPI: `(9 << 8) | 0x68` ≈ 2400 DPI (`0x0960`).
- Byte 6 contains Polling Rate status: `0x03` (1000 Hz / USB_SPEED 3).
