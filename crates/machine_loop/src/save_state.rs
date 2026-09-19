//! Amiga 500 Save State Architecture & Serialization Specification
//!
//! Provides decoupled, serde-compatible state snapshots (`A500State`),
//! metadata headers (`SaveStateHeader`), compatibility verification, and
//! round-trip serialization (JSON & gzip-compressed binary).

use std::fmt;
use std::io::{Read, Write};
use std::path::Path;

use config::{A500Config, VideoStandard};
use cpu::CpuState;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};

/// Magic identifier for Amiga 500 Save State files ("A500")
pub const SAVE_STATE_MAGIC: [u8; 4] = *b"A500";

/// Active schema format version for A500 save states
pub const SAVE_STATE_VERSION: u32 = 1;

/// Metadata header identifying state compatibility, checksums, and machine configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveStateHeader {
    /// Format identifier: b"A500"
    pub magic: [u8; 4],
    /// Schema format version (e.g. 1)
    pub version: u32,
    /// Unix timestamp when the save state was created (seconds since epoch)
    pub timestamp: u64,
    /// Video standard (PAL or NTSC)
    pub video_standard: VideoStandard,
    /// Configured Chip RAM size in bytes (e.g. 524,288 or 1,048,576)
    pub chip_ram_size: usize,
    /// Configured Slow RAM size in bytes (0 or 524,288)
    pub slow_ram_size: usize,
    /// Configured Fast RAM size in bytes (0 to 8,388,608)
    pub fast_ram_size: usize,
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
    /// File header magic bytes do not match "A500"
    InvalidMagic,
    /// Save state schema version is unsupported
    IncompatibleVersion { found: u32, supported: u32 },
    /// Configured Chip RAM size does not match save state Chip RAM size
    MemorySizeMismatch {
        expected_chip: usize,
        actual_chip: usize,
    },
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
            Self::InvalidMagic => write!(f, "Invalid save state magic header (expected 'A500')"),
            Self::IncompatibleVersion { found, supported } => {
                write!(
                    f,
                    "Incompatible save state version: found {found}, supported {supported}"
                )
            }
            Self::MemorySizeMismatch {
                expected_chip,
                actual_chip,
            } => {
                write!(
                    f,
                    "Memory size mismatch: state has {expected_chip} bytes Chip RAM, machine has {actual_chip} bytes"
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

    /// Serializes and compresses the state snapshot to gzip-compressed bytes
    pub fn to_compressed_bytes(&self) -> Result<Vec<u8>, SaveStateError> {
        let json_bytes = serde_json::to_vec(self)
            .map_err(|e| SaveStateError::SerializationFailed(e.to_string()))?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&json_bytes)
            .map_err(|e| SaveStateError::SerializationFailed(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| SaveStateError::SerializationFailed(e.to_string()))
    }

    /// Deserializes an A500State snapshot from bytes, automatically detecting gzip vs raw JSON
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SaveStateError> {
        if bytes.len() >= 2 && bytes[0] == 0x1F && bytes[1] == 0x8B {
            // Gzip compressed payload
            let mut decoder = GzDecoder::new(bytes);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| SaveStateError::CorruptedData(e.to_string()))?;
            serde_json::from_slice(&decompressed)
                .map_err(|e| SaveStateError::DeserializationFailed(e.to_string()))
        } else {
            // Raw JSON bytes
            serde_json::from_slice(bytes)
                .map_err(|e| SaveStateError::DeserializationFailed(e.to_string()))
        }
    }

    /// Saves the snapshot to a file (compressed if path ends in .gz / .a500z or requested)
    pub(crate) fn save_to_file(
        &self,
        path: impl AsRef<Path>,
        compressed: bool,
    ) -> Result<(), SaveStateError> {
        let path = path.as_ref();
        let bytes = if compressed {
            self.to_compressed_bytes()?
        } else {
            self.to_json_pretty()?.into_bytes()
        };
        std::fs::write(path, bytes).map_err(|e| SaveStateError::IoError(e.to_string()))
    }

    /// Loads a snapshot from a file, automatically detecting compression
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, SaveStateError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| SaveStateError::IoError(e.to_string()))?;
        Self::from_bytes(&bytes)
    }
}
