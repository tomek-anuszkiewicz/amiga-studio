//! Save file format, JSON persistence and PNG preview encoding.

use machine_loop::{A500Machine, A500State};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// Active schema format version for A500 save states
pub const SAVE_STATE_VERSION: u32 = 5;

/// Metadata header identifying state compatibility, creation time, and display preview
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveStateHeader {
    /// Schema format version (e.g. 1)
    pub version: u32,
    /// Unix timestamp when the save state was created (seconds since epoch)
    pub timestamp: u64,
    /// PNG-encoded screenshot bytes captured from Denise frame builder
    pub screenshot_png: Vec<u8>,
}

/// File metadata and pure machine state are separate values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveState {
    pub header: SaveStateHeader,
    pub state: Box<A500State>,
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

impl SaveState {
    /// Captures the file envelope and preview outside the machine stepping core.
    pub fn capture(machine: &A500Machine) -> Self {
        let frame = &machine.denise.frame_builder;
        Self {
            header: SaveStateHeader {
                version: SAVE_STATE_VERSION,
                timestamp: 0,
                screenshot_png: encode_screenshot_png(
                    frame.frame_buffer(),
                    frame.width,
                    frame.height,
                    frame.row_stride(),
                ),
            },
            state: Box::new(machine.save_state()),
        }
    }

    fn validate_version(&self) -> Result<(), SaveStateError> {
        if self.header.version != SAVE_STATE_VERSION {
            return Err(SaveStateError::IncompatibleVersion {
                found: self.header.version,
                supported: SAVE_STATE_VERSION,
            });
        }
        Ok(())
    }

    /// Serializes the complete save file to a JSON string
    pub fn to_json(&self) -> Result<String, SaveStateError> {
        serde_json::to_string(self).map_err(|e| SaveStateError::SerializationFailed(e.to_string()))
    }

    /// Serializes the complete save file to a pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, SaveStateError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SaveStateError::SerializationFailed(e.to_string()))
    }

    /// Deserializes a save file from a JSON string
    pub fn from_json(json_str: &str) -> Result<Self, SaveStateError> {
        let saved: Self = serde_json::from_str(json_str)
            .map_err(|e| SaveStateError::DeserializationFailed(e.to_string()))?;
        saved.validate_version()?;
        Ok(saved)
    }

    /// Deserializes a save file from raw JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SaveStateError> {
        let saved: Self = serde_json::from_slice(bytes)
            .map_err(|e| SaveStateError::DeserializationFailed(e.to_string()))?;
        saved.validate_version()?;
        Ok(saved)
    }

    /// Saves the snapshot to a JSON file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), SaveStateError> {
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
fn encode_screenshot_png(frame_buffer: &[u32], width: u32, height: u32, stride: usize) -> Vec<u8> {
    use image::ImageEncoder;
    let total_pixels = (width as usize).saturating_mul(height as usize);
    if total_pixels == 0 || frame_buffer.len() < stride.saturating_mul(height as usize) {
        return Vec::new();
    }
    let mut rgba = Vec::with_capacity(total_pixels.saturating_mul(4));
    for &pixel in frame_buffer
        .chunks_exact(stride)
        .take(height as usize)
        .flat_map(|row| &row[..width as usize])
    {
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
