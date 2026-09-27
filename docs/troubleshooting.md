# Cosmic Byte Spectrum — Linux Troubleshooting Guide

## 1. Device Not Detected
If `spectrumctl list` reports that no device is found:

1. Verify USB connection:
   ```bash
   lsusb -d 30fa:1440
   ```
   You should see:
   ```
   Bus XXX Device YYY: ID 30fa:1440 INSTANT USB GAMING MOUSE
   ```

2. Check hidraw device nodes:
   ```bash
   ls -l /dev/hidraw*
   ```
   The Cosmic Byte Spectrum creates two hidraw nodes (typically `/dev/hidraw4` and `/dev/hidraw5`). The vendor configuration interface is on Interface 1 (`/dev/hidraw5`).

3. Check sysfs descriptors:
   ```bash
   grep -H . /sys/class/hidraw/*/device/uevent | grep -i 30fa
   ```

---

## 2. Permission Denied Errors
If you see an error like:
```
Permission denied accessing '/dev/hidraw5'. Please install udev rules or run with sufficient privileges.
```

Install the udev rule to grant access to local desktop users:
```bash
sudo cp packaging/udev/99-cosmic-byte-spectrum.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```
After reloading rules, disconnect and reconnect the mouse USB cable.

---

## 3. Cursor Still Works Normally
Interface 0 is the standard boot mouse interface handled by Linux kernel `hid-generic`. `spectrumctl` and `spectrum-gui` communicate strictly via HID Feature reports over Interface 1. The cursor, mouse buttons, and scroll wheel remain completely functional at all times.

---

## 4. Manual Device Override
If your Linux system assigns the control interface to a non-standard path, you can specify it directly:
```bash
spectrumctl --device /dev/hidrawX get
# or via environment variable:
export SPECTRUM_HIDRAW=/dev/hidrawX
spectrumctl get
```
