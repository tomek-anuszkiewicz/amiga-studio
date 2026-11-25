//! Save file format, JSON persistence and PNG preview encoding.

mod encoding;
mod media;

use encoding::{Bytes, Optional, Pixels, Required};
use machine_loop::config::{A500Config, FastRamSize, SlowRamSize, FLOPPY_DRIVE_COUNT};
use machine_loop::denise::{
    frame_builder::{FrameBuilder, FRAME_BUFFER_CAPACITY},
    Denise,
};
use machine_loop::floppy::{FloppyController, FloppyDrive};
use machine_loop::physical_memory::{
    PhysicalMemory, CHIP_RAM_SIZE_512K, MAX_FAST_RAM_SIZE, SLOW_RAM_SIZE,
};
use machine_loop::{A500Machine, A500State};
use media::{DiskReference, RomReference};
pub use media::{MediaSources, SourceImage};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::fmt;

/// Active schema format version for A500 save states
pub const SAVE_STATE_VERSION: u32 = 17;

/// Metadata header identifying state compatibility, creation time, and display preview
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveStateHeader {
    /// Schema format version (e.g. 1)
    pub version: u32,
    /// Unix timestamp when the save state was created (seconds since epoch)
    pub timestamp: u64,
    /// PNG-encoded screenshot bytes captured from Denise frame builder
    #[serde(with = "encoding::preview")]
    pub screenshot_png: Vec<u8>,
    pub association: SaveAssociation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Portable player association with the game and mounted library entries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveAssociation {
    pub game_id: Option<String>,
    pub mounted: [Option<String>; FLOPPY_DRIVE_COUNT],
}

type FileMemory = PhysicalMemory<
    Bytes<CHIP_RAM_SIZE_512K>,
    Optional<Bytes<SLOW_RAM_SIZE>>,
    Optional<Bytes<MAX_FAST_RAM_SIZE>>,
    Required<RomReference>,
>;
type FileDisplay = Denise<FrameBuilder<Pixels>>;
type FileDisk = FloppyController<FloppyDrive<Optional<DiskReference>>>;
type FileMachine = A500State<FileMemory, FileDisplay, FileDisk>;

/// File metadata and guest state. External images are verified before the
/// stored data can become a full machine snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveState {
    pub header: SaveStateHeader,
    state: Box<FileMachine>,
}

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

#[derive(Deserialize)]
struct Envelope<T> {
    header: T,
    state: Option<T>,
}

impl Serialize for SaveState {
    fn serialize<S: serde::Serializer>(&self, encoder: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut fields = encoder.serialize_struct("SaveState", 2)?;
        fields.serialize_field("header", &self.header)?;
        fields.serialize_field("state", &self.state)?;
        fields.end()
    }
}

impl<'de> Deserialize<'de> for SaveState {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let Envelope { header, state } = Envelope::<Box<RawValue>>::deserialize(decoder)?;
        Self::decode(&header, state.as_deref()).map_err(serde::de::Error::custom)
    }
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
    /// Corrupted or truncated payload
    CorruptedData(String),
    /// A required source image was missing or could not be verified.
    MediaUnavailable(String),
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
            Self::CorruptedData(msg) => write!(f, "Save state corrupted: {msg}"),
            Self::MediaUnavailable(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for SaveStateError {}

impl SaveState {
    /// Captures a preview and the current guest state against known source images.
    pub fn capture(machine: &A500Machine, sources: &MediaSources) -> Result<Self, SaveStateError> {
        let frame = &machine.denise.frame_builder;
        let screenshot_png = encode_screenshot_png(
            frame.frame_buffer(),
            frame.width,
            frame.height,
            frame.row_stride(),
        );
        let rom = RomReference::capture(
            machine.physical_memory.kickstart_rom(),
            sources.rom.as_ref(),
        )?;
        let mut disks: [_; FLOPPY_DRIVE_COUNT] = std::array::from_fn(|_| Optional(None));
        for (index, (drive, source)) in machine.floppy.drives.iter().zip(&sources.disks).enumerate()
        {
            if let Some(tracks) = drive.tracks() {
                disks[index] = Optional(Some(DiskReference::capture(
                    index,
                    tracks.len(),
                    source.as_ref(),
                )?));
            }
        }
        let memory = machine.physical_memory.clone().map_storage(
            Bytes,
            |bank| Optional(bank.map(Bytes)),
            |bank| Optional(bank.map(Bytes)),
            |_| Required(rom),
        );
        let display = machine
            .denise
            .clone()
            .map_frame(|frame| frame.map_storage(Pixels));
        let floppy = machine.floppy.with_drives(std::array::from_fn(|index| {
            machine.floppy.drives[index].with_tracks(Optional(disks[index].0.take()))
        }));
        let state = machine.capture_with_storage(memory, display, floppy);
        let saved = Self {
            header: SaveStateHeader {
                version: SAVE_STATE_VERSION,
                timestamp: 0,
                association: SaveAssociation::default(),
                description: None,
                screenshot_png,
            },
            state: Box::new(state),
        };
        saved.validate()?;
        Ok(saved)
    }

    pub fn cck(&self) -> u64 {
        self.state.cck
    }

    pub fn cpu(&self) -> &machine_loop::cpu::CpuState {
        &self.state.cpu
    }

    pub fn game_ports(&self) -> &machine_loop::game_ports::GamePorts {
        &self.state.game_ports
    }

    /// Rebuilds media from verified sources without generating insertion edges.
    pub fn restore(self, sources: &MediaSources) -> Result<Box<A500State>, SaveStateError> {
        self.validate()?;
        let (_, _, _, Required(rom)) = self.state.physical_memory.storage();
        let rom = rom.restore(sources.rom.as_ref())?;
        let mut disks: [_; FLOPPY_DRIVE_COUNT] = std::array::from_fn(|_| None);
        for (index, (drive, source)) in self
            .state
            .floppy
            .drives
            .iter()
            .zip(&sources.disks)
            .enumerate()
        {
            if let Some(reference) = &drive.tracks().0 {
                disks[index] = Some(reference.restore(index, source.as_ref())?);
            }
        }
        Ok(Box::new(self.state.map_storage(
            |memory| {
                memory.map_storage(
                    |chip| chip.0,
                    |bank| bank.0.map(|bytes| bytes.0),
                    |bank| bank.0.map(|bytes| bytes.0),
                    |_| rom,
                )
            },
            |denise| denise.map_frame(|frame| frame.map_storage(|pixels| pixels.0)),
            |floppy| {
                floppy.with_drives(std::array::from_fn(|index| {
                    floppy.drives[index].with_tracks(disks[index].take())
                }))
            },
        )))
    }

    pub fn synthetic_rom(&self) -> bool {
        let (_, _, _, Required(rom)) = self.state.physical_memory.storage();
        rom.is_synthetic()
    }

    pub fn rom_matches(&self, bytes: &[u8]) -> bool {
        let (_, _, _, Required(rom)) = self.state.physical_memory.storage();
        rom.matches(bytes)
    }

    pub fn disk_matches(&self, drive: usize, bytes: &[u8]) -> bool {
        self.state
            .floppy
            .drives
            .get(drive)
            .and_then(|drive| drive.tracks().0.as_ref())
            .is_some_and(|reference| reference.matches(bytes))
    }

    pub fn disk_name(&self, drive: usize) -> Option<&str> {
        self.state
            .floppy
            .drives
            .get(drive)?
            .tracks()
            .0
            .as_ref()?
            .name()
    }

    pub fn disk_inserted(&self, drive: usize) -> bool {
        self.state
            .floppy
            .drives
            .get(drive)
            .is_some_and(|drive| drive.disk_inserted)
    }

    fn validate(&self) -> Result<(), SaveStateError> {
        if self.header.version != SAVE_STATE_VERSION {
            return Err(SaveStateError::IncompatibleVersion {
                found: self.header.version,
                supported: SAVE_STATE_VERSION,
            });
        }
        let state = &self.state;
        let config = &state.config;
        let (chip, slow, fast, _) = state.physical_memory.storage();
        let slow_size = match config.slow_ram() {
            SlowRamSize::None => None,
            SlowRamSize::Kb512 => Some(SLOW_RAM_SIZE),
        };
        let fast_size = match config.fast_ram() {
            FastRamSize::None => None,
            FastRamSize::Mb4 => Some(MAX_FAST_RAM_SIZE),
        };
        if *config != A500Config::from_preset(config.active_preset(), config.video_standard())
            || *config != state.physical_memory.config
        {
            return Err(SaveStateError::CorruptedData(
                "Inconsistent hardware configuration".into(),
            ));
        }
        if chip.0.len() != CHIP_RAM_SIZE_512K
            || slow.0.as_ref().map(|bank| bank.0.len()) != slow_size
            || fast.0.as_ref().map(|bank| bank.0.len()) != fast_size
        {
            return Err(SaveStateError::CorruptedData(
                "RAM bank sizes do not match the hardware configuration".into(),
            ));
        }
        let frame = &state.denise.frame_builder;
        if !frame.has_geometry(config.video_standard())
            || frame.storage().0.len() != FRAME_BUFFER_CAPACITY
        {
            return Err(SaveStateError::CorruptedData(
                "Invalid frame buffer geometry".into(),
            ));
        }
        for (index, drive) in state.floppy.drives.iter().enumerate() {
            if (!drive.connected && (drive.disk_inserted || index == 0))
                || drive.disk_inserted != drive.tracks().0.is_some()
                || (!drive.disk_inserted && self.header.association.mounted[index].is_some())
            {
                return Err(SaveStateError::CorruptedData(format!(
                    "Inconsistent media for DF{index}"
                )));
            }
        }
        Ok(())
    }

    /// Serializes the complete save file to a JSON string.
    pub fn to_json(&self) -> Result<String, SaveStateError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|e| SaveStateError::SerializationFailed(e.to_string()))
    }

    /// Serializes the complete save file to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, SaveStateError> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|e| SaveStateError::SerializationFailed(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self, SaveStateError> {
        Self::from_bytes(json.as_bytes())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SaveStateError> {
        let Envelope { header, state } =
            serde_json::from_slice::<Envelope<&RawValue>>(bytes).map_err(deserialization_error)?;
        Self::decode(header, state)
    }

    fn decode(header: &RawValue, state: Option<&RawValue>) -> Result<Self, SaveStateError> {
        let version = serde_json::from_str::<VersionHeader>(header.get())
            .map_err(deserialization_error)?
            .version;
        if version != SAVE_STATE_VERSION {
            return Err(SaveStateError::IncompatibleVersion {
                found: version,
                supported: SAVE_STATE_VERSION,
            });
        }
        let state = state
            .ok_or_else(|| SaveStateError::DeserializationFailed("Missing machine state".into()))?;
        let saved = Self {
            header: serde_json::from_str(header.get()).map_err(deserialization_error)?,
            state: serde_json::from_str(state.get()).map_err(deserialization_error)?,
        };
        saved.validate()?;
        Ok(saved)
    }
}

fn deserialization_error(error: serde_json::Error) -> SaveStateError {
    SaveStateError::DeserializationFailed(error.to_string())
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
