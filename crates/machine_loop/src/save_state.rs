//! Amiga 500 Save State Architecture & Serialization Specification
//!
//! Provides decoupled, serde-compatible state snapshots (`A500State`),
//! metadata headers (`SaveStateHeader`), compatibility verification, and
//! round-trip serialization (formatted JSON).

use std::fmt;
use std::path::Path;

use config::A500Config;
use cpu::CpuState;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};

/// Active schema format version for A500 save states
pub const SAVE_STATE_VERSION: u32 = 1;

/// Metadata header identifying state compatibility, creation time, and display preview
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveStateHeader {
    /// Schema format version (e.g. 1)
    pub version: u32,
    /// Unix timestamp when the save state was created (seconds since epoch)
    pub timestamp: u64,
    /// PNG-encoded screenshot bytes captured from Denise frame builder
    #[serde(default)]
    pub screenshot_png: Vec<u8>,
}

/// Master state container representing a full Amiga 500 machine snapshot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A500State {
    /// Compatibility header and metadata
    pub header: SaveStateHeader,
    /// Master monotonic Color Clock counter
    pub cck: u64,
    /// Active hardware configuration model
    pub config: A500Config,
    /// Motorola 68000 CPU register and micro-execution state
    pub cpu: CpuState,
    /// 24-bit physical memory buffers, overlay flag, and open bus settings
    pub physical_memory: PhysicalMemory,
    /// OKI MSM6242B Real-Time Clock
    pub rtc: rtc::RtcMsm6242b,
    /// Agnus (beam counters, copper, blitter, dma, chip RAM bus lock)
    pub agnus: agnus::Agnus,
    /// Denise (video control, sprites, frame builder, color palette, collisions)
    pub denise: denise::Denise,
    /// Paula (audio channels, serial UART, interrupt multiplexer)
    pub paula: paula::Paula,
    /// MOS 8520 CIA-A
    pub cia_a: cia::Cia,
    /// MOS 8520 CIA-B
    pub cia_b: cia::Cia,
    /// 3.5" DD Floppy controller and drive units
    pub floppy: floppy::FloppyController,
    /// MOS 6500/1 keyboard microcontroller
    pub keyboard: keyboard::Keyboard,
    /// Dual Atari 9-pin controller game ports
    pub game_ports: game_ports::GamePorts,
}

/// Errors that can occur during save state serialization, deserialization, or verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveStateError {
    /// Save state schema version is unsupported
    IncompatibleVersion { found: u32, supported: u32 },
    /// Serialization error
    SerializationFailed(String),
    /// Deserialization error
    DeserializationFailed(String),
    /// File I/O error
    IoError(String),
    /// Corrupted or truncated payload
    CorruptedData(String),
}

impl fmt::Display for SaveStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleVersion { found, supported } => {
                write!(
                    f,
                    "Incompatible save state version: found {found}, supported {supported}"
                )
            }
            Self::SerializationFailed(msg) => write!(f, "Save state serialization failed: {msg}"),
            Self::DeserializationFailed(msg) => {
                write!(f, "Save state deserialization failed: {msg}")
            }
            Self::IoError(msg) => write!(f, "Save state I/O error: {msg}"),
            Self::CorruptedData(msg) => write!(f, "Save state corrupted: {msg}"),
        }
    }
}

impl std::error::Error for SaveStateError {}

impl A500State {
    /// Serializes the complete machine state to a JSON string
    pub fn to_json(&self) -> Result<String, SaveStateError> {
        serde_json::to_string(self).map_err(|e| SaveStateError::SerializationFailed(e.to_string()))
    }

    /// Serializes the complete machine state to a pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, SaveStateError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SaveStateError::SerializationFailed(e.to_string()))
    }

    /// Deserializes an A500State snapshot from a JSON string
    pub fn from_json(json_str: &str) -> Result<Self, SaveStateError> {
        serde_json::from_str(json_str)
            .map_err(|e| SaveStateError::DeserializationFailed(e.to_string()))
    }

    /// Deserializes an A500State snapshot from raw JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SaveStateError> {
        serde_json::from_slice(bytes)
            .map_err(|e| SaveStateError::DeserializationFailed(e.to_string()))
    }

    /// Saves the snapshot to a JSON file
    pub(crate) fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), SaveStateError> {
        let path = path.as_ref();
        let json_pretty = self.to_json_pretty()?;
        std::fs::write(path, json_pretty).map_err(|e| SaveStateError::IoError(e.to_string()))
    }

    /// Loads a snapshot from a JSON file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, SaveStateError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| SaveStateError::IoError(e.to_string()))?;
        Self::from_bytes(&bytes)
    }
}

/// Encodes an ARGB (0xAARRGGBB) pixel buffer into PNG bytes.
pub(crate) fn encode_screenshot_png(frame_buffer: &[u32], width: u32, height: u32) -> Vec<u8> {
    use image::ImageEncoder;
    let total_pixels = (width as usize).saturating_mul(height as usize);
    if total_pixels == 0 || frame_buffer.len() < total_pixels {
        return Vec::new();
    }
    let mut rgba = Vec::with_capacity(total_pixels.saturating_mul(4));
    for &pixel in &frame_buffer[..total_pixels] {
        rgba.push(((pixel >> 16) & 0xFF) as u8); // R
        rgba.push(((pixel >> 8) & 0xFF) as u8); // G
        rgba.push((pixel & 0xFF) as u8); // B
        rgba.push(((pixel >> 24) & 0xFF) as u8); // A
    }
    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    if encoder
        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
        .is_err()
    {
        return Vec::new();
    }
    png_bytes
}
