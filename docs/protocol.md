# Cosmic Byte Spectrum Protocol Specification

## Hardware Overview
- **Device:** Cosmic Byte Spectrum RGB Wired Gaming Mouse
- **SKU:** `TCBP03429`
- **EAN:** `8906107601954`
- **USB Vendor ID (VID):** `0x30FA` (Instant Microelectronics Co., Ltd.)
- **USB Product ID (PID):** `0x1440` (USB GAMING MOUSE)
- **Control Interface:** USB Interface 1 (Vendor HID Usage Page `0xFF01`, Usage `0x01`)
- **Transport Mechanism:** HID Feature Reports on **Report ID `0x07`**
- **Report Length:** Fixed 8 bytes (`[0x07, CMD, DATA0, DATA1, DATA2, DATA3, DATA4, DATA5]`)

---

## Linux Transport ioctls
Because the device uses HID Feature Reports over the control endpoint without an interrupt OUT endpoint on Interface 1, writes must be sent via `HIDIOCSFEATURE`:
- `HIDIOCSFEATURE(8)` = `0xC0084806`
- `HIDIOCGFEATURE(8)` = `0xC0084807`

---

## Command Set

### 1. Command `0x13`: RGB Chroma Lighting
Sets active lighting effect, animation speed, brightness, direction, and symmetry.
```
Byte 0: 0x07 (Report ID)
Byte 1: 0x13 (Command: SET_LED)
Byte 2: (user_color_en << 7) | (mode & 0x7F)
Byte 3: (direction << 7) | (symmetry << 4) | (speed & 0x07)
Byte 4: 0x00
Byte 5: brightness & 0x07  (0..4 corresponding to Level 1..5)
Byte 6: 0x00
Byte 7: 0x00
```

Supported RGB Modes (16 total):
1. `DPI Breathing`
2. `Cycle Breathing`
3. `Static Light`
4. `Flowing Water`
5. `Mono Water`
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

### 2. Command `0x14`: DPI Stage Value & Color
Configures a single DPI stage and its associated LED color.
```
Byte 0: 0x07 (Report ID)
Byte 1: 0x14 (Command: SET_DPI_STAGE)
Byte 2: (stage_index << 5) | ((dpi_step_index & 0x01) << 4) | (green_nibble & 0x0F)
Byte 3: ((red_nibble & 0x0F) << 4) | (blue_nibble & 0x0F)
Byte 4..7: 0x00
```
- Colors are encoded as inverted 4-bit nibbles: `nibble = ((255 - channel + 8) / 16).min(15)`.
- Valid sensor DPI steps (23 discrete values): `[200, 400, 600, 800, 1000, 1200, 1400, 1600, 1800, 2000, 2400, 3200, 4000, 4800, 5600, 6400, 7200, 8000, 8800, 9600, 10400, 11200, 12800]`.

### 3. Command `0x15`: Active DPI Stages List
Specifies which DPI stages are active in the hardware DPI cycling loop.
```
Byte 0: 0x07 (Report ID)
Byte 1: 0x15 (Command: SET_ACTIVE_STAGES)
Byte 2..7: Stage indices (1-indexed, e.g. 1..6, or 0 if inactive)
```

### 4. Command `0x10`: Programmable Button Mapping
Remaps physical mouse buttons to actions.
```
Byte 0: 0x07 (Report ID)
Byte 1: 0x10 (Command: SET_BUTTON)
Byte 2: Hardware Button Code (1=Left, 2=Middle, 3=Right, 4=Back, 5=Forward, 6=DpiLoop)
Byte 3: Action Code (1=LeftClick, 2=MiddleClick, 3=RightClick, 4=Back, 5=Forward, 6=DpiLoop, 14=RapidFire, 15=LedLoop, 16=Dpi+, 17=Dpi-, 41=ModeLoop, 0=Disabled)
Byte 4..7: 0x00
```

### 5. Command `0x11`: Polling Rate Toggle
```
Byte 0: 0x07 (Report ID)
Byte 1: 0x11 (Command: SET_POLLING)
Byte 2: Hardware Rate Code (0x00=1000Hz, 0x01=500Hz, 0x03=250Hz, 0x07=125Hz)
Byte 3..7: 0x00
```

### 6. Command `0x20`: Save / Commit to EEPROM
Instructs the mouse microcontroller to write all current settings to onboard flash memory.
```
Byte 0: 0x07 (Report ID)
Byte 1: 0x20 (Command: SAVE_TO_FLASH)
Byte 2..7: 0x00
```

---

## Device Readback
Calling `HIDIOCGFEATURE(8)` with buffer `[0x07, 0, 0, 0, 0, 0, 0, 0]` returns the active mouse state:
- `Byte 2-3`: Current active DPI (little-endian 16-bit word).
- `Byte 6`: Status bitfield containing active profile `((byte6 >> 2) & 0x03)` and polling rate.
