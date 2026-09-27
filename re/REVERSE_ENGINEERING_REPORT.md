# Reverse-Engineering Report: Cosmic Byte Spectrum Gaming Mouse

**Target Input:** `CB Spectrum Gaming Mouse.[20230330].exe`  
**Target Hardware:** Cosmic Byte Spectrum RGB Wired Gaming Mouse (`VID: 0x30FA, PID: 0x1440`)  
**Status:** Completed & Hardware Verified

---

## 1. What is inside the installer?
- **Installer Type:** Inno Setup v5/v6 installer (`CB Spectrum Gaming Mouse.[20230330].exe`, SHA-256: `b3b8babdc34b78d7b4dbe21273ce9e461f6bbb06d21fca5b38d06c3a6ac83698`).
- **Extracted Content:** 158 total files extracted via `innoextract`:
  - 1 main GUI executable: `Gaming Mouse 3.0.exe` (SHA-256: `fdfaa726f26d08875eb7a2fd10c7334f75867020179d5461da7e06198bd7c2d1`).
  - Configuration files: `config.ini`, `configReset.ini`, `LanguageText.ini`, `LBWarning.ini`.
  - UI skin layouts: `skin_main.ini`, `skin_color.ini`, `skin_keyset.ini`, `skin_advance.ini`, `skin_mac.ini`.
  - Preloaded macros: `constMacro.dat`, `macroGrp.ini`, `PUBG1.dat`, `PUBG2.dat`.
  - Fonts & artwork: TrueType fonts (`Roboto`, `Manrope`, `Cairo`) and JPEG/PNG UI textures.
- **Drivers/Services:** No kernel drivers or background Windows services are bundled. The software is a purely user-mode application communicating directly with standard Windows HID subsystem drivers (`hidclass.sys` / `hidusb.sys`).

---

## 2. Which binary controls the mouse?
- **Executable:** `Gaming Mouse 3.0.exe`.
- **PE Type:** 32-bit x86 Portable Executable (`pei-i386`), statically linked runtime, single monolithic binary.
- All hardware detection, packet framing, USB transfers, profile management, and macro execution are handled directly by this single process.

---

## 3. How is the mouse discovered?
- **Discovery Routine:** `VA 0x004198a0` – `0x00419c9f`.
- **Discovery Mechanism:**
  1. Queries HID device class using `SETUPAPI.dll`: `SetupDiGetClassDevsW` with `GUID_DEVINTERFACE_HID` (`{4D1E55B2-F16F-11CF-88CB-001111000030}`).
  2. Enumerates device interfaces via `SetupDiEnumDeviceInterfaces` and retrieves device path strings with `SetupDiGetDeviceInterfaceDetailW`.
  3. Opens candidate devices with `CreateFileW` and queries hardware attributes via `HID.DLL`: `HidD_GetAttributes`.
  4. Validates that Vendor ID equals `0x30FA` and Product ID equals `0x1440` (or fallback `0x1E01`).
  5. Obtains preparsed report descriptor data with `HidD_GetPreparsedData` and parses capabilities with `HidP_GetCaps`.
  6. Checks usage page and usage:
     - `UsagePage == 0xFF00, Usage == 0x0001`: Opens read handle for mouse notifications (`ds:0x668418`).
     - `UsagePage == 0xFF01, Usage == 0x0001`: Opens write handle for feature reports (`ds:0x668414`).

---

## 4. What VID/PID/interfaces are involved?
- **Vendor ID (VID):** `0x30FA` (Instant Microelectronics Co., Ltd.)
- **Product ID (PID):** `0x1440` (Instant USB Gaming Mouse)
- **Linux Interfaces:**
  - `Interface 0` (`/dev/hidraw4`): Standard USB mouse collection (`UsagePage: 0x01, Usage: 0x02`), standard motion and button reports.
  - `Interface 1` (`/dev/hidraw5`): Vendor configuration and keyboard collection (`UsagePage: 0xFF01, Usage: 0x01` for config; `UsagePage: 0xFF00, Usage: 0x01` for input events).

---

## 5. What transport is used?
- **Transport:** USB Full-Speed HID Feature Reports.
- **Report ID:** `0x07` (mandatory byte 0).
- **Report Length:** Exactly **8 bytes** (1 byte Report ID + 7 payload bytes).
- **Control Mechanism:** USB Control Transfers over Endpoint 0 (`SET_REPORT` / `GET_REPORT` on Feature Report `0x07`).
- **Linux Node:** `/dev/hidraw5` (Interface 1).

---

## 6. What Windows APIs are used?
- `SETUPAPI.dll`:
  - `SetupDiGetClassDevsW`
  - `SetupDiEnumDeviceInterfaces`
  - `SetupDiGetDeviceInterfaceDetailW`
  - `SetupDiDestroyDeviceInfoList`
- `HID.DLL`:
  - `HidD_GetAttributes`
  - `HidD_GetPreparsedData`
  - `HidD_FreePreparsedData`
  - `HidP_GetCaps`
  - `HidD_SetFeature` (`VA 0x005c8e03`): Sends 8-byte configuration feature reports.
  - `HidD_GetFeature` (`VA 0x005c8dfd`): Reads 8-byte status/register reports.
- `KERNEL32.dll`:
  - `CreateFileW`: Opens device endpoints.
  - `ReadFile` (`VA 0x005f9460`): Asynchronously reads Input Report `0x06` for button/DPI notifications.
  - `CloseHandle`: Releases device handles.

---

## 7. What packets configure DPI?
The previous implementation suffered from the bug where "GUI reports 800 DPI, physical mouse remains at previous DPI" because of a fundamental protocol misunderstanding. Static reverse engineering revealed the true DPI architecture:

1. **Command `0x14` (`VA 0x0042a3a0`):**
   - Configures **ONLY** the stage's scroll-wheel LED indicator color, NOT the DPI sensor resolution!
   - Format: `[0x07, 0x14, (stage << 5) | (g_nib & 0x0F), (r_nib << 4) | (b_nib & 0x0F), 0, 0, 0, 0]`.
   - Nibble inversion: `nibble = (255 - component) / 16`.
2. **Resolution Step Packing in Memory Chunk `0x10` (`VA 0x0041b520` – `0x0041b688`):**
   - The sensor DPI step indices (1..24, corresponding to 200..12800 DPI) for all 6 stages are packed into an 8-byte record sent via Command `0x18`:
     - `Byte 3`: bit 4 array of all 6 stages:
       `((s0&0x10)>>4) | (((s1&0x10)>>4)<<1) | (((s2&0x10)>>4)<<2) | (((s3&0x10)>>4)<<3) | (((s4&0x10)>>4)<<4) | (((s5&0x10)>>4)<<5)`
     - `Byte 4`: `0x08`
     - `Byte 5`: `((s0 & 0x0F) << 4) | (s1 & 0x0F)`
     - `Byte 6`: `((s2 & 0x0F) << 4) | (s3 & 0x0F)`
     - `Byte 7`: `((s4 & 0x0F) << 4) | (s5 & 0x0F)`
3. **Command `0x15` (`VA 0x00428070`):**
   - Sets the active stages list: `[0x07, 0x15, stage1, stage2, stage3, stage4, stage5, stage6]`.
4. **Command `0x16` (`VA 0x00427fe0`):**
   - Enables DPI/Feature flags: `[0x07, 0x16, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00]`.

---

## 8. What packets configure RGB?
- **Command:** `0x13` (`VA 0x00429df1` – `0x00429ee6`).
- **Packet Layout:**
  - `Byte 0`: `0x07`
  - `Byte 1`: `0x13`
  - `Byte 2`: `(user_color_en << 7) | (mode & 0x7F)`
  - `Byte 3`: `(direction << 7) | (symmetry << 4) | (scolor << 3) | (speed & 0x07)`
  - `Byte 4`: `(flag1 << 7) | (flag2 << 6) | (brightness_hi << 4) | (color_index & 0x0F)`
  - `Byte 5`: `brightness & 0x07`
  - `Byte 6`: `(custom_color >> 8) & 0x03`
  - `Byte 7`: `custom_color & 0xFF`
- **Supported Modes:** 16 modes (DPI Breathing, Cycle Breathing, Static Light, Flowing Water, Mono Water, Comet Streak, Neon, Ambilight, Flicker, Star Trek, Ripple, Enraptured, Button Response, LED Off, Single Breath, Cycle Color).

---

## 9. What packets configure polling?
- **Hardware Register:** Sensor configuration chunk `0x18` (`VA 0x0041bbe6` – `0x0041bc35`).
- **Hardware Codes:**
  - 1000 Hz: `0x00`
  - 500 Hz: `0x01`
  - 250 Hz: `0x03`
  - 125 Hz: `0x07`
- Stored in Byte 6 of chunk `0x18`, transmitted via Command `0x18` write sequence.

---

## 10. What packets configure buttons?
- **Command:** `0x10` (`VA 0x0042f900` – `0x0042fb71`).
- **Packet Layout:**
  - `Byte 0`: `0x07`
  - `Byte 1`: `0x10`
  - `Byte 2`: Hardware button code (1=Left, 2=Middle, 3=Right, 4=Back, 5=Forward, 6=DpiLoop)
  - `Byte 3`: Action code (1=Left, 2=Middle, 3=Right, 4=Back, 5=Forward, 6=DpiLoop, 14=Fire, 15=LedLoop, 16=DPI+, 17=DPI-, 41=ModeLoop, 0=Disabled)
  - `Byte 4..7`: `0x00`

---

## 11. What happens during Apply/Save?
- **Call Site:** `VA 0x0043fd20` – `0x0043fdce`.
- **Sequence:**
  1. Synchronizes memory structures (`VA 0x0043c410`).
  2. Flushes register buffer (`VA 0x00413ca0`) with Command `0x18` finish sequence and 5 ms sleep.
  3. Sends Command `0x20`:
     ```
     [0x07, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
     ```
  4. The mouse MCU writes all volatile RAM registers to internal non-volatile EEPROM/flash memory.

---

## 12. Is there an ACK?
- The protocol does not use a separate custom in-band ACK packet.
- ACK is handled at the **USB Link Layer**: The device responds to every valid `SET_REPORT` control transfer with a standard USB `ACK` handshake.
- If a malformed buffer or invalid Report ID is sent, the device responds with USB `STALL`, resulting in an `EIO` (I/O error) on the Linux `ioctl`.

---

## 13. Is there read-back?
- **Yes.**
  - **Status Query:** Reading HID Feature Report `0x07` via `HIDIOCGFEATURE(8)` returns current device status.
    - Bytes 2–3: Active DPI in little-endian format (e.g. `0x0968` = 2408 ≈ 2400 DPI).
    - Byte 6: Active Profile `[bits 3:2]` and EEPROM Capacity `[bit 1]`.
  - **Memory Dump:** Reading chunk bytes via Command `0x18` (`Byte 2 = 0x05`) followed by `HIDIOCGFEATURE(8)` returns exact EEPROM byte contents (e.g. verified Chunk 0 header `5a 03 a5 40 14 fa 30 10`).

---

## 14. Where are settings stored?
- **Device Side:**
  - MCU Volatile RAM: Active running configuration.
  - MCU Non-Volatile EEPROM (4088 bytes): Persistent store committed when Command `0x20` executes.
- **Host Side:**
  - OEM software stores local profile presets in `skins/config/config.ini` under `[ShareData]` and `[Config0..3]`.
  - Linux application stores local profiles in `~/.config/cosmic-byte-spectrum/profiles/*.toml`.

---

## 15. What packet sequences are required?
- **To update DPI:**
  1. Send Command `0x14` for each modified stage (LED color).
  2. Send Command `0x18` byte writes for Chunk `0x10` (DPI step indices) followed by finalize packets.
  3. Send Command `0x15` (Active stages queue).
  4. Send Command `0x16` (`flags = 0x07`).
  5. Send Command `0x20` (Commit to flash).
  6. Read back Feature Report `0x07` to confirm active DPI matches expected.
- **To update RGB:**
  1. Send Command `0x13` (RGB parameters).
  2. Send Command `0x20` (Commit).
- **To remap Buttons:**
  1. Send Command `0x10` for each button.
  2. Send Command `0x20` (Commit).

---

## 16. What remains unknown?
- **Firmware Flashing Routine:** The OEM updater routines were not reverse engineered because firmware flashing is explicitly out of scope for v1 to protect physical mouse hardware from bricking.
- **Macro Script Storage Format on MCU:** Complex multi-step macros (e.g. recoil compensation scripts in `PUBG.dat`) are handled partially host-side via simulated keyboard scan codes; onboard macro memory limits are unverified.
- All core user-facing features (DPI stages, DPI resolutions, polling rates, RGB effects, button mappings, profiles, and onboard persistence) are **100% understood, documented, and verified on real hardware**.
