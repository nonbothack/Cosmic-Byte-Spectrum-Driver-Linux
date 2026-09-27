# Cosmic Byte Spectrum — Transport Layer Analysis

## 1. Physical Device Identification
- **USB Vendor ID (VID):** `0x30FA` (Instant Microelectronics Co., Ltd.)
- **USB Product ID (PID):** `0x1440` (Instant USB Gaming Mouse / CB Spectrum)
- **Alternate PID in PE:** `0x1E01` (Checked during enumeration at `VA 0x004199ef`)
- **Device Product String:** `INSTANT USB GAMING MOUSE`
- **Linux Device Nodes:**
  - `/dev/hidraw4`: USB Interface 0 (Standard USB HID Boot Mouse, EP 0x81 IN, 8-byte input reports)
  - `/dev/hidraw5`: USB Interface 1 (Keyboard Collection & Vendor Feature Collection)

---

## 2. USB Interface & HID Report Descriptor Breakdown

The device exposes two USB interfaces. Interface 1 (`/dev/hidraw5`) contains the vendor configuration collections:

### Interface 1 Collections:
1. **Application Collection: Keyboard (`UsagePage: 0x01 (Generic Desktop), Usage: 0x06 (Keyboard)`)**
   - Report ID `0x03`: Keyboard modifiers & 6KRO scan codes (Input Report, 8 bytes)
2. **Application Collection: Consumer Control (`UsagePage: 0x0C (Consumer), Usage: 0x01 (Consumer Control)`)**
   - Report ID `0x02`: Media key events (Input Report, 2 bytes)
3. **Application Collection: Mouse Notification (`UsagePage: 0xFF00 (Vendor-defined), Usage: 0x01`)**
   - **Report ID `0x06`:** Asynchronous button & DPI notifications from mouse to PC (Input Report, 2 bytes payload)
   - Windows open path: Opened with `GENERIC_READ` via `CreateFileW` (`VA 0x00419bac`)
   - Read API: Read asynchronously via `ReadFile` (`VA 0x00446c32`)
4. **Application Collection: Vendor Configuration (`UsagePage: 0xFF01 (Vendor-defined), Usage: 0x01`)**
   - **Report ID `0x07`:** Bidirectional configuration transport (Feature Report, 7 bytes payload + 1 byte ID = 8 bytes total)
   - Windows open path: Opened with `GENERIC_WRITE` via `CreateFileW` (`VA 0x00419bfd`)
   - Write API: Written via `HidD_SetFeature` (`VA 0x005c8e03`)
   - Read API: Read via `HidD_GetFeature` (`VA 0x005c8dfd`)
5. **Application Collection: System Power (`UsagePage: 0x01, Usage: 0x80 (System Control)`)**
   - Report ID `0x08`: Sleep/Wake buttons (Input Report, 1 byte)

---

## 3. Host-to-Device Transport Mechanics

### Windows vs. Linux API Mapping
| Operation | Windows OEM API | Linux Native Equivalent |
|---|---|---|
| Open Configuration Node | `SetupDiGetDeviceInterfaceDetailW` + `CreateFileW` for `UsagePage 0xFF01` | `open("/dev/hidraw5", O_RDWR)` |
| Send Configuration Packet | `HidD_SetFeature(hDevice, buf, 8)` | `ioctl(fd, HIDIOCSFEATURE(8), buf)` (`0xC0084806`) |
| Read Device Status | `HidD_GetFeature(hDevice, buf, 8)` | `ioctl(fd, HIDIOCGFEATURE(8), buf)` (`0xC0084807`) |
| Receive Mouse Events | `ReadFile(hReadDevice, buf, 3, ...)` (Report ID 0x06) | `read(fd, buf, 3)` on `/dev/hidraw5` |

### Packet Framing
Every configuration packet sent or received over the transport is exactly **8 bytes** long:
```text
Byte 0: 0x07 (Report ID, mandatory)
Byte 1: Command Opcode
Byte 2: Data 0
Byte 3: Data 1
Byte 4: Data 2
Byte 5: Data 3
Byte 6: Data 4
Byte 7: Data 5
```
- No framing header beyond Report ID `0x07`.
- No packet checksum is required for HID Feature Reports (USB CRC-16 handles link-layer integrity at the hardware bus level).
- Linux kernel hidraw ioctl takes buffer starting with Report ID (`buf[0] = 0x07`).
