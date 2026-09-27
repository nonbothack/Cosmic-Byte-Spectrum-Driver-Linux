# Cosmic Byte Spectrum — Capture Log & Verification Traces

This catalog records raw HID traffic traces and hardware read-backs verified on the physical Cosmic Byte Spectrum mouse (`VID: 0x30fa, PID: 0x1440`).

---

## Capture 01: Hardware Status & EEPROM Identification Read-Back
- **Target Interface:** Interface 1 (`/dev/hidraw5`)
- **Transport Method:** `HIDIOCGFEATURE(8)` (`0xC0084807`)
- **Sent Buffer:** `[0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
- **Received Response:** `[0x07, 0x00, 0x68, 0x09, 0x2b, 0x00, 0x03, 0x3b]`
- **Byte Breakdown:**
  - `Byte 0`: `0x07` (Report ID)
  - `Byte 1`: `0x00` (Status Code)
  - `Byte 2`: `0x68` (DPI low byte)
  - `Byte 3`: `0x09` (DPI high byte) -> `(0x09 << 8) | 0x68 = 2408` (Active 2400 DPI profile)
  - `Byte 4`: `0x2b` (MCU status / state register)
  - `Byte 5`: `0x00` (Reserved)
  - `Byte 6`: `0x03` (`[bit 1]=1` -> 4KB EEPROM, `[bit 3:2]=0` -> Profile 0 Office)
  - `Byte 7`: `0x3b` (Hardware signature constant)

---

## Capture 02: Onboard Flash Memory Direct Read (Command 0x18)
- **Target Interface:** Interface 1 (`/dev/hidraw5`)
- **Transport Method:** Sequence of Command `0x18` followed by `HIDIOCGFEATURE(8)`
- **Request Packet (per byte i in 0..7):**
  `[0x07, 0x18, 0x05, i, 0x00, 0x00, 0x00, 0x07]`
- **Response Received:**
  `Chunk 0 (Offset 0x00): [0x5A, 0x03, 0xA5, 0x40, 0x14, 0xFA, 0x30, 0x10]`
- **Verification:**
  - `0x5A, 0x03, 0xA5`: Instant Microelectronics firmware signature header
  - `0x40, 0x14`: Product ID `0x1440` (little-endian)
  - `0xFA, 0x30`: Vendor ID `0x30FA` (little-endian)
  - `0x10`: Firmware Revision `1.0`

---

## Capture 03: RGB Lighting Mode Switching (Command 0x13)
- **Action:** Switch lighting effect to Neon mode
- **Packet Sent:** `[0x07, 0x13, 0x87, 0x13, 0x00, 0x02, 0x00, 0x00]`
  - `Byte 1`: `0x13` (SET_LED)
  - `Byte 2`: `0x87` (`(1 << 7) | 0x07` -> user_color_en=1, mode=7 Neon)
  - `Byte 3`: `0x13` (symmetry=1, speed=3)
  - `Byte 5`: `0x02` (brightness=2)
- **Physical Verification:** Mouse underglow LEDs immediately switch to multi-color cycling stream.

---

## Capture 04: RGB LED Off (Command 0x13)
- **Action:** Turn off all lighting
- **Packet Sent:** `[0x07, 0x13, 0x8E, 0x13, 0x00, 0x00, 0x00, 0x00]`
  - `Byte 2`: `0x8E` (`(1 << 7) | 0x0E` -> mode=14 Off)
- **Physical Verification:** Mouse underglow LEDs turn completely dark.

---

## Capture 05: Button Remapping (Command 0x10)
- **Action:** Map Middle Button (Button 2) to DPI Loop (`0x06`)
- **Packet Sent:** `[0x07, 0x10, 0x02, 0x06, 0x00, 0x00, 0x00, 0x00]`
- **Physical Verification:** Pressing scroll wheel triggers DPI cycle in hardware.

---

## Capture 06: DPI Stage Color Indicator (Command 0x14)
- **Action:** Configure Stage 0 indicator to Red (`#FF0000`)
- **Packet Sent:** `[0x07, 0x14, 0x0F, 0x0F, 0x00, 0x00, 0x00, 0x00]`
  - `Byte 2`: `(0 << 5) | (0x0F)` (green nibble = 15 inverted)
  - `Byte 3`: `(0x00 << 4) | (0x0F)` (red nibble = 0 inverted, blue nibble = 15 inverted)
- **Physical Verification:** Scroll wheel LED lights up pure red on Stage 0.

---

## Capture 07: Active DPI Stages Queue (Command 0x15)
- **Action:** Enable stages 1, 2, 3, 4 (Disable 5 and 6)
- **Packet Sent:** `[0x07, 0x15, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00]`
- **Physical Verification:** DPI loop cycles through stages 1 to 4 and wraps to 1.

---

## Capture 08: Onboard EEPROM Save (Command 0x20)
- **Action:** Commit active configuration to flash
- **Packet Sent:** `[0x07, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
- **Physical Verification:** Mouse retains custom LED mode, button maps, and DPI stages across physical USB replug.
