//! File references to verified source images.

use crate::encoding::Optional;
use crate::SaveStateError;
use machine_loop::config::FLOPPY_DRIVE_COUNT;
use machine_loop::floppy::{FloppyDisk, Track, FORMATTED_DISK_BYTES, TRACKS_PER_DISK};
use machine_loop::physical_memory::{validate_kickstart_size, KickstartBytes, KICKSTART_ROM_SIZE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceImage {
    /// Display label from the supplying frontend.
    pub name: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaSources {
    pub rom: Option<SourceImage>,
    pub disks: [Option<SourceImage>; FLOPPY_DRIVE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
struct Sha256Hex(String);

impl Sha256Hex {
    fn of(bytes: &[u8]) -> Self {
        let mut hex = String::with_capacity(64);
        const DIGITS: &[u8] = b"0123456789abcdef";
        for byte in Sha256::digest(bytes) {
            hex.push(char::from(DIGITS[usize::from(byte >> 4)]));
            hex.push(char::from(DIGITS[usize::from(byte & 15)]));
        }
        Self(hex)
    }
}

impl TryFrom<String> for Sha256Hex {
    type Error = &'static str;

    fn try_from(hex: String) -> Result<Self, Self::Error> {
        if hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            Ok(Self(hex))
        } else {
            Err("Expected a 64-digit lowercase SHA-256 digest")
        }
    }
}

impl From<Sha256Hex> for String {
    fn from(digest: Sha256Hex) -> Self {
        digest.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RomReference {
    sha256: Sha256Hex,
    name: Optional<String>,
    synthetic: bool,
}

impl RomReference {
    pub(super) fn capture(
        rom: &[u8],
        source: Option<&SourceImage>,
    ) -> Result<Self, SaveStateError> {
        if let Some(source) = source.filter(|source| source.bytes == rom) {
            return Ok(Self {
                sha256: Sha256Hex::of(rom),
                name: Optional(source.name.clone()),
                synthetic: false,
            });
        }
        if rom == KickstartBytes::synthetic().as_slice() {
            return Ok(Self {
                sha256: Sha256Hex::of(rom),
                name: Optional(None),
                synthetic: true,
            });
        }
        Err(SaveStateError::MediaUnavailable(
            "Kickstart ROM source is unavailable for the current ROM bytes".into(),
        ))
    }

    pub(super) fn is_synthetic(&self) -> bool {
        self.synthetic
    }

    pub(super) fn matches(&self, bytes: &[u8]) -> bool {
        bytes.len() == KICKSTART_ROM_SIZE && Sha256Hex::of(bytes) == self.sha256
    }

    pub(super) fn restore(
        &self,
        source: Option<&SourceImage>,
    ) -> Result<KickstartBytes, SaveStateError> {
        if self.synthetic {
            let rom = KickstartBytes::synthetic();
            if !self.matches(rom.as_slice()) {
                return Err(SaveStateError::CorruptedData(
                    "Incorrect synthetic Kickstart ROM identity".into(),
                ));
            }
            return Ok(rom);
        }
        let asset = describe("Kickstart ROM", self.name.0.as_deref());
        let source = source.ok_or_else(|| unavailable(&asset))?;
        validate_kickstart_size(source.bytes.len())
            .map_err(|error| SaveStateError::MediaUnavailable(error.to_string()))?;
        if !self.matches(&source.bytes) {
            return Err(mismatch(&asset));
        }
        KickstartBytes::from_image(source.bytes.clone())
            .map_err(|error| SaveStateError::MediaUnavailable(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiskReference {
    sha256: Sha256Hex,
    name: Optional<String>,
}

impl DiskReference {
    pub(super) fn name(&self) -> Option<&str> {
        self.name.0.as_deref()
    }

    pub(super) fn capture(
        drive: usize,
        track_count: usize,
        source: Option<&SourceImage>,
    ) -> Result<Self, SaveStateError> {
        let asset = format!("ADF for DF{drive}");
        if track_count != TRACKS_PER_DISK {
            return Err(SaveStateError::CorruptedData(format!(
                "Incorrect track count for DF{drive}"
            )));
        }
        let source = source.ok_or_else(|| unavailable(&asset))?;
        if source.bytes.len() != FORMATTED_DISK_BYTES {
            return Err(invalid_adf_size(&asset, source.bytes.len()));
        }
        Ok(Self {
            sha256: Sha256Hex::of(&source.bytes),
            name: Optional(source.name.clone()),
        })
    }

    pub(super) fn matches(&self, bytes: &[u8]) -> bool {
        bytes.len() == FORMATTED_DISK_BYTES && Sha256Hex::of(bytes) == self.sha256
    }

    pub(super) fn restore(
        &self,
        drive: usize,
        source: Option<&SourceImage>,
    ) -> Result<Vec<Track>, SaveStateError> {
        let asset = describe(&format!("ADF for DF{drive}"), self.name());
        let source = source.ok_or_else(|| unavailable(&asset))?;
        if source.bytes.len() == FORMATTED_DISK_BYTES && !self.matches(&source.bytes) {
            return Err(mismatch(&asset));
        }
        FloppyDisk::from_adf(&source.bytes)
            .map(FloppyDisk::into_tracks)
            .ok_or_else(|| invalid_adf_size(&asset, source.bytes.len()))
    }
}

fn invalid_adf_size(asset: &str, actual: usize) -> SaveStateError {
    SaveStateError::MediaUnavailable(format!(
        "{asset} must contain {FORMATTED_DISK_BYTES} bytes; received {actual} bytes"
    ))
}

fn describe(kind: &str, name: Option<&str>) -> String {
    name.map_or_else(|| kind.to_string(), |name| format!("{kind} ({name})"))
}

fn unavailable(asset: &str) -> SaveStateError {
    SaveStateError::MediaUnavailable(format!("{asset} source is unavailable"))
}

fn mismatch(asset: &str) -> SaveStateError {
    SaveStateError::MediaUnavailable(format!("{asset} content does not match the save"))
}
