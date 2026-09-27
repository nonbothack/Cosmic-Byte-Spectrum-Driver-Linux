# Cosmic Byte Spectrum — Static Call Paths Analysis

## Binary Target
- **Binary Name:** `Gaming Mouse 3.0.exe`
- **Installer:** `CB Spectrum Gaming Mouse.[20230330].exe` (Inno Setup)
- **SHA-256:** `fdfaa726f26d08875eb7a2fd10c7334f75867020179d5461da7e06198bd7c2d1`
- **Architecture:** x86 32-bit PE (`pei-i386`)
- **Image Base:** `0x00400000`

---

## 1. Device Discovery & Handle Management

```text
WinMain / Initialization
  │
  └── VA 0x004198a0 – 0x00419c9f: Device Enumeration & Handle Setup
        │
        ├── SETUPAPI.dll: SetupDiGetClassDevsW (GUID: GUID_DEVINTERFACE_HID {4D1E55B2-F16F-11CF-88CB-001111000030})
        ├── SETUPAPI.dll: SetupDiEnumDeviceInterfaces
        ├── SETUPAPI.dll: SetupDiGetDeviceInterfaceDetailW
        ├── HID.DLL: HidD_GetAttributes
        │     └── Checks: VendorID == 0x30FA, ProductID == 0x1440 (or alternate 0x1E01)
        ├── HID.DLL: HidD_GetPreparsedData
        ├── HID.DLL: HidP_GetCaps
        │
        ├── VA 0x00419b78 – 0x00419bc7: Open Read Handle (ds:0x668418)
        │     ├── Checks: UsagePage == 0xFF00, Usage == 0x0001
        │     └── KERNEL32.dll: CreateFileW(GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, OPEN_EXISTING)
        │           └── Stored in ds:0x668418 (Read Handle, Input Report 0x06)
        │
        └── VA 0x00419bce – 0x00419c15: Open Write Handle (ds:0x668414)
              ├── Checks: UsagePage == 0xFF01, Usage == 0x0001
              └── KERNEL32.dll: CreateFileW(GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, OPEN_EXISTING)
                    └── Stored in ds:0x668414 (Write Handle, Feature Report 0x07)
```

---

## 2. Low-Level Transport Primitive: `HidD_SetFeature`

All configuration writes pass to the mouse as 8-byte HID Feature Reports on Report ID `0x07` through the write handle `ds:0x668414`:

```text
Any Packet Construction Function
  │
  ├── Populates 8-byte buffer at ds:0x6680d0:
  │     Byte 0: 0x07 (Report ID)
  │     Byte 1: Command Opcode (0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x18, 0x20)
  │     Byte 2..7: Command Payload
  │
  └── VA 0x005c8e03: JMP [0x005f91fc] -> HID.DLL: HidD_SetFeature(
        HANDLE hDevice = ds:0x668414,
        PVOID ReportBuffer = 0x6680d0,
        ULONG ReportBufferLength = ds:0x668090 (8)
      )
```

### Complete List of `HidD_SetFeature` Call Sites (23 total):
1. `VA 0x00413939`: Register write routine (`0x413860`)
2. `VA 0x004148c3`: Factory reset / register initialization
3. `VA 0x004271eb`: Command `0x09` (Gun mode / profile selection)
4. `VA 0x00427328`: Macro trigger command
5. `VA 0x00427d54`: Command `0x0B` (X/Y sensitivity offset)
6. `VA 0x00427e86`: Command `0x16` (Mode & feature enable flags)
7. `VA 0x00427f96`: Command `0x15` variant
8. `VA 0x0042802b`: Command `0x16` (`flags = 0x07`)
9. `VA 0x00428147`: Command `0x15` (Active DPI stages queue)
10. `VA 0x00429ee6`: Command `0x13` (RGB lighting effect configuration)
11. `VA 0x00429fa2`: Command `0x14` (DPI stage LED color nibbles)
12. `VA 0x0042a01f`: Command `0x14` variant (single color component)
13. `VA 0x0042a434`: Command `0x14` (DPI stage LED RGB color calculation)
14. `VA 0x0042a55b`: Command `0x12` (Breathing cycle speed parameter)
15. `VA 0x0042e221`: Command `0x11` (Mode loop / profile switch response)
16. `VA 0x0042f986`: Command `0x10` (Button 1 remap)
17. `VA 0x0042f9e5`: Command `0x10` (Button 2 remap)
18. `VA 0x0042fa77`: Command `0x10` (Button 3 remap)
19. `VA 0x0042faf3`: Command `0x10` (Button 4 remap)
20. `VA 0x0042fb71`: Command `0x10` (Button 5/6 remap)
21. `VA 0x0043fdbc`: Command `0x20` (Save settings to onboard EEPROM/flash)
22. `VA 0x0047ee51`: Macro step packet send
23. `VA 0x0047f12e`: Macro finish packet send

---

## 3. High-Level Call Paths by Feature

### 3.1. DPI Configuration Call Path
```text
UI DPI Slider / Preset Change (Stage 0..5)
  │
  ├── 1. Stage Resolution Lookup (VA 0x00417d95 – 0x00417f15)
  │     Maps DPI value (200..12800) -> Step Index (1..24)
  │     Stored in ds:0x662060[stage_index]
  │
  ├── 2. Stage LED Color (VA 0x0042a3a0 – 0x0042a439)
  │     Converts RGB channels to 4-bit nibbles: nibble = (255 - component) / 16
  │     Sends Command 0x14:
  │       Byte 0: 0x07
  │       Byte 1: 0x14
  │       Byte 2: (stage_index << 5) | (green_nibble & 0x0F)
  │       Byte 3: (red_nibble << 4) | (blue_nibble & 0x0F)
  │       Byte 4..7: 0x00
  │     Calls HidD_SetFeature (VA 0x0042a434)
  │
  ├── 3. Resolution Step Packing into Chunk 0x10 (VA 0x0041b520 – 0x0041b688)
  │     Byte 3: Bit 4 array of all 6 stages:
  │       ((s0&0x10)>>4) | (((s1&0x10)>>4)<<1) | (((s2&0x10)>>4)<<2) |
  │       (((s3&0x10)>>4)<<3) | (((s4&0x10)>>4)<<4) | (((s5&0x10)>>4)<<5)
  │     Byte 4: 0x08
  │     Byte 5: ((s0 & 0x0F) << 4) | (s1 & 0x0F)
  │     Byte 6: ((s2 & 0x0F) << 4) | (s3 & 0x0F)
  │     Byte 7: ((s4 & 0x0F) << 4) | (s5 & 0x0F)
  │     Calls Write Chunk Routine (VA 0x00413950) for chunk offset 0x10
  │       Sends each byte via Command 0x18 (VA 0x00413860)
  │
  ├── 4. Active Stages Queue (VA 0x00428070 – 0x0042814c)
  │     Sends Command 0x15:
  │       Byte 0: 0x07
  │       Byte 1: 0x15
  │       Byte 2..7: Enabled stage numbers (1..6) or 0x00
  │     Calls HidD_SetFeature (VA 0x00428147)
  │
  └── 5. Feature Flags (VA 0x00427fe0 – 0x00428030)
        Sends Command 0x16:
          Byte 0: 0x07
          Byte 1: 0x16
          Byte 2: 0x00
          Byte 3: 0x07 (flags enable)
          Byte 4..7: 0x00
        Calls HidD_SetFeature (VA 0x0042802b)
```

### 3.2. Polling Rate Call Path
```text
UI Polling Rate Selection (125, 250, 500, 1000 Hz)
  │
  ├── VA 0x00423a5f – 0x00423aa5: Look up hardware code:
  │     1000 Hz -> 0x00
  │     500 Hz  -> 0x01
  │     250 Hz  -> 0x03
  │     125 Hz  -> 0x07
  │     Stored in ds:0x66889a
  │
  └── VA 0x0041bbe6 – 0x0041bc35: Encoded into Sensor Chunk 0x18:
        Byte 6: Hardware code (ds:0x66889a)
        Calls Write Chunk Routine (VA 0x00413950) for chunk offset 0x18
          Sends via Command 0x18 byte write routine (VA 0x00413860)
```

### 3.3. RGB Lighting Call Path
```text
UI RGB Mode, Color, Speed, Brightness Change
  │
  └── VA 0x00429df1 – 0x00429ee6: Constructs Command 0x13
        Byte 0: 0x07 (Report ID)
        Byte 1: 0x13 (Opcode)
        Byte 2: (user_color_en << 7) | (mode & 0x7F)
        Byte 3: (direction << 7) | (symmetry << 4) | (scolor << 3) | (speed & 0x07)
        Byte 4: (flag1 << 7) | (flag2 << 6) | (brightness_hi << 4) | (color_index & 0x0F)
        Byte 5: brightness & 0x07
        Byte 6: (custom_color >> 8) & 0x03
        Byte 7: custom_color & 0xFF
        Calls HidD_SetFeature (VA 0x00429ee6)
```

### 3.4. Button Remap Call Path
```text
UI Button Remap Selection
  │
  └── VA 0x0042f900 – 0x0042fb71: Constructs Command 0x10
        Byte 0: 0x07
        Byte 1: 0x10
        Byte 2: Hardware Key Code (1=Left, 2=Middle, 3=Right, 4=Back, 5=Forward, 6=DpiLoop)
        Byte 3: Action Code (1=Left, 2=Middle, 3=Right, 4=Back, 5=Forward, 6=DpiLoop, 14=Fire, 15=LedLoop, 16=Dpi+, 17=Dpi-, 41=ModeLoop, 0=Disabled)
        Byte 4..7: 0x00
        Calls HidD_SetFeature
```

### 3.5. Onboard Save / Commit Call Path
```text
UI Click Apply / OK / Save
  │
  ├── 1. Memory Buffer Sync (VA 0x0043c410)
  ├── 2. Register Flush (VA 0x00413ca0)
  │     Calls Command 0x18 finish sequence
  │     Sleep(5ms)
  │
  └── 3. Commit Command (VA 0x0043fd20 – 0x0043fdce)
        Sends Command 0x20:
          Byte 0: 0x07
          Byte 1: 0x20
          Byte 2..7: 0x00
        Calls HidD_SetFeature (VA 0x0043fdbc)
```
