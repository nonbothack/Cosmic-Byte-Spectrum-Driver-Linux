# Cosmic Byte Spectrum — Protocol Specification

## 1. Overview
The Cosmic Byte Spectrum RGB Gaming Mouse uses a proprietary protocol designed by Instant Microelectronics Co., Ltd. Communicating over USB Full-Speed HID, all configuration commands and queries are carried via **HID Feature Reports** on **Report ID 0x07** with a fixed length of **8 bytes**.

---

## 2. Packet Framing
All configuration packets have the following fixed 8-byte structure:

```
 0        1        2        3        4        5        6        7
+--------+--------+--------+--------+--------+--------+--------+--------+
| 0x07   | CMD    | DATA 0 | DATA 1 | DATA 2 | DATA 3 | DATA 4 | DATA 5 |
+--------+--------+--------+--------+--------+--------+--------+--------+
```

- **Byte 0 (Report ID):** Must always be `0x07`.
- **Byte 1 (Command Opcode):** Opcode indicating the operation.
- **Bytes 2–7 (Payload):** Command-specific arguments. Unused bytes must be `0x00`.

---

## 3. Command Opcodes

### 3.1. Command `0x10`: Button / Key Mapping
Configures the action assigned to a physical mouse button.

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x10 (SET_BUTTON)
Byte 2: Hardware Key Code
Byte 3: Action Code
Byte 4..7: 0x00
```

#### Hardware Key Codes:
- `1`: Button 1 (Left Click)
- `2`: Button 2 (Middle Click / Scroll Wheel Click)
- `3`: Button 3 (Right Click)
- `4`: Button 4 (Backward / Side Back)
- `5`: Button 5 (Forward / Side Front)
- `6`: Button 6 (DPI Loop Switch)

#### Action Codes:
- `0`: Disabled
- `1`: Left Click
- `2`: Middle Click
- `3`: Right Click
- `4`: Back
- `5`: Forward
- `6`: DPI Loop
- `14`: Rapid Fire
- `15`: LED Effect Loop
- `16`: DPI +
- `17`: DPI -
- `41`: Mode Loop

---

### 3.2. Command `0x13`: RGB Lighting Control
Configures effect mode, color, brightness, and animation speed for the RGB underglow.

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x13 (SET_LED)
Byte 2: (user_color_en << 7) | (color_mask & 0x7F)   [0xFF for full spectrum loop]
Byte 3: (direction << 7) | ((mode & 0x0F) << 4) | (scolor << 3) | (speed & 0x07)
Byte 4: (flag1 << 7) | (symmetry << 6) | ((direction << 4) & 0x30) | (color_index & 0x0F)
Byte 5: brightness & 0x07
Byte 6: (custom_color >> 8) & 0x03
Byte 7: custom_color & 0xFF
```

#### Lighting Modes:
1. `DPI Breathing Effect`
2. `Cycle Breathing Effect`
3. `Static Light (Light On)`
4. `Flowing Water Effect`
5. `Mono Water Effect`
6. `Comet Streak`
7. `Neon`
8. `Ambilight`
9. `Flicker`
10. `Star Trek`
11. `Ripple`
12. `Enraptured`
13. `Button Response`
14. `LED Off`
15. `Single Breath`
16. `Cycle Color`

#### Parameters:
- `brightness`: 0 to 4 (Level 1 to 5)
- `speed`: 0 to 4 (Level 1 to 5)
- `direction`: 0 = Forward, 1 = Reverse
- `symmetry`: 0 = Normal, 1 = Symmetric

---

### 3.3. Command `0x14`: DPI Stage LED Indicator Color
Configures the scroll wheel RGB LED color associated with a specific DPI stage.

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x14 (SET_DPI_STAGE_LED)
Byte 2: (stage_index << 5) | (green_nibble & 0x0F)
Byte 3: (red_nibble << 4) | (blue_nibble & 0x0F)
Byte 4..7: 0x00
```

- `stage_index`: 0 to 5 (Stages 1 through 6)
- **Nibble Conversion Formula (VA 0x0042a370):**
  ```c
  nibble = (255 - component_value) / 16;
  ```
  Inverts the 8-bit channel (0..255) into a 4-bit active-low intensity level (0..15).

---

### 3.4. Command `0x15`: Active DPI Stages Queue
Defines the list and ordering of stages visited when the DPI switch button is clicked.

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x15 (SET_ACTIVE_STAGES)
Byte 2: 1st Active Stage (1..6, or 0 if disabled)
Byte 3: 2nd Active Stage (1..6, or 0 if disabled)
Byte 4: 3rd Active Stage (1..6, or 0 if disabled)
Byte 5: 4th Active Stage (1..6, or 0 if disabled)
Byte 6: 5th Active Stage (1..6, or 0 if disabled)
Byte 7: 6th Active Stage (1..6, or 0 if disabled)
```

---

### 3.5. Command `0x16`: Feature Enable Flags
Controls mode flags and enables feature processing in the MCU firmware.

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x16 (SET_FLAGS)
Byte 2: 0x00
Byte 3: flags (0x07 enables all 3 feature channels)
Byte 4..7: 0x00
```

---

### 3.6. Command `0x18`: Register / EEPROM Direct Access
Writes and reads registers and configuration chunks in mouse memory.

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x18 (ACCESS_REGISTER)
Byte 2: Control Flags
          0x03 = Write byte
          0x02 = Finalize step 1
          0x00 = Finalize step 2 / End
          0x05 = Read byte request
Byte 3: byte_index (0..7 within chunk)
Byte 4: chunk_offset & 0xFF
Byte 5: (chunk_offset >> 8) & 0xFF
Byte 6: data_byte (when writing) / 0x00 (when reading)
Byte 7: total_chunk_length - 1 (typically 0x07 for 8-byte chunk)
```

#### Chunk `0x10`: 6-Stage DPI Resolution Step Packing
Stores the 1-based step index (1..24) for all 6 DPI stages:
```
Byte 0: 0x00
Byte 1: 0x00
Byte 2: 0x00
Byte 3: Bit 4 array of all 6 stages:
        ((s0&0x10)>>4) | (((s1&0x10)>>4)<<1) | (((s2&0x10)>>4)<<2) |
        (((s3&0x10)>>4)<<3) | (((s4&0x10)>>4)<<4) | (((s5&0x10)>>4)<<5)
Byte 4: 0x08
Byte 5: ((s0 & 0x0F) << 4) | (s1 & 0x0F)
Byte 6: ((s2 & 0x0F) << 4) | (s3 & 0x0F)
Byte 7: ((s4 & 0x0F) << 4) | (s5 & 0x0F)
```

#### Hardware Step Index Table:
| Step Index | DPI | Hex Value in PE |
|---|---|---|
| 1 | 200 | `0x00C8` |
| 2 | 400 | `0x0190` |
| 3 | 600 | `0x0258` |
| 4 | 800 | `0x0320` |
| 5 | 1000 | `0x03E8` |
| 6 | 1200 | `0x04B0` |
| 7 | 1400 | `0x0578` |
| 8 | 1600 | `0x0640` |
| 9 | 1800 | `0x0708` |
| 10 | 2000 | `0x07D0` |
| 12 | 2400 | `0x0960` |
| 13 | 3200 | `0x0C80` |
| 14 | 4000 | `0x0FA0` |
| 15 | 4800 | `0x12C0` |
| 16 | 5600 | `0x15E0` |
| 17 | 6400 | `0x1900` |
| 18 | 7200 | `0x1C20` |
| 19 | 8000 | `0x1F40` |
| 20 | 8800 | `0x2260` |
| 21 | 9600 | `0x2580` |
| 22 | 10400 | `0x28A0` |
| 23 | 11200 | `0x2BC0` |
| 24 | 12800 | `0x3200` |

---

### 3.7. Command `0x20`: Save / Commit to Onboard Flash
Flushes all settings in MCU RAM into onboard non-volatile EEPROM/flash memory.

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x20 (SAVE_TO_FLASH)
Byte 2..7: 0x00
```

---

## 4. Status Query & Readback
Reading Feature Report `0x07` (`HIDIOCGFEATURE(8)`) returns the current device status:

```
Byte 0: 0x07 (Report ID)
Byte 1: 0x00 (Status)
Byte 2: Current Active DPI Low Byte
Byte 3: Current Active DPI High Byte
Byte 4: State Register
Byte 5: 0x00
Byte 6: [bit 1] = EEPROM size (1=4KB, 0=2KB), [bits 3:2] = Active Profile (0..3)
Byte 7: Signature (0x3B)
```
