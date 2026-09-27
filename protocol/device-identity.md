# Cosmic Byte Spectrum — Device Identity & Hardware Profile

## 1. Physical Device Identification (Linux Host)

- **Vendor ID:** `0x30fa` (INSTANT Microelectronics Co., Ltd.)
- **Product ID:** `0x1440` (USB GAMING MOUSE)
- **USB Revision:** `1.10`
- **Device Speed:** Full Speed (12 Mbps)
- **Manufacturer String:** `INSTANT`
- **Product String:** `USB GAMING MOUSE`
- **Serial Number:** None (`iSerial: 0`)
- **Device Node (Control):** `/dev/hidraw5` (Interface 1)
- **Device Node (Mouse pointer):** `/dev/hidraw4` (Interface 0)

## 2. USB Interfaces and Endpoints

### Interface 0: Standard Mouse
- **Class:** `0x03` (HID)
- **Subclass:** `0x01` (Boot Interface)
- **Protocol:** `0x02` (Mouse)
- **Endpoint:** `0x81` (EP 1 IN, Interrupt, 8 bytes, bInterval = 1ms)
- **HID Report Descriptor Length:** 66 bytes
- **Report IDs:**
  - `Report ID 0x01`: Mouse buttons 1-5, X (16-bit rel), Y (16-bit rel), Wheel (8-bit rel)

### Interface 1: Configuration & Auxiliary Inputs
- **Class:** `0x03` (HID)
- **Subclass:** `0x00` (None)
- **Protocol:** `0x01` (Keyboard)
- **Endpoint:** `0x82` (EP 2 IN, Interrupt, 8 bytes, bInterval = 1ms)
- **HID Report Descriptor Length:** 140 bytes
- **Report IDs:**
  - `Report ID 0x03`: Standard Keyboard (modifiers + 6 key array)
  - `Report ID 0x02`: Consumer Control (media keys)
  - `Report ID 0x06`: Vendor Input Report (2 bytes payload)
  - `Report ID 0x07`: **Vendor Feature Report** (`Usage Page 0xFF01, Usage 0x01`, 7 bytes payload + 1 byte ID = 8 bytes total)
  - `Report ID 0x08`: System Control (Sleep/Wake/Power)

## 3. Communication Transport

- **Host-to-Device Transport:** HID Feature Reports on Interface 1 (`/dev/hidraw5`).
  - No OUT interrupt endpoints exist on either interface. All configuration packets are sent via control transfers (`Set_Report` on Endpoint 0) using HID Feature Report ID `0x07`.
  - Report length is fixed at **8 bytes** (`[0x07, CMD, DATA0, DATA1, DATA2, DATA3, DATA4, DATA5]`).
- **Device-to-Host Transport:** HID Feature Reports read via `HIDIOCGFEATURE(8)` with buffer initialized to Report ID `0x07`.
  - Returns current device state (e.g. Polling Rate in Byte 6).

## 4. Verification Evidence

- Physical mouse read test on `/dev/hidraw5`:
  ```
  HIDIOCGFEATURE(8) with Report ID 0x07 -> returned [0x07, 0x00, 0x68, 0x09, 0x00, 0x00, 0x03, 0x3b]
  ```
- OEM Windows Executable Analysis (`Gaming Mouse 3.0.exe`):
  - Statically confirmed `HidD_SetFeature` and `HidD_GetFeature` calls targeting fixed 8-byte buffer `[0x07, CMD, ...]`.
  - Target device table in binary maps VID `0x30fa`, PID `0x1440`, Usage Page `0xff01`, Usage `0x01`.
