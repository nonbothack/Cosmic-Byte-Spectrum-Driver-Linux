# Cosmic Byte Spectrum — Feature Protocol Matrix

This matrix maps every device capability against its read/write semantics, persistence, evidence source in static disassembly, and physical hardware verification status on the live Cosmic Byte Spectrum mouse (`VID: 0x30fa, PID: 0x1440`).

| Feature | Read | Write | ACK Mechanism | Read-back Verified | Persistent Across Reconnect | Evidence Source | Hardware Verification Status |
|---|:---:|:---:|:---:|:---:|:---:|---|:---:|
| **Device Detection** | ✅ | N/A | HID enumeration | ✅ USB sysfs | ✅ | `VA 0x004198a0`, `/dev/hidraw5` | ✅ VERIFIED |
| **DPI Resolution Step** | ✅ | ✅ | USB ACK | ✅ Feature Report 0x07 (Bytes 2–3) | ✅ (via Command 0x20) | `VA 0x0041b520`, `VA 0x00417d95` | ✅ VERIFIED |
| **DPI Stage LED Color** | ✅ | ✅ | USB ACK | ✅ Stage LED changes | ✅ (via Command 0x20) | `VA 0x0042a3a0` (Command 0x14) | ✅ VERIFIED |
| **Active DPI Stages Queue** | ✅ | ✅ | USB ACK | ✅ Button loop cycle | ✅ (via Command 0x20) | `VA 0x00428070` (Command 0x15) | ✅ VERIFIED |
| **Active Mode / Flags** | ✅ | ✅ | USB ACK | ✅ Feature Report 0x07 (Byte 6) | ✅ (via Command 0x20) | `VA 0x00427e48` (Command 0x16) | ✅ VERIFIED |
| **RGB Lighting Mode** | ❌ (RAM) | ✅ | USB ACK | ✅ Visual LED change | ✅ (via Command 0x20) | `VA 0x00429df1` (Command 0x13) | ✅ VERIFIED |
| **RGB Color / Speed / Brightness** | ❌ (RAM) | ✅ | USB ACK | ✅ Visual LED change | ✅ (via Command 0x20) | `VA 0x00429df1` (Command 0x13) | ✅ VERIFIED |
| **Programmable Buttons** | ❌ (RAM) | ✅ | USB ACK | ✅ Functional test | ✅ (via Command 0x20) | `VA 0x0042f900` (Command 0x10) | ✅ VERIFIED |
| **Onboard Flash Commit** | N/A | ✅ | USB ACK | ✅ Preserved on replug | ✅ Persistent | `VA 0x0043fd20` (Command 0x20) | ✅ VERIFIED |
| **Onboard EEPROM Direct Access** | ✅ | ✅ | USB ACK + GetFeature | ✅ Verified Byte 0..7 | ✅ Persistent | `VA 0x00413860`, `VA 0x00413a00` (Cmd 0x18) | ✅ VERIFIED |
| **Firmware Update** | ❌ | ❌ | N/A | N/A | N/A | Explicitly excluded for safety | ❌ Out of Scope (v1) |

---

## Semantics Summary
1. **DPI Read-back:** The current active DPI can be queried at any time by reading HID Feature Report `0x07` (`HIDIOCGFEATURE(8)`). Bytes 2 and 3 return the active DPI resolution in little-endian format (e.g. `0x0968` ≈ 2400 DPI).
2. **Onboard Persistence:** Modifications to RGB, DPI stages, and Button mappings remain in device MCU RAM until Command `0x20` is transmitted. Once Command `0x20` executes, the MCU commits all registers to internal non-volatile flash memory, surviving disconnects and power cycles.
3. **ACK Behavior:** The device acknowledges all valid HID Feature Reports at the USB bus protocol level (USB ACK handshake). Malformed buffers or incorrect Report IDs are rejected with a USB STALL / `EIO`.
