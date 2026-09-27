//! Cosmic Byte Spectrum Core Service Layer
//!
//! Provides high-level device management, settings synchronization,
//! and profile management for the Cosmic Byte Spectrum mouse.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use log::info;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use spectrum_hid::{HidError, HidTransport, LinuxHidRaw};
use spectrum_protocol::{
    dpi_to_step_index, Button, ButtonAction, DpiStage, PacketEncoder, PollingRate, ProtocolError,
    RgbColor, RgbConfig, RgbMode, PRODUCT_ID, REPORT_ID, VENDOR_ID,
};

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("HID transport error: {0}")]
    Hid(#[from] HidError),

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("DPI verification failed: wrote {target} DPI, hardware EEPROM read back {actual} DPI")]
    DpiVerificationFailed { target: u16, actual: u16 },

    #[error("Polling rate verification failed: wrote hw code 0x{target:02x}, hardware EEPROM read back 0x{actual:02x}")]
    PollingRateVerificationFailed { target: u8, actual: u8 },

    #[error("Profile error: {0}")]
    Profile(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Information identifying the mouse hardware
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub hidraw_path: Option<String>,
    pub interface: u8,
    pub connected: bool,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            name: "Cosmic Byte Spectrum RGB Gaming Mouse".to_string(),
            vendor_id: VENDOR_ID,
            product_id: PRODUCT_ID,
            hidraw_path: None,
            interface: 1,
            connected: false,
        }
    }
}

/// Comprehensive settings snapshot of the mouse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseSettings {
    pub active_dpi: u16,
    pub polling_rate: PollingRate,
    pub stages: Vec<DpiStage>,
    pub rgb: RgbConfig,
    pub buttons: HashMap<Button, ButtonAction>,
}

impl Default for MouseSettings {
    fn default() -> Self {
        let default_stages = vec![
            DpiStage {
                stage_index: 0,
                dpi: 800,
                color: RgbColor::new(255, 0, 0),
                enabled: true,
            },
            DpiStage {
                stage_index: 1,
                dpi: 1600,
                color: RgbColor::new(0, 255, 0),
                enabled: true,
            },
            DpiStage {
                stage_index: 2,
                dpi: 2400,
                color: RgbColor::new(0, 0, 255),
                enabled: true,
            },
            DpiStage {
                stage_index: 3,
                dpi: 3200,
                color: RgbColor::new(255, 255, 0),
                enabled: true,
            },
            DpiStage {
                stage_index: 4,
                dpi: 4800,
                color: RgbColor::new(0, 255, 255),
                enabled: true,
            },
            DpiStage {
                stage_index: 5,
                dpi: 6400,
                color: RgbColor::new(255, 0, 255),
                enabled: true,
            },
        ];

        let mut buttons = HashMap::new();
        buttons.insert(Button::Left, ButtonAction::LeftClick);
        buttons.insert(Button::Middle, ButtonAction::MiddleClick);
        buttons.insert(Button::Right, ButtonAction::RightClick);
        buttons.insert(Button::Back, ButtonAction::Back);
        buttons.insert(Button::Forward, ButtonAction::Forward);
        buttons.insert(Button::DpiLoop, ButtonAction::DpiLoop);

        Self {
            active_dpi: 1600,
            polling_rate: PollingRate::Hz1000,
            stages: default_stages,
            rgb: RgbConfig::default(),
            buttons,
        }
    }
}

/// Abstract high-level mouse controller trait
pub trait SpectrumDevice {
    fn identify(&self) -> Result<DeviceInfo, CoreError>;
    fn get_settings(&mut self) -> Result<MouseSettings, CoreError>;
    fn set_dpi(&mut self, dpi: u16) -> Result<(), CoreError>;
    fn set_dpi_stage(&mut self, stage: DpiStage) -> Result<(), CoreError>;
    fn set_polling_rate(&mut self, rate: PollingRate) -> Result<(), CoreError>;
    fn set_rgb(&mut self, config: RgbConfig) -> Result<(), CoreError>;
    fn set_button(&mut self, button: Button, action: ButtonAction) -> Result<(), CoreError>;
    fn save_to_onboard(&mut self) -> Result<(), CoreError>;
    fn apply_profile(&mut self, profile: &Profile) -> Result<(), CoreError>;
}

/// High-level device controller managing HID packets and state
pub struct SpectrumController<T: HidTransport> {
    transport: T,
    cached_settings: MouseSettings,
}

impl SpectrumController<LinuxHidRaw> {
    /// Connects to a physical Spectrum mouse automatically
    pub fn connect_auto() -> Result<Self, CoreError> {
        let transport = LinuxHidRaw::open_auto()?;
        Ok(Self::new(transport))
    }

    /// Connects to a specific `/dev/hidrawX` path
    pub fn connect_path<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let transport = LinuxHidRaw::open(path)?;
        Ok(Self::new(transport))
    }
}

impl<T: HidTransport> SpectrumController<T> {
    pub fn new(transport: T) -> Self {
        let cached = StateManager::load_state();
        Self {
            transport,
            cached_settings: cached,
        }
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl<T: HidTransport> SpectrumDevice for SpectrumController<T> {
    fn identify(&self) -> Result<DeviceInfo, CoreError> {
        let path = self
            .transport
            .device_path()
            .map(|p| p.to_string_lossy().to_string());
        Ok(DeviceInfo {
            name: "Cosmic Byte Spectrum RGB Gaming Mouse".to_string(),
            vendor_id: VENDOR_ID,
            product_id: PRODUCT_ID,
            hidraw_path: path,
            interface: 1,
            connected: true,
        })
    }

    fn get_settings(&mut self) -> Result<MouseSettings, CoreError> {
        // 1. Read Chunk 0x10 to retrieve true hardware DPI stages from mouse EEPROM
        if let Ok(chunk10) = self.transport.read_chunk(0x10, 8) {
            if chunk10.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&chunk10);
                if let Ok(dpis) = PacketEncoder::decode_dpi_resolution_chunk(&arr) {
                    for (i, &d) in dpis.iter().enumerate() {
                        if i < self.cached_settings.stages.len() {
                            self.cached_settings.stages[i].dpi = d;
                        }
                    }
                    if !self.cached_settings.stages.is_empty() {
                        self.cached_settings.active_dpi = self.cached_settings.stages[0].dpi;
                    }
                }
            }
        }

        // 2. Read Chunk 0x18 to retrieve true hardware polling rate
        if let Ok(chunk18) = self.transport.read_chunk(0x18, 8) {
            if chunk18.len() >= 7 {
                if let Ok(rate) = PollingRate::from_hw_code(chunk18[6]) {
                    self.cached_settings.polling_rate = rate;
                }
            }
        }

        // 3. Status report query to keep device connection alive and check status
        let _ = self.transport.get_feature_report(REPORT_ID);

        StateManager::save_state(&self.cached_settings);
        Ok(self.cached_settings.clone())
    }

    fn set_dpi(&mut self, dpi: u16) -> Result<(), CoreError> {
        dpi_to_step_index(dpi)?; // validate support

        // Update active stage 0 with target DPI
        if let Some(stage0) = self.cached_settings.stages.first_mut() {
            stage0.dpi = dpi;
            stage0.enabled = true;
        }

        // 1. Encode and write Chunk 0x10 (DPI step indices for all 6 stages)
        let chunk10 = PacketEncoder::encode_dpi_resolution_chunk(&self.cached_settings.stages)?;
        self.transport.write_chunk(0x10, &chunk10)?;

        // 2. Configure DPI stage indicator LED color for stage 0
        if let Some(stage0) = self.cached_settings.stages.first() {
            let led_pkt = PacketEncoder::encode_dpi_stage(stage0)?;
            self.transport.send_feature_report(led_pkt.as_bytes())?;
        }

        // 3. Send Command 0x15 active stages queue
        let stages_pkt = PacketEncoder::encode_active_stages(&self.cached_settings.stages);
        self.transport.send_feature_report(stages_pkt.as_bytes())?;

        // 4. Send Command 0x16 feature enable flags (0x07)
        let flags_pkt = PacketEncoder::encode_flags(0x07);
        self.transport.send_feature_report(flags_pkt.as_bytes())?;

        // 5. Send Command 0x20 to commit to onboard EEPROM/flash
        let save_pkt = PacketEncoder::encode_save();
        self.transport.send_feature_report(save_pkt.as_bytes())?;

        // 6. Read-back verification from hardware EEPROM
        let read_chunk = self.transport.read_chunk(0x10, 8)?;
        if read_chunk.len() == 8 {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&read_chunk);
            let decoded = PacketEncoder::decode_dpi_resolution_chunk(&arr)?;
            let verified_dpi = decoded[0];
            if verified_dpi != dpi {
                return Err(CoreError::DpiVerificationFailed {
                    target: dpi,
                    actual: verified_dpi,
                });
            }
        }

        self.cached_settings.active_dpi = dpi;
        StateManager::save_state(&self.cached_settings);
        info!("Set and hardware-verified active DPI to {}", dpi);
        Ok(())
    }

    fn set_dpi_stage(&mut self, stage: DpiStage) -> Result<(), CoreError> {
        let idx = stage.stage_index as usize;
        if idx >= 6 {
            return Err(CoreError::Protocol(ProtocolError::InvalidStageIndex(stage.stage_index)));
        }

        if idx < self.cached_settings.stages.len() {
            self.cached_settings.stages[idx] = stage.clone();
        }

        // 1. Encode and write Chunk 0x10
        let chunk10 = PacketEncoder::encode_dpi_resolution_chunk(&self.cached_settings.stages)?;
        self.transport.write_chunk(0x10, &chunk10)?;

        // 2. Configure DPI stage indicator LED color
        let led_pkt = PacketEncoder::encode_dpi_stage(&stage)?;
        self.transport.send_feature_report(led_pkt.as_bytes())?;

        // 3. Send Command 0x15 active stages queue
        let stages_pkt = PacketEncoder::encode_active_stages(&self.cached_settings.stages);
        self.transport.send_feature_report(stages_pkt.as_bytes())?;

        // 4. Send Command 0x16 feature enable flags (0x07)
        let flags_pkt = PacketEncoder::encode_flags(0x07);
        self.transport.send_feature_report(flags_pkt.as_bytes())?;

        // 5. Send Command 0x20 commit to flash
        let save_pkt = PacketEncoder::encode_save();
        self.transport.send_feature_report(save_pkt.as_bytes())?;

        // 6. Read-back verification
        let read_chunk = self.transport.read_chunk(0x10, 8)?;
        if read_chunk.len() == 8 {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&read_chunk);
            let decoded = PacketEncoder::decode_dpi_resolution_chunk(&arr)?;
            let verified_dpi = decoded[idx];
            if verified_dpi != stage.dpi {
                return Err(CoreError::DpiVerificationFailed {
                    target: stage.dpi,
                    actual: verified_dpi,
                });
            }
        }

        if idx == 0 {
            self.cached_settings.active_dpi = stage.dpi;
        }

        StateManager::save_state(&self.cached_settings);
        info!("Updated and hardware-verified DPI stage {}", idx);
        Ok(())
    }

    fn set_polling_rate(&mut self, rate: PollingRate) -> Result<(), CoreError> {
        // Update Chunk 0x18 Byte 6 (Sensor & Polling Rate configuration)
        // Disassembly VA 0x0041bbe6: Polling rate is strictly configured via Chunk 0x18 Byte 6.
        // Command 0x11 is an Office/Game mode toggle (0x00 or 0x02), not polling rate.
        // Sending 0x11 with polling hardware codes crashes the mouse MCU sensor interrupt loop and locks the cursor!
        let mut chunk_18 = self.transport.read_chunk(0x18, 8).unwrap_or_else(|_| {
            vec![0x40, 0x3f, 0x08, 0x00, 0xb3, 0xff, 0x00, 0x00]
        });
        if chunk_18.len() >= 8 {
            chunk_18[6] = rate.hw_code();
            self.transport.write_chunk(0x18, &chunk_18)?;
        }

        // Commit to flash
        let save_pkt = PacketEncoder::encode_save();
        self.transport.send_feature_report(save_pkt.as_bytes())?;

        // Readback verification from Chunk 0x18 Byte 6
        let read_chunk = self.transport.read_chunk(0x18, 8)?;
        if read_chunk.len() >= 7 {
            let verified_code = read_chunk[6];
            if verified_code != rate.hw_code() {
                return Err(CoreError::PollingRateVerificationFailed {
                    target: rate.hw_code(),
                    actual: verified_code,
                });
            }
        }

        self.cached_settings.polling_rate = rate;
        StateManager::save_state(&self.cached_settings);
        info!("Set and hardware-verified polling rate to {}", rate);
        Ok(())
    }

    fn set_rgb(&mut self, config: RgbConfig) -> Result<(), CoreError> {
        // 1. Send Command 0x14: Program active LED color nibbles into mouse hardware color table
        let led_pkt = PacketEncoder::encode_dpi_stage_led(0, &config.color)?;
        self.transport.send_feature_report(led_pkt.as_bytes())?;

        // Stabilization pause (matches OEM driver timing)
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 2. Send Command 0x13: Configure RGB effect mode, speed, brightness, and flags
        let rgb_pkt = PacketEncoder::encode_rgb(&config);
        self.transport.send_feature_report(rgb_pkt.as_bytes())?;

        // Stabilization pause
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 3. Send Command 0x20: Save / commit configuration into onboard non-volatile EEPROM/flash
        let save_pkt = PacketEncoder::encode_save();
        self.transport.send_feature_report(save_pkt.as_bytes())?;

        self.cached_settings.rgb = config;
        StateManager::save_state(&self.cached_settings);
        info!("Applied and committed RGB configuration: {}", self.cached_settings.rgb.mode);
        Ok(())
    }

    fn set_button(&mut self, button: Button, action: ButtonAction) -> Result<(), CoreError> {
        let packet = PacketEncoder::encode_button(button, action);
        self.transport.send_feature_report(packet.as_bytes())?;
        let save_pkt = PacketEncoder::encode_save();
        self.transport.send_feature_report(save_pkt.as_bytes())?;
        self.cached_settings.buttons.insert(button, action);
        StateManager::save_state(&self.cached_settings);
        info!("Configured and committed Button {:?} to {:?}", button, action);
        Ok(())
    }

    fn save_to_onboard(&mut self) -> Result<(), CoreError> {
        let packet = PacketEncoder::encode_save();
        self.transport.send_feature_report(packet.as_bytes())?;
        StateManager::save_state(&self.cached_settings);
        info!("Settings committed to mouse onboard flash memory");
        Ok(())
    }

    fn apply_profile(&mut self, profile: &Profile) -> Result<(), CoreError> {
        info!("Applying profile '{}'", profile.name);

        // Apply Polling Rate
        let rate = PollingRate::from_hz(profile.polling_rate)?;
        self.set_polling_rate(rate)?;

        // Apply DPI stages
        for (i, &dpi) in profile.dpi_stages.iter().enumerate().take(6) {
            let stage = DpiStage {
                stage_index: i as u8,
                dpi,
                color: RgbColor::new(255, 0, 0),
                enabled: true,
            };
            self.set_dpi_stage(stage)?;
        }

        // Apply active DPI
        self.set_dpi(profile.active_dpi)?;

        // Apply RGB
        self.set_rgb(profile.rgb.clone())?;

        // Apply Buttons
        for (btn, act) in &profile.buttons {
            self.set_button(*btn, *act)?;
        }

        Ok(())
    }
}

/// Host-side profile representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub dpi_stages: Vec<u16>,
    pub active_dpi: u16,
    pub polling_rate: u16,
    pub rgb: RgbConfig,
    pub buttons: Vec<(Button, ButtonAction)>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: Some("Balanced daily driver profile".to_string()),
            dpi_stages: vec![800, 1600, 2400, 3200, 4800, 6400],
            active_dpi: 1600,
            polling_rate: 1000,
            rgb: RgbConfig::default(),
            buttons: vec![
                (Button::Left, ButtonAction::LeftClick),
                (Button::Middle, ButtonAction::MiddleClick),
                (Button::Right, ButtonAction::RightClick),
                (Button::Back, ButtonAction::Back),
                (Button::Forward, ButtonAction::Forward),
                (Button::DpiLoop, ButtonAction::DpiLoop),
            ],
        }
    }
}

impl Profile {
    pub fn gaming() -> Self {
        Self {
            name: "gaming".to_string(),
            description: Some("Competitive FPS configuration (800 DPI, 1000 Hz)".to_string()),
            dpi_stages: vec![400, 800, 1200, 1600],
            active_dpi: 800,
            polling_rate: 1000,
            rgb: RgbConfig {
                mode: RgbMode::StaticLight,
                color: RgbColor::new(255, 0, 0),
                brightness: 3,
                speed: 2,
                direction: false,
                symmetry: true,
            },
            buttons: vec![
                (Button::Left, ButtonAction::LeftClick),
                (Button::Middle, ButtonAction::MiddleClick),
                (Button::Right, ButtonAction::RightClick),
                (Button::Back, ButtonAction::Back),
                (Button::Forward, ButtonAction::Forward),
                (Button::DpiLoop, ButtonAction::DpiLoop),
            ],
        }
    }

    pub fn office() -> Self {
        Self {
            name: "office".to_string(),
            description: Some("Work & productivity profile (1600 DPI, 500 Hz, Subtle RGB)".to_string()),
            dpi_stages: vec![800, 1200, 1600, 2400],
            active_dpi: 1600,
            polling_rate: 500,
            rgb: RgbConfig {
                mode: RgbMode::SingleBreath,
                color: RgbColor::new(0, 128, 255),
                brightness: 1,
                speed: 1,
                direction: false,
                symmetry: true,
            },
            buttons: vec![
                (Button::Left, ButtonAction::LeftClick),
                (Button::Middle, ButtonAction::MiddleClick),
                (Button::Right, ButtonAction::RightClick),
                (Button::Back, ButtonAction::Back),
                (Button::Forward, ButtonAction::Forward),
                (Button::DpiLoop, ButtonAction::DpiLoop),
            ],
        }
    }
}

/// Profile management utilities
pub struct ProfileManager;

impl ProfileManager {
    /// Returns path to profile directory (~/.config/cosmic-byte-spectrum/profiles)
    pub fn profiles_dir() -> PathBuf {
        let base = dirs_next_config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("cosmic-byte-spectrum").join("profiles")
    }

    /// Lists all available profiles
    pub fn list() -> Result<Vec<String>, CoreError> {
        let dir = Self::profiles_dir();
        if !dir.exists() {
            return Ok(vec![
                "default".to_string(),
                "gaming".to_string(),
                "office".to_string(),
            ]);
        }

        let mut list = Vec::new();
        for entry in fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    list.push(stem.to_string());
                }
            }
        }
        if list.is_empty() {
            list.push("default".to_string());
            list.push("gaming".to_string());
            list.push("office".to_string());
        }
        list.sort();
        Ok(list)
    }

    /// Loads a profile by name
    pub fn load(name: &str) -> Result<Profile, CoreError> {
        let dir = Self::profiles_dir();
        let file_path = dir.join(format!("{}.toml", name));

        if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            let profile: Profile = toml::from_str(&content)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            return Ok(profile);
        }

        // Fallbacks for built-in profiles
        match name {
            "gaming" => Ok(Profile::gaming()),
            "office" => Ok(Profile::office()),
            "default" => Ok(Profile::default()),
            _ => Err(CoreError::Profile(format!("Profile '{}' not found", name))),
        }
    }

    /// Saves a profile to disk
    pub fn save(profile: &Profile) -> Result<PathBuf, CoreError> {
        let dir = Self::profiles_dir();
        fs::create_dir_all(&dir)?;
        let file_path = dir.join(format!("{}.toml", profile.name));
        let content = toml::to_string_pretty(profile)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        fs::write(&file_path, content)?;
        info!("Saved profile to {}", file_path.display());
        Ok(file_path)
    }
}

/// Active settings persistence manager
pub struct StateManager;

impl StateManager {
    pub fn state_file() -> PathBuf {
        let base = dirs_next_config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("cosmic-byte-spectrum").join("state.json")
    }

    pub fn load_state() -> MouseSettings {
        let path = Self::state_file();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<MouseSettings>(&content) {
                    return settings;
                }
            }
        }
        MouseSettings::default()
    }

    pub fn save_state(settings: &MouseSettings) {
        let path = Self::state_file();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(settings) {
            let _ = fs::write(&path, content);
        }
    }
}

fn dirs_next_config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg));
    }
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectrum_hid::MockHidTransport;

    #[test]
    fn test_spectrum_controller_with_mock() {
        let mock = MockHidTransport::new();
        let mut controller = SpectrumController::new(mock);

        let info = controller.identify().unwrap();
        assert_eq!(info.vendor_id, 0x30fa);
        assert_eq!(info.product_id, 0x1440);

        let settings = controller.get_settings().unwrap();
        assert_eq!(settings.active_dpi, 800);
        assert_eq!(settings.polling_rate, PollingRate::Hz1000);

        // Hardware-verified DPI change
        controller.set_dpi(1600).unwrap();
        let updated_dpi = controller.get_settings().unwrap();
        assert_eq!(updated_dpi.active_dpi, 1600);
        assert_eq!(updated_dpi.stages[0].dpi, 1600);

        // Polling rate change
        controller.set_polling_rate(PollingRate::Hz500).unwrap();
        let updated = controller.get_settings().unwrap();
        assert_eq!(updated.polling_rate, PollingRate::Hz500);

        // RGB change with dual-command sequence
        let rgb_cfg = RgbConfig {
            mode: RgbMode::StaticLight,
            brightness: 3,
            speed: 2,
            color: RgbColor::new(255, 0, 0),
            direction: false,
            symmetry: true,
        };
        controller.set_rgb(rgb_cfg.clone()).unwrap();
        assert_eq!(controller.cached_settings.rgb, rgb_cfg);

        controller.save_to_onboard().unwrap();
    }

    #[test]
    fn test_profile_serialize_deserialize() {
        let profile = Profile::gaming();
        let toml_str = toml::to_string(&profile).unwrap();
        let loaded: Profile = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.name, "gaming");
        assert_eq!(loaded.active_dpi, 800);
        assert_eq!(loaded.polling_rate, 1000);
    }
}
