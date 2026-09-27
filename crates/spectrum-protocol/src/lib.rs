//! Cosmic Byte Spectrum Protocol Implementation
//!
//! Encodes and decodes HID Feature Report packets for the Cosmic Byte Spectrum
//! RGB gaming mouse (VID 0x30FA, PID 0x1440).

use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// USB Vendor ID for Instant Microelectronics / Cosmic Byte Spectrum
pub const VENDOR_ID: u16 = 0x30fa;

/// USB Product ID for Spectrum gaming mouse
pub const PRODUCT_ID: u16 = 0x1440;

/// Target HID Feature Report ID
pub const REPORT_ID: u8 = 0x07;

/// Fixed feature report length in bytes (1 byte report ID + 7 payload bytes)
pub const REPORT_LEN: usize = 8;

/// Protocol command opcodes
pub mod cmd {
    pub const SET_BUTTON: u8 = 0x10;
    pub const SET_POLLING: u8 = 0x11;
    pub const SET_LED: u8 = 0x13;
    pub const SET_DPI_STAGE: u8 = 0x14;
    pub const SET_ACTIVE_STAGES: u8 = 0x15;
    pub const SET_FLAGS: u8 = 0x16;
    pub const SET_SENSITIVITY: u8 = 0x17;
    pub const ACCESS_REGISTER: u8 = 0x18;
    pub const SAVE_TO_FLASH: u8 = 0x20;
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("Invalid packet length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("Invalid report ID: expected 0x07, got 0x{0:02x}")]
    InvalidReportId(u8),

    #[error("Unsupported DPI value: {0}. Supported: 200..12800")]
    InvalidDpi(u16),

    #[error("Invalid DPI stage index: {0}. Must be 0..5")]
    InvalidStageIndex(u8),

    #[error("Unsupported polling rate: {0} Hz. Supported: 125, 250, 500, 1000")]
    InvalidPollingRate(u16),

    #[error("Invalid RGB mode: {0}")]
    InvalidRgbMode(u8),

    #[error("Invalid hex color: {0}")]
    InvalidHexColor(String),

    #[error("Invalid button code: {0}")]
    InvalidButton(u8),

    #[error("Invalid action code: {0}")]
    InvalidAction(u8),
}

/// Hardware DPI steps supported by the sensor/firmware
pub const VALID_DPI_STEPS: [u16; 23] = [
    200, 400, 600, 800, 1000, 1200, 1400, 1600, 1800, 2000,
    2400, 3200, 4000, 4800, 5600, 6400, 7200, 8000, 8800, 9600,
    10400, 11200, 12800,
];

/// Converts human DPI (e.g. 1600) to hardware step index (1..24)
pub fn dpi_to_step_index(dpi: u16) -> Result<u8, ProtocolError> {
    match dpi {
        200 => Ok(1),
        400 => Ok(2),
        600 => Ok(3),
        800 => Ok(4),
        1000 => Ok(5),
        1200 => Ok(6),
        1400 => Ok(7),
        1600 => Ok(8),
        1800 => Ok(9),
        2000 => Ok(10),
        2400 => Ok(12),
        3200 => Ok(13),
        4000 => Ok(14),
        4800 => Ok(15),
        5600 => Ok(16),
        6400 => Ok(17),
        7200 => Ok(18),
        8000 => Ok(19),
        8800 => Ok(20),
        9600 => Ok(21),
        10400 => Ok(22),
        11200 => Ok(23),
        12800 => Ok(24),
        _ => Err(ProtocolError::InvalidDpi(dpi)),
    }
}

/// Converts hardware step index (1..24) to human DPI
pub fn step_index_to_dpi(step: u8) -> Result<u16, ProtocolError> {
    match step {
        1 => Ok(200),
        2 => Ok(400),
        3 => Ok(600),
        4 => Ok(800),
        5 => Ok(1000),
        6 => Ok(1200),
        7 => Ok(1400),
        8 => Ok(1600),
        9 => Ok(1800),
        10 => Ok(2000),
        11 | 12 => Ok(2400),
        13 => Ok(3200),
        14 => Ok(4000),
        15 => Ok(4800),
        16 => Ok(5600),
        17 => Ok(6400),
        18 => Ok(7200),
        19 => Ok(8000),
        20 => Ok(8800),
        21 => Ok(9600),
        22 => Ok(10400),
        23 => Ok(11200),
        24 => Ok(12800),
        _ => Err(ProtocolError::InvalidDpi(step as u16)),
    }
}

/// Supported USB polling rates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PollingRate {
    Hz125,
    Hz250,
    Hz500,
    Hz1000,
}

impl PollingRate {
    pub fn hz(&self) -> u16 {
        match self {
            PollingRate::Hz125 => 125,
            PollingRate::Hz250 => 250,
            PollingRate::Hz500 => 500,
            PollingRate::Hz1000 => 1000,
        }
    }

    pub fn hw_code(&self) -> u8 {
        match self {
            PollingRate::Hz125 => 0x07,
            PollingRate::Hz250 => 0x03,
            PollingRate::Hz500 => 0x01,
            PollingRate::Hz1000 => 0x00,
        }
    }

    pub fn from_hz(hz: u16) -> Result<Self, ProtocolError> {
        match hz {
            125 => Ok(PollingRate::Hz125),
            250 => Ok(PollingRate::Hz250),
            500 => Ok(PollingRate::Hz500),
            1000 => Ok(PollingRate::Hz1000),
            _ => Err(ProtocolError::InvalidPollingRate(hz)),
        }
    }

    pub fn from_hw_code(code: u8) -> Result<Self, ProtocolError> {
        match code & 0x07 {
            0x00 => Ok(PollingRate::Hz1000),
            0x01 => Ok(PollingRate::Hz500),
            0x03 => Ok(PollingRate::Hz250),
            0x07 => Ok(PollingRate::Hz125),
            _ => {
                // If code is in range 0..3 (UI code)
                match code & 0x03 {
                    0 => Ok(PollingRate::Hz125),
                    1 => Ok(PollingRate::Hz250),
                    2 => Ok(PollingRate::Hz500),
                    3 => Ok(PollingRate::Hz1000),
                    _ => Err(ProtocolError::InvalidPollingRate(code as u16)),
                }
            }
        }
    }
}

impl fmt::Display for PollingRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.hz())
    }
}

/// 8-bit RGB color with 4-bit nibble conversion for firmware storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Result<Self, ProtocolError> {
        let clean = hex.trim().trim_start_matches('#');
        if clean.len() != 6 {
            return Err(ProtocolError::InvalidHexColor(hex.to_string()));
        }
        let r = u8::from_str_radix(&clean[0..2], 16)
            .map_err(|_| ProtocolError::InvalidHexColor(hex.to_string()))?;
        let g = u8::from_str_radix(&clean[2..4], 16)
            .map_err(|_| ProtocolError::InvalidHexColor(hex.to_string()))?;
        let b = u8::from_str_radix(&clean[4..6], 16)
            .map_err(|_| ProtocolError::InvalidHexColor(hex.to_string()))?;
        Ok(Self { r, g, b })
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Converts 8-bit channel to 4-bit nibble according to firmware algorithm
    pub fn to_nibbles(&self) -> (u8, u8, u8) {
        let r_nib = ((255u16 - self.r as u16 + 8) / 16).min(15) as u8;
        let g_nib = ((255u16 - self.g as u16 + 8) / 16).min(15) as u8;
        let b_nib = ((255u16 - self.b as u16 + 8) / 16).min(15) as u8;
        (r_nib, g_nib, b_nib)
    }

    /// Converts 4-bit nibbles back to 8-bit channel
    pub fn from_nibbles(r_nib: u8, g_nib: u8, b_nib: u8) -> Self {
        let r = (255 - (r_nib.min(15) as u16 * 16)).min(255) as u8;
        let g = (255 - (g_nib.min(15) as u16 * 16)).min(255) as u8;
        let b = (255 - (b_nib.min(15) as u16 * 16)).min(255) as u8;
        Self { r, g, b }
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// 16 Hardware RGB lighting effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RgbMode {
    DpiBreathing = 1,
    CycleBreathing = 2,
    StaticLight = 3,
    FlowingWater = 4,
    MonoWater = 5,
    CometStreak = 6,
    Neon = 7,
    Ambilight = 8,
    Flicker = 9,
    StarTrek = 10,
    Ripple = 11,
    Enraptured = 12,
    ButtonResponse = 13,
    Off = 14,
    SingleBreath = 15,
    CycleColor = 16,
}

impl RgbMode {
    pub fn from_u8(val: u8) -> Result<Self, ProtocolError> {
        match val {
            1 => Ok(RgbMode::DpiBreathing),
            2 => Ok(RgbMode::CycleBreathing),
            3 => Ok(RgbMode::StaticLight),
            4 => Ok(RgbMode::FlowingWater),
            5 => Ok(RgbMode::MonoWater),
            6 => Ok(RgbMode::CometStreak),
            7 => Ok(RgbMode::Neon),
            8 => Ok(RgbMode::Ambilight),
            9 => Ok(RgbMode::Flicker),
            10 => Ok(RgbMode::StarTrek),
            11 => Ok(RgbMode::Ripple),
            12 => Ok(RgbMode::Enraptured),
            13 => Ok(RgbMode::ButtonResponse),
            14 => Ok(RgbMode::Off),
            15 => Ok(RgbMode::SingleBreath),
            16 => Ok(RgbMode::CycleColor),
            _ => Err(ProtocolError::InvalidRgbMode(val)),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            RgbMode::DpiBreathing => "DPI Breathing",
            RgbMode::CycleBreathing => "Cycle Breathing",
            RgbMode::StaticLight => "Static Light",
            RgbMode::FlowingWater => "Flowing Water",
            RgbMode::MonoWater => "Mono Water",
            RgbMode::CometStreak => "Comet Streak",
            RgbMode::Neon => "Neon",
            RgbMode::Ambilight => "Ambilight",
            RgbMode::Flicker => "Flicker",
            RgbMode::StarTrek => "Star Trek",
            RgbMode::Ripple => "Ripple",
            RgbMode::Enraptured => "Enraptured",
            RgbMode::ButtonResponse => "Button Response",
            RgbMode::Off => "LED Off",
            RgbMode::SingleBreath => "Single Breath",
            RgbMode::CycleColor => "Cycle Color",
        }
    }
}

impl FromStr for RgbMode {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase().replace(['-', '_', ' '], "");
        match lower.as_str() {
            "dpibreathing" | "1" => Ok(RgbMode::DpiBreathing),
            "cyclebreathing" | "2" => Ok(RgbMode::CycleBreathing),
            "static" | "staticlight" | "lighton" | "3" => Ok(RgbMode::StaticLight),
            "flowingwater" | "flow" | "4" => Ok(RgbMode::FlowingWater),
            "monowater" | "5" => Ok(RgbMode::MonoWater),
            "comet" | "cometstreak" | "6" => Ok(RgbMode::CometStreak),
            "neon" | "7" => Ok(RgbMode::Neon),
            "ambilight" | "8" => Ok(RgbMode::Ambilight),
            "flicker" | "9" => Ok(RgbMode::Flicker),
            "startrek" | "10" => Ok(RgbMode::StarTrek),
            "ripple" | "11" => Ok(RgbMode::Ripple),
            "enraptured" | "12" => Ok(RgbMode::Enraptured),
            "buttonresponse" | "13" => Ok(RgbMode::ButtonResponse),
            "off" | "ledoff" | "14" => Ok(RgbMode::Off),
            "singlebreath" | "breath" | "15" => Ok(RgbMode::SingleBreath),
            "cyclecolor" | "cycle" | "16" => Ok(RgbMode::CycleColor),
            _ => Err(ProtocolError::InvalidRgbMode(0)),
        }
    }
}

impl fmt::Display for RgbMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Full RGB Configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbConfig {
    pub mode: RgbMode,
    pub brightness: u8, // 0..4 (level 1..5)
    pub speed: u8,      // 0..4 (level 1..5)
    pub color: RgbColor,
    pub direction: bool,
    pub symmetry: bool,
}

impl Default for RgbConfig {
    fn default() -> Self {
        Self {
            mode: RgbMode::Neon,
            brightness: 2,
            speed: 3,
            color: RgbColor::new(255, 0, 0),
            direction: false,
            symmetry: true,
        }
    }
}

/// Mouse programmable buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Button {
    Left = 1,
    Middle = 2,
    Right = 3,
    Back = 4,
    Forward = 5,
    DpiLoop = 6,
}

impl Button {
    pub fn hw_code(&self) -> u8 {
        match self {
            Button::Left => 1,
            Button::Middle => 2,
            Button::Right => 3,
            Button::Back => 4,
            Button::Forward => 5,
            Button::DpiLoop => 6,
        }
    }

    pub fn from_hw_code(code: u8) -> Result<Self, ProtocolError> {
        match code {
            1 => Ok(Button::Left),
            2 => Ok(Button::Middle),
            3 => Ok(Button::Right),
            4 => Ok(Button::Back),
            5 => Ok(Button::Forward),
            6 => Ok(Button::DpiLoop),
            _ => Err(ProtocolError::InvalidButton(code)),
        }
    }
}

impl FromStr for Button {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "1" | "left" | "l" => Ok(Button::Left),
            "2" | "middle" | "m" | "wheel" => Ok(Button::Middle),
            "3" | "right" | "r" => Ok(Button::Right),
            "4" | "back" | "backward" => Ok(Button::Back),
            "5" | "forward" | "fwd" => Ok(Button::Forward),
            "6" | "dpi" | "dpiloop" => Ok(Button::DpiLoop),
            _ => Err(ProtocolError::InvalidButton(0)),
        }
    }
}

/// Action assigned to a button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ButtonAction {
    Disabled = 0,
    LeftClick = 1,
    MiddleClick = 2,
    RightClick = 3,
    Back = 4,
    Forward = 5,
    DpiLoop = 6,
    RapidFire = 14,
    LedLoop = 15,
    DpiPlus = 16,
    DpiMinus = 17,
    ModeLoop = 41,
}

impl ButtonAction {
    pub fn action_code(&self) -> u8 {
        *self as u8
    }

    pub fn from_action_code(code: u8) -> Result<Self, ProtocolError> {
        match code {
            0 => Ok(ButtonAction::Disabled),
            1 => Ok(ButtonAction::LeftClick),
            2 => Ok(ButtonAction::MiddleClick),
            3 => Ok(ButtonAction::RightClick),
            4 => Ok(ButtonAction::Back),
            5 => Ok(ButtonAction::Forward),
            6 => Ok(ButtonAction::DpiLoop),
            14 => Ok(ButtonAction::RapidFire),
            15 => Ok(ButtonAction::LedLoop),
            16 => Ok(ButtonAction::DpiPlus),
            17 => Ok(ButtonAction::DpiMinus),
            41 => Ok(ButtonAction::ModeLoop),
            _ => Err(ProtocolError::InvalidAction(code)),
        }
    }
}

impl FromStr for ButtonAction {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.to_lowercase().replace(['-', '_', ' '], "");
        match clean.as_str() {
            "disabled" | "none" | "0" => Ok(ButtonAction::Disabled),
            "left" | "leftclick" | "1" => Ok(ButtonAction::LeftClick),
            "middle" | "middleclick" | "2" => Ok(ButtonAction::MiddleClick),
            "right" | "rightclick" | "3" => Ok(ButtonAction::RightClick),
            "back" | "backward" | "4" => Ok(ButtonAction::Back),
            "forward" | "fwd" | "5" => Ok(ButtonAction::Forward),
            "dpiloop" | "dpi" | "6" => Ok(ButtonAction::DpiLoop),
            "rapidfire" | "fire" | "14" => Ok(ButtonAction::RapidFire),
            "ledloop" | "led" | "15" => Ok(ButtonAction::LedLoop),
            "dpiplus" | "dpi+" | "16" => Ok(ButtonAction::DpiPlus),
            "dpiminus" | "dpi-" | "17" => Ok(ButtonAction::DpiMinus),
            "modeloop" | "mode" | "41" => Ok(ButtonAction::ModeLoop),
            _ => Err(ProtocolError::InvalidAction(0)),
        }
    }
}

/// A single DPI stage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpiStage {
    pub stage_index: u8, // 0..5
    pub dpi: u16,        // e.g. 800, 1600
    pub color: RgbColor,
    pub enabled: bool,
}

/// Raw 8-byte Feature Report packet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet(pub [u8; REPORT_LEN]);

impl Packet {
    pub fn new(bytes: [u8; REPORT_LEN]) -> Result<Self, ProtocolError> {
        if bytes[0] != REPORT_ID {
            return Err(ProtocolError::InvalidReportId(bytes[0]));
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; REPORT_LEN] {
        &self.0
    }

    pub fn command(&self) -> u8 {
        self.0[1]
    }
}

/// Protocol packet builder/encoder
pub struct PacketEncoder;

impl PacketEncoder {
    /// Builds Command 0x13: Set RGB lighting configuration
    pub fn encode_rgb(config: &RgbConfig) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::SET_LED;

        let user_color_en = 1u8;
        let mode = config.mode as u8;
        buf[2] = (user_color_en << 7) | (mode & 0x7f);

        let dir = if config.direction { 1u8 } else { 0u8 };
        let sym = if config.symmetry { 1u8 } else { 0u8 };
        let speed = (config.speed.min(4)) & 0x07;
        buf[3] = (dir << 7) | (sym << 4) | speed;

        buf[4] = 0x00;
        buf[5] = (config.brightness.min(4)) & 0x07;
        buf[6] = 0x00;
        buf[7] = 0x00;

        Packet(buf)
    }

    /// Builds Command 0x14: Set DPI stage scroll wheel LED indicator color
    ///
    /// Byte 2 = (stage_index << 5) | (green_nibble & 0x0F)
    /// Byte 3 = (red_nibble << 4) | (blue_nibble & 0x0F)
    pub fn encode_dpi_stage_led(stage_index: u8, color: &RgbColor) -> Result<Packet, ProtocolError> {
        if stage_index > 5 {
            return Err(ProtocolError::InvalidStageIndex(stage_index));
        }
        let (r_nib, g_nib, b_nib) = color.to_nibbles();

        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::SET_DPI_STAGE; // 0x14
        buf[2] = ((stage_index & 0x07) << 5) | (g_nib & 0x0f);
        buf[3] = ((r_nib & 0x0f) << 4) | (b_nib & 0x0f);
        buf[4] = 0;
        buf[5] = 0;
        buf[6] = 0;
        buf[7] = 0;

        Ok(Packet(buf))
    }

    /// Builds Command 0x14: Set DPI stage LED color from DpiStage struct
    pub fn encode_dpi_stage(stage: &DpiStage) -> Result<Packet, ProtocolError> {
        Self::encode_dpi_stage_led(stage.stage_index, &stage.color)
    }

    /// Packs 6 DPI stages into Chunk 0x10 (8 bytes) for sensor hardware configuration
    pub fn encode_dpi_resolution_chunk(stages: &[DpiStage]) -> Result<[u8; 8], ProtocolError> {
        let mut steps = [4u8; 6]; // default step 4 = 800 DPI
        for s in stages {
            if (s.stage_index as usize) < 6 {
                steps[s.stage_index as usize] = dpi_to_step_index(s.dpi)?;
            }
        }
        Self::encode_dpi_steps_to_chunk(&steps)
    }

    /// Packs step indices (1..24) for 6 stages into Chunk 0x10 format
    ///
    /// Layout:
    /// Byte 0..2: 0x00
    /// Byte 3: Bit 4 array of all 6 stages
    /// Byte 4: 0x08 constant
    /// Byte 5: ((s0 & 0x0F) << 4) | (s1 & 0x0F)
    /// Byte 6: ((s2 & 0x0F) << 4) | (s3 & 0x0F)
    /// Byte 7: ((s4 & 0x0F) << 4) | (s5 & 0x0F)
    pub fn encode_dpi_steps_to_chunk(steps: &[u8; 6]) -> Result<[u8; 8], ProtocolError> {
        for &s in steps {
            if !(1..=24).contains(&s) {
                return Err(ProtocolError::InvalidDpi(s as u16));
            }
        }
        let mut b3 = 0u8;
        for (i, &s) in steps.iter().enumerate() {
            if (s & 0x10) != 0 {
                b3 |= 1 << i;
            }
        }
        let b4 = 0x08;
        let b5 = ((steps[0] & 0x0f) << 4) | (steps[1] & 0x0f);
        let b6 = ((steps[2] & 0x0f) << 4) | (steps[3] & 0x0f);
        let b7 = ((steps[4] & 0x0f) << 4) | (steps[5] & 0x0f);

        Ok([0x00, 0x00, 0x00, b3, b4, b5, b6, b7])
    }

    /// Unpacks Chunk 0x10 into 6 step indices (1..24)
    pub fn decode_dpi_chunk_to_steps(chunk: &[u8; 8]) -> Result<[u8; 6], ProtocolError> {
        let b3 = chunk[3];
        let mut steps = [0u8; 6];

        steps[0] = ((chunk[5] >> 4) & 0x0f) | (((b3 >> 0) & 0x01) << 4);
        steps[1] = (chunk[5] & 0x0f) | (((b3 >> 1) & 0x01) << 4);
        steps[2] = ((chunk[6] >> 4) & 0x0f) | (((b3 >> 2) & 0x01) << 4);
        steps[3] = (chunk[6] & 0x0f) | (((b3 >> 3) & 0x01) << 4);
        steps[4] = ((chunk[7] >> 4) & 0x0f) | (((b3 >> 4) & 0x01) << 4);
        steps[5] = (chunk[7] & 0x0f) | (((b3 >> 5) & 0x01) << 4);

        for &s in &steps {
            if !(1..=24).contains(&s) {
                return Err(ProtocolError::InvalidDpi(s as u16));
            }
        }
        Ok(steps)
    }

    /// Unpacks Chunk 0x10 into human DPI values for the 6 stages
    pub fn decode_dpi_resolution_chunk(chunk: &[u8; 8]) -> Result<[u16; 6], ProtocolError> {
        let steps = Self::decode_dpi_chunk_to_steps(chunk)?;
        let mut dpis = [0u16; 6];
        for (i, &s) in steps.iter().enumerate() {
            dpis[i] = step_index_to_dpi(s)?;
        }
        Ok(dpis)
    }

    /// Builds Command 0x18: Write single byte to register/chunk
    pub fn encode_chunk_write_byte(
        offset: u16,
        byte_index: u8,
        data_byte: u8,
        chunk_len: u8,
    ) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::ACCESS_REGISTER; // 0x18
        buf[2] = 0x03; // write byte flag
        buf[3] = byte_index;
        buf[4] = (offset & 0xff) as u8;
        buf[5] = ((offset >> 8) & 0xff) as u8;
        buf[6] = data_byte;
        buf[7] = chunk_len.saturating_sub(1);
        Packet(buf)
    }

    /// Builds Command 0x18: Finalize write step 1
    pub fn encode_chunk_finalize_step1(offset: u16, chunk_len: u8) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::ACCESS_REGISTER;
        buf[2] = 0x09;
        buf[3] = 0x00;
        buf[4] = (offset & 0xff) as u8;
        buf[5] = ((offset >> 8) & 0xff) as u8;
        buf[6] = 0x00;
        buf[7] = chunk_len.saturating_sub(1);
        Packet(buf)
    }

    /// Builds Command 0x18: Finalize write step 2
    pub fn encode_chunk_finalize_step2(offset: u16) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::ACCESS_REGISTER;
        buf[2] = 0x00;
        buf[3] = 0x00;
        buf[4] = (offset & 0xff) as u8;
        buf[5] = ((offset >> 8) & 0xff) as u8;
        buf[6] = 0x00;
        buf[7] = 0x00;
        Packet(buf)
    }

    /// Builds Command 0x18: Read prep request (VA 0x00413ad0)
    pub fn encode_chunk_read_prep(offset: u16, byte_index: u8, chunk_len: u8) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::ACCESS_REGISTER;
        buf[2] = 0x03;
        buf[3] = byte_index;
        buf[4] = (offset & 0xff) as u8;
        buf[5] = ((offset >> 8) & 0xff) as u8;
        buf[6] = 0x00;
        buf[7] = chunk_len.saturating_sub(1);
        Packet(buf)
    }

    /// Builds Command 0x18: Read byte request (VA 0x00413a00)
    pub fn encode_chunk_read_byte(offset: u16, byte_index: u8, chunk_len: u8) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::ACCESS_REGISTER;
        buf[2] = 0x05; // read byte flag
        buf[3] = byte_index;
        buf[4] = (offset & 0xff) as u8;
        buf[5] = ((offset >> 8) & 0xff) as u8;
        buf[6] = 0x00;
        buf[7] = chunk_len.saturating_sub(1);
        Packet(buf)
    }

    /// Builds Command 0x16: Feature flags (0x07 enables all feature channels)
    pub fn encode_flags(flags: u8) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::SET_FLAGS;
        buf[2] = 0x00;
        buf[3] = flags;
        buf[4] = 0;
        buf[5] = 0;
        buf[6] = 0;
        buf[7] = 0;
        Packet(buf)
    }

    /// Builds Command 0x15: Set active DPI stages list
    pub fn encode_active_stages(stages: &[DpiStage]) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::SET_ACTIVE_STAGES;

        let mut out_idx = 2;
        for s in stages {
            if s.enabled && out_idx < REPORT_LEN {
                buf[out_idx] = s.stage_index + 1;
                out_idx += 1;
            }
        }
        Packet(buf)
    }

    /// Builds Command 0x10: Button mapping
    pub fn encode_button(button: Button, action: ButtonAction) -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::SET_BUTTON;
        buf[2] = button.hw_code();
        buf[3] = action.action_code();
        buf[4] = 0;
        buf[5] = 0;
        buf[6] = 0;
        buf[7] = 0;
        Packet(buf)
    }

    /// Builds Command 0x20: Save all settings to mouse onboard EEPROM/flash
    pub fn encode_save() -> Packet {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = cmd::SAVE_TO_FLASH;
        Packet(buf)
    }
}

/// Detailed hardware device status decoded from Feature Report 0x07
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub active_dpi: u16,
    pub polling_rate: PollingRate,
    pub profile_index: u8,
    pub eeprom_size_bytes: usize,
    pub signature: u8,
}

/// Protocol status decoder
pub struct PacketDecoder;

impl PacketDecoder {
    /// Parses the 8-byte status response returned by HIDIOCGFEATURE(8)
    pub fn decode_status(buf: &[u8]) -> Result<(u16, PollingRate), ProtocolError> {
        let status = Self::decode_device_status(buf)?;
        Ok((status.active_dpi, status.polling_rate))
    }

    /// Parses the complete 8-byte device status returned by HIDIOCGFEATURE(8)
    pub fn decode_device_status(buf: &[u8]) -> Result<DeviceStatus, ProtocolError> {
        if buf.len() < REPORT_LEN {
            return Err(ProtocolError::InvalidLength {
                expected: REPORT_LEN,
                actual: buf.len(),
            });
        }
        if buf[0] != REPORT_ID {
            return Err(ProtocolError::InvalidReportId(buf[0]));
        }

        // Bytes 2 and 3 contain DPI measurement
        let raw_dpi = u16::from_le_bytes([buf[2], buf[3]]);
        let active_dpi = if raw_dpi > 0 { raw_dpi } else { 800 };

        // In Feature Report 0x07, Byte 6 contains Profile Index [bits 3:2] and EEPROM Size [bit 1].
        // True polling rate is stored in EEPROM Chunk 0x18 Byte 6, so fallback to Hz1000 if not a direct polling code.
        let polling_rate = PollingRate::from_hw_code(buf[6]).unwrap_or(PollingRate::Hz1000);
        let profile_index = (buf[6] >> 2) & 0x03;
        let eeprom_size_bytes = if (buf[6] & 0x02) != 0 { 4088 } else { 2040 };
        let signature = buf[7];

        Ok(DeviceStatus {
            active_dpi,
            polling_rate,
            profile_index,
            eeprom_size_bytes,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpi_step_conversion() {
        assert_eq!(dpi_to_step_index(200).unwrap(), 1);
        assert_eq!(dpi_to_step_index(800).unwrap(), 4);
        assert_eq!(dpi_to_step_index(1600).unwrap(), 8);
        assert_eq!(dpi_to_step_index(2400).unwrap(), 12);
        assert_eq!(dpi_to_step_index(12800).unwrap(), 24);

        assert_eq!(step_index_to_dpi(1).unwrap(), 200);
        assert_eq!(step_index_to_dpi(4).unwrap(), 800);
        assert_eq!(step_index_to_dpi(8).unwrap(), 1600);
        assert_eq!(step_index_to_dpi(24).unwrap(), 12800);
    }

    #[test]
    fn test_dpi_resolution_chunk_packing_roundtrip() {
        // Steps: 8 (1600), 12 (2400), 4 (800), 17 (6400), 22 (10400), 24 (12800)
        let steps = [8u8, 12, 4, 17, 22, 24];
        let chunk = PacketEncoder::encode_dpi_steps_to_chunk(&steps).unwrap();
        assert_eq!(chunk, [0x00, 0x00, 0x00, 0x38, 0x08, 0x8c, 0x41, 0x68]);

        let decoded = PacketEncoder::decode_dpi_chunk_to_steps(&chunk).unwrap();
        assert_eq!(decoded, steps);

        let dpis = PacketEncoder::decode_dpi_resolution_chunk(&chunk).unwrap();
        assert_eq!(dpis, [1600, 2400, 800, 6400, 10400, 12800]);
    }

    #[test]
    fn test_dpi_stage_led_nibble_encoding() {
        let color = RgbColor::new(255, 0, 0); // Pure Red
        let pkt = PacketEncoder::encode_dpi_stage_led(0, &color).unwrap();
        // Byte 2: (0 << 5) | (g_nib=15 & 0x0F) = 0x0F
        // Byte 3: (r_nib=0 << 4) | (b_nib=15 & 0x0F) = 0x0F
        assert_eq!(pkt.as_bytes(), &[0x07, 0x14, 0x0f, 0x0f, 0, 0, 0, 0]);

        // Stage 1 (green):
        let color_green = RgbColor::new(0, 255, 0);
        let pkt_g = PacketEncoder::encode_dpi_stage_led(1, &color_green).unwrap();
        // Byte 2: (1 << 5) | (g_nib=0) = 0x20
        // Byte 3: (r_nib=15 << 4) | (b_nib=15) = 0xFF
        assert_eq!(pkt_g.as_bytes(), &[0x07, 0x14, 0x20, 0xff, 0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_chunk_write_packets() {
        let p_byte = PacketEncoder::encode_chunk_write_byte(0x10, 5, 0x8c, 8);
        assert_eq!(p_byte.as_bytes(), &[0x07, 0x18, 0x03, 5, 0x10, 0x00, 0x8c, 0x07]);

        let p_fin1 = PacketEncoder::encode_chunk_finalize_step1(0x10, 8);
        assert_eq!(p_fin1.as_bytes(), &[0x07, 0x18, 0x09, 0x00, 0x10, 0x00, 0x00, 0x07]);

        let p_fin2 = PacketEncoder::encode_chunk_finalize_step2(0x10);
        assert_eq!(p_fin2.as_bytes(), &[0x07, 0x18, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_encode_flags() {
        let pkt = PacketEncoder::encode_flags(0x07);
        assert_eq!(pkt.as_bytes(), &[0x07, 0x16, 0x00, 0x07, 0, 0, 0, 0]);
    }

    #[test]
    fn test_polling_rate_conversion() {
        assert_eq!(PollingRate::Hz1000.hw_code(), 0x00);
        assert_eq!(PollingRate::Hz500.hw_code(), 0x01);
        assert_eq!(PollingRate::Hz250.hw_code(), 0x03);
        assert_eq!(PollingRate::Hz125.hw_code(), 0x07);

        assert_eq!(PollingRate::from_hz(1000).unwrap(), PollingRate::Hz1000);
        assert_eq!(PollingRate::from_hz(500).unwrap(), PollingRate::Hz500);
    }

    #[test]
    fn test_rgb_nibble_conversion() {
        let color = RgbColor::new(255, 0, 128);
        let (r, g, b) = color.to_nibbles();
        assert_eq!(r, 0); // 255 inverted -> 0
        assert_eq!(g, 15); // 0 inverted -> 15
        assert!(b > 0 && b < 15);
    }

    #[test]
    fn test_encode_save() {
        let pkt = PacketEncoder::encode_save();
        assert_eq!(pkt.as_bytes(), &[0x07, 0x20, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_button() {
        let pkt = PacketEncoder::encode_button(Button::Middle, ButtonAction::DpiLoop);
        assert_eq!(pkt.as_bytes(), &[0x07, 0x10, 0x02, 0x06, 0, 0, 0, 0]);
    }

    #[test]
    fn test_decode_status() {
        let raw = [0x07, 0x00, 0x68, 0x09, 0x00, 0x00, 0x03, 0x3b];
        let (dpi, rate) = PacketDecoder::decode_status(&raw).unwrap();
        assert_eq!(dpi, 2408);
        assert_eq!(rate, PollingRate::Hz250);

        let status = PacketDecoder::decode_device_status(&raw).unwrap();
        assert_eq!(status.active_dpi, 2408);
        assert_eq!(status.polling_rate, PollingRate::Hz250);
        assert_eq!(status.profile_index, 0);
        assert_eq!(status.eeprom_size_bytes, 4088);
        assert_eq!(status.signature, 0x3b);
    }
}
