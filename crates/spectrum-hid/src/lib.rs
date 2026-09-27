//! Cosmic Byte Spectrum HID Transport Layer
//!
//! Provides device discovery and communication over Linux `hidraw`
//! using HID Feature Reports on Report ID 0x07.

use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use log::{debug, info};
use thiserror::Error;

use spectrum_protocol::{
    cmd, PollingRate, ProtocolError, RgbMode, PRODUCT_ID, REPORT_ID, REPORT_LEN, VENDOR_ID,
};

// Linux HIDRAW ioctls for 8-byte feature reports
// Direction: _IOC_READ | _IOC_WRITE = 3 (0xC0000000)
// Size: 8 bytes (0x00080000)
// Type: 'H' = 0x48 (0x00004800)
// Number: 0x06 (HIDIOCSFEATURE) -> 0xC0084806
// Number: 0x07 (HIDIOCGFEATURE) -> 0xC0084807
const HIDIOCSFEATURE_8: libc::c_ulong = 0xC0084806;
const HIDIOCGFEATURE_8: libc::c_ulong = 0xC0084807;

#[derive(Error, Debug)]
pub enum HidError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Device not found: Cosmic Byte Spectrum (VID: 0x{VENDOR_ID:04x}, PID: 0x{PRODUCT_ID:04x})")]
    DeviceNotFound,

    #[error("Permission denied accessing '{0}'. Please install udev rules or run with sufficient privileges.")]
    PermissionDenied(PathBuf),

    #[error("HID ioctl failed: {0} (errno: {1})")]
    IoctlFailed(&'static str, i32),

    #[error("Invalid report response length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

/// Abstract HID transport interface
pub trait HidTransport: Send + Sync {
    /// Sends an 8-byte feature report
    fn send_feature_report(&mut self, data: &[u8; REPORT_LEN]) -> Result<(), HidError>;

    /// Reads an 8-byte feature report for the given report ID
    fn get_feature_report(&mut self, report_id: u8) -> Result<[u8; REPORT_LEN], HidError>;

    /// Device path or identifier if available
    fn device_path(&self) -> Option<&Path>;

    /// Writes an arbitrary-length data chunk to the device at the given offset using Command 0x18
    /// (with exact timing and finalize sequence disassembled from OEM application)
    fn write_chunk(&mut self, offset: u16, data: &[u8]) -> Result<(), HidError> {
        let chunk_len = data.len() as u8;
        for (i, &b) in data.iter().enumerate() {
            let pkt = spectrum_protocol::PacketEncoder::encode_chunk_write_byte(
                offset,
                i as u8,
                b,
                chunk_len,
            );
            self.send_feature_report(pkt.as_bytes())?;
            std::thread::sleep(std::time::Duration::from_millis(4));
        }

        // Finalize step 1 (flags = 0x09)
        let fin1 = spectrum_protocol::PacketEncoder::encode_chunk_finalize_step1(offset, chunk_len);
        self.send_feature_report(fin1.as_bytes())?;
        std::thread::sleep(std::time::Duration::from_millis(2));

        // Finalize step 2 (flags = 0x00)
        let fin2 = spectrum_protocol::PacketEncoder::encode_chunk_finalize_step2(offset);
        self.send_feature_report(fin2.as_bytes())?;
        std::thread::sleep(std::time::Duration::from_millis(2));

        Ok(())
    }

    /// Reads an arbitrary-length data chunk from the device at the given offset using Command 0x18
    /// (executes prep sequence VA 0x00413ad0, then read sequence VA 0x00413a00)
    fn read_chunk(&mut self, offset: u16, len: usize) -> Result<Vec<u8>, HidError> {
        let chunk_len = len as u8;
        // Phase 1: Prep sequence (VA 0x00413ad0)
        for i in 0..len {
            let pkt = spectrum_protocol::PacketEncoder::encode_chunk_read_prep(
                offset,
                i as u8,
                chunk_len,
            );
            self.send_feature_report(pkt.as_bytes())?;
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        let fin = spectrum_protocol::PacketEncoder::encode_chunk_finalize_step2(offset);
        self.send_feature_report(fin.as_bytes())?;
        std::thread::sleep(std::time::Duration::from_millis(2));

        // Phase 2: Read loop (VA 0x00413a00)
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            let pkt = spectrum_protocol::PacketEncoder::encode_chunk_read_byte(
                offset,
                i as u8,
                chunk_len,
            );
            self.send_feature_report(pkt.as_bytes())?;
            std::thread::sleep(std::time::Duration::from_millis(2));
            let buf = self.get_feature_report(REPORT_ID)?;
            // Byte 1 of the report contains the data byte
            result.push(buf[1]);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        let clear = spectrum_protocol::PacketEncoder::encode_chunk_finalize_step2(0);
        self.send_feature_report(clear.as_bytes())?;

        Ok(result)
    }
}

/// Linux `/dev/hidraw*` implementation
pub struct LinuxHidRaw {
    file: File,
    path: PathBuf,
}

impl LinuxHidRaw {
    /// Opens the specified hidraw path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, HidError> {
        let p = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&p)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    HidError::PermissionDenied(p.clone())
                } else {
                    HidError::Io(e)
                }
            })?;

        debug!("Opened hidraw device at {}", p.display());
        Ok(Self { file, path: p })
    }

    /// Automatically discovers and opens the Cosmic Byte Spectrum control interface
    pub fn open_auto() -> Result<Self, HidError> {
        if let Ok(override_path) = std::env::var("SPECTRUM_HIDRAW") {
            info!("Using override hidraw device: {}", override_path);
            return Self::open(override_path);
        }

        let discovered = find_spectrum_device()?;
        info!("Auto-discovered Spectrum mouse at {}", discovered.display());
        Self::open(discovered)
    }

    pub fn fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl HidTransport for LinuxHidRaw {
    fn send_feature_report(&mut self, data: &[u8; REPORT_LEN]) -> Result<(), HidError> {
        debug!("HIDIOCSFEATURE sending: {:02x?}", data);
        let ret = unsafe {
            libc::ioctl(self.file.as_raw_fd(), HIDIOCSFEATURE_8, data.as_ptr())
        };
        if ret < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return Err(HidError::IoctlFailed("HIDIOCSFEATURE", errno));
        }
        Ok(())
    }

    fn get_feature_report(&mut self, report_id: u8) -> Result<[u8; REPORT_LEN], HidError> {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = report_id;

        debug!("HIDIOCGFEATURE requesting report 0x{:02x}", report_id);
        let ret = unsafe {
            libc::ioctl(self.file.as_raw_fd(), HIDIOCGFEATURE_8, buf.as_mut_ptr())
        };
        if ret < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return Err(HidError::IoctlFailed("HIDIOCGFEATURE", errno));
        }
        debug!("HIDIOCGFEATURE received: {:02x?}", buf);
        Ok(buf)
    }

    fn device_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

/// Discovers the Cosmic Byte Spectrum vendor control interface in `/sys/class/hidraw`
pub fn find_spectrum_device() -> Result<PathBuf, HidError> {
    let hidraw_dir = Path::new("/sys/class/hidraw");
    if !hidraw_dir.exists() {
        return Err(HidError::DeviceNotFound);
    }

    let entries = fs::read_dir(hidraw_dir).map_err(HidError::Io)?;
    let mut fallback_candidate: Option<PathBuf> = None;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("hidraw") {
            continue;
        }

        let dev_sys_path = entry.path().join("device");
        let uevent_path = dev_sys_path.join("uevent");
        if !uevent_path.exists() {
            continue;
        }

        let uevent_content = match fs::read_to_string(&uevent_path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        // Format is HID_ID=0003:000030FA:00001440
        let target_hid_id = format!("HID_ID=0003:{:08X}:{:08X}", VENDOR_ID, PRODUCT_ID);
        if !uevent_content.to_uppercase().contains(&target_hid_id) {
            continue;
        }

        let dev_node = PathBuf::from(format!("/dev/{}", name_str));

        // Check if report_descriptor contains Vendor Page 0xFF01 or Report ID 0x07
        let report_desc_path = dev_sys_path.join("report_descriptor");
        if let Ok(desc_bytes) = fs::read(&report_desc_path) {
            // Usage Page (0xFF01) = 0x06, 0x01, 0xFF
            // Report ID (0x07) = 0x85, 0x07
            let has_vendor_page = desc_bytes.windows(3).any(|w| w == [0x06, 0x01, 0xff]);
            let has_report_id_7 = desc_bytes.windows(2).any(|w| w == [0x85, 0x07]);

            if has_vendor_page || has_report_id_7 {
                debug!("Identified primary Spectrum vendor interface at {}", dev_node.display());
                return Ok(dev_node);
            }
        }

        // Also check if HID_PHYS indicates input1 (Interface 1)
        if uevent_content.contains("/input1") {
            debug!("Identified Spectrum interface 1 at {}", dev_node.display());
            return Ok(dev_node);
        }

        fallback_candidate = Some(dev_node);
    }

    fallback_candidate.ok_or(HidError::DeviceNotFound)
}

/// In-memory mock HID transport for testing without physical mouse
pub struct MockHidTransport {
    pub current_dpi: u16,
    pub current_polling: PollingRate,
    pub current_rgb_mode: RgbMode,
    pub last_written_report: Option<[u8; REPORT_LEN]>,
    pub save_count: usize,
    pub eeprom: [u8; 4088],
}

impl Default for MockHidTransport {
    fn default() -> Self {
        let mut eeprom = [0u8; 4088];
        // Chunk 0x00: Hardware identification header
        eeprom[0..8].copy_from_slice(&[0x5A, 0x03, 0xA5, 0x40, 0x14, 0xFA, 0x30, 0x10]);
        // Chunk 0x08: Button mapping registers
        eeprom[8..16].copy_from_slice(&[0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x00, 0x00]);
        // Chunk 0x10: 6-stage DPI step index configuration
        eeprom[16..24].copy_from_slice(&[0x00, 0x00, 0x00, 0x38, 0x08, 0x48, 0xd1, 0x68]);
        // Chunk 0x18: Sensor & Polling rate configuration (1000 Hz)
        eeprom[24..32].copy_from_slice(&[0x40, 0x3f, 0x08, 0x00, 0xb3, 0xff, 0x00, 0x00]);

        Self {
            current_dpi: 2400,
            current_polling: PollingRate::Hz1000,
            current_rgb_mode: RgbMode::Neon,
            last_written_report: None,
            save_count: 0,
            eeprom,
        }
    }
}

impl MockHidTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl HidTransport for MockHidTransport {
    fn send_feature_report(&mut self, data: &[u8; REPORT_LEN]) -> Result<(), HidError> {
        self.last_written_report = Some(*data);
        if data[0] != REPORT_ID {
            return Err(HidError::Protocol(ProtocolError::InvalidReportId(data[0])));
        }

        match data[1] {
            cmd::SET_POLLING => {
                self.current_polling = PollingRate::from_hw_code(data[2])?;
            }
            cmd::SET_LED => {
                let mode_num = (data[3] >> 4) & 0x0f;
                self.current_rgb_mode = RgbMode::from_u8(mode_num)?;
            }
            cmd::ACCESS_REGISTER => {
                let flags = data[2];
                let byte_idx = data[3] as usize;
                let offset = (data[4] as usize) | ((data[5] as usize) << 8);
                let addr = offset + byte_idx;
                if flags == 0x03 && addr < self.eeprom.len() {
                    self.eeprom[addr] = data[6];
                }
            }
            cmd::SAVE_TO_FLASH => {
                self.save_count += 1;
            }
            _ => {}
        }

        Ok(())
    }

    fn get_feature_report(&mut self, report_id: u8) -> Result<[u8; REPORT_LEN], HidError> {
        if report_id != REPORT_ID {
            return Err(HidError::Protocol(ProtocolError::InvalidReportId(report_id)));
        }

        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = 0x00;
        let dpi_bytes = self.current_dpi.to_le_bytes();
        buf[2] = dpi_bytes[0];
        buf[3] = dpi_bytes[1];
        buf[4] = 0x00;
        buf[5] = 0x00;
        buf[6] = self.current_polling.hw_code();
        buf[7] = 0x3b;

        Ok(buf)
    }

    fn device_path(&self) -> Option<&Path> {
        None
    }

    fn write_chunk(&mut self, offset: u16, data: &[u8]) -> Result<(), HidError> {
        let off = offset as usize;
        let end = (off + data.len()).min(self.eeprom.len());
        if off < self.eeprom.len() {
            self.eeprom[off..end].copy_from_slice(&data[..end - off]);
        }
        Ok(())
    }

    fn read_chunk(&mut self, offset: u16, len: usize) -> Result<Vec<u8>, HidError> {
        let off = offset as usize;
        let end = (off + len).min(self.eeprom.len());
        if off < self.eeprom.len() {
            Ok(self.eeprom[off..end].to_vec())
        } else {
            Ok(vec![0; len])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_transport_read_status() {
        let mut mock = MockHidTransport::new();
        let report = mock.get_feature_report(REPORT_ID).unwrap();
        assert_eq!(report[0], 0x07);
        let (dpi, rate) = spectrum_protocol::PacketDecoder::decode_status(&report).unwrap();
        assert_eq!(dpi, 2400);
        assert_eq!(rate, PollingRate::Hz1000);
    }

    #[test]
    fn test_mock_transport_write_rgb() {
        let mut mock = MockHidTransport::new();
        let config = spectrum_protocol::RgbConfig {
            mode: RgbMode::StaticLight,
            ..Default::default()
        };
        let packet = spectrum_protocol::PacketEncoder::encode_rgb(&config);
        mock.send_feature_report(packet.as_bytes()).unwrap();
        assert_eq!(mock.current_rgb_mode, RgbMode::StaticLight);
    }

    #[test]
    fn test_mock_transport_chunk_access() {
        let mut mock = MockHidTransport::new();
        // Check initial Chunk 0x00 header
        let chunk0 = mock.read_chunk(0x00, 8).unwrap();
        assert_eq!(chunk0, vec![0x5A, 0x03, 0xA5, 0x40, 0x14, 0xFA, 0x30, 0x10]);

        // Write custom Chunk 0x10
        let new_chunk10 = [0x00, 0x00, 0x00, 0x38, 0x08, 0x8c, 0x41, 0x68];
        mock.write_chunk(0x10, &new_chunk10).unwrap();

        let read_back = mock.read_chunk(0x10, 8).unwrap();
        assert_eq!(read_back, new_chunk10);
    }
}
