# Cosmic Byte Spectrum — Reverse Engineering Methodology

## 1. Objectives & Safety Constraints
- **Hardware Target:** Cosmic Byte Spectrum RGB Wired Gaming Mouse (SKU: `TCBP03429`, EAN: `8906107601954`).
- **Rule A:** Never invent a protocol. Every packet format must be grounded in descriptor analysis, static reverse engineering, or live hardware observation.
- **Rule B:** Protect the physical device. Controlled writes only after verifying read reports. Firmware updates strictly out of scope.
- **Rule C:** Maintain normal pointer movement and button clicks throughout configuration.

---

## 2. Payload Extraction
The supplied OEM Windows installer `CB Spectrum Gaming Mouse.[20230330].exe` was extracted using `innoextract`:
```bash
innoextract --output-dir .re/extracted "CB Spectrum Gaming Mouse.[20230330].exe"
```
Key extracted components:
- `Gaming Mouse 3.0.exe`: 32-bit PE GUI executable.
- `skins/`: INI files and layout resources defining UI control IDs, language strings, and default key mappings.

---

## 3. Disassembly & Static Analysis
Disassembly of `Gaming Mouse 3.0.exe` revealed that hardware communication is performed via the Windows HID subsystem:
- Calls to `HidD_GetFeature` at `0x5c8dfd`
- Calls to `HidD_SetFeature` at `0x5c8e03`
- All feature reports operate on Report ID `0x07` with fixed 8-byte buffers.

Key function call sites mapped:
- `0x429de0`: Lighting configuration (Command `0x13`).
- `0x429f00`: DPI stage mapping (Command `0x14`).
- `0x427ed5`: Active stages queue (Command `0x15`).
- `0x42f8f0`: Button remapping (Command `0x10`).
- `0x43fd30`: EEPROM flash commit (Command `0x20`).
- `0x446e0e`: Initial status inquiry reading DPI and active profile index via `HIDIOCGFEATURE`.
- `0x413860`: Byte-level EEPROM block programming (Command `0x18`).

---

## 4. Hardware Verification on Linux
Using Linux `HIDIOCGFEATURE(8)` and `HIDIOCSFEATURE(8)` on `/dev/hidraw5`:
1. Verified reading active report: returned `[0x07, 0x00, 0x68, 0x09, 0x0f, 0x00, 0x03, 0x3b]` (2400 DPI, 250 Hz, Profile 1).
2. Verified live RGB state transitions: Neon Mode 7, Static Light, Flowing Water acknowledged and displayed on physical hardware.
3. Verified button remapping: Button 4 mapped to RapidFire, tested, and restored to Back.
