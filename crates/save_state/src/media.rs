//! File references to verified source images and the guest's modified tracks.

use crate::encoding::{decode, encode, read_string};
use crate::SaveStateError;
use machine_loop::config::FLOPPY_DRIVE_COUNT;
use machine_loop::floppy::{
    FloppyDisk, Track, FORMATTED_DISK_BYTES, RECORDED_TRACK_BYTES, TRACKS_PER_DISK, TRACK_BYTES,
};
use machine_loop::physical_memory::{validate_kickstart_size, KickstartBytes, KICKSTART_ROM_SIZE};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

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
pub(super) struct RomReference {
    sha256: Sha256Hex,
    name: Option<String>,
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
                name: source.name.clone(),
                synthetic: false,
            });
        }
        if rom == KickstartBytes::synthetic().as_slice() {
            return Ok(Self {
                sha256: Sha256Hex::of(rom),
                name: None,
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
        let asset = describe("Kickstart ROM", self.name.as_deref());
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
pub(super) struct DiskReference {
    sha256: Sha256Hex,
    name: Option<String>,
    modified_tracks: Replacements,
}

impl DiskReference {
    pub(super) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(super) fn capture(
        drive: usize,
        actual: &[Track],
        source: Option<&SourceImage>,
    ) -> Result<Self, SaveStateError> {
        let asset = format!("ADF for DF{drive}");
        if actual.len() != TRACKS_PER_DISK {
            return Err(SaveStateError::CorruptedData(format!(
                "Incorrect track count for DF{drive}"
            )));
        }
        let source = source.ok_or_else(|| unavailable(&asset))?;
        let original = prepare(&asset, source)?;
        let modified_tracks = actual
            .iter()
            .zip(&original)
            .enumerate()
            .filter(|(_, (actual, original))| actual != original)
            .map(|(index, (actual, _))| (index, actual.clone()))
            .collect();
        Ok(Self {
            sha256: Sha256Hex::of(&source.bytes),
            name: source.name.clone(),
            modified_tracks: Replacements(modified_tracks),
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
        let asset = describe(&format!("ADF for DF{drive}"), self.name.as_deref());
        let source = source.ok_or_else(|| unavailable(&asset))?;
        if source.bytes.len() == FORMATTED_DISK_BYTES && !self.matches(&source.bytes) {
            return Err(mismatch(&asset));
        }
        let mut tracks = prepare(&asset, source)?;
        for (&index, track) in &self.modified_tracks.0 {
            tracks[index] = track.clone();
        }
        Ok(tracks)
    }
}

fn prepare(asset: &str, source: &SourceImage) -> Result<Vec<Track>, SaveStateError> {
    FloppyDisk::from_adf(&source.bytes)
        .map(FloppyDisk::into_tracks)
        .ok_or_else(|| {
            SaveStateError::MediaUnavailable(format!(
                "{asset} must contain {FORMATTED_DISK_BYTES} bytes; received {} bytes",
                source.bytes.len(),
            ))
        })
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Replacements(BTreeMap<usize, Track>);

impl Serialize for Replacements {
    fn serialize<S: Serializer>(&self, encoder: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = encoder.serialize_map(Some(self.0.len()))?;
        for (index, track) in &self.0 {
            map.serialize_entry(index, &TrackString(track))?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Replacements {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Tracks;
        impl<'de> de::Visitor<'de> for Tracks {
            type Value = Replacements;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("modified tracks keyed by track index")
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut tracks = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let index = key
                        .parse::<usize>()
                        .ok()
                        .filter(|&index| index < TRACKS_PER_DISK && index.to_string() == key)
                        .ok_or_else(|| de::Error::custom("Invalid modified track index"))?;
                    if tracks.contains_key(&index) {
                        return Err(de::Error::custom("Duplicate modified track index"));
                    }
                    tracks.insert(index, map.next_value::<SavedTrack>()?.0);
                }
                Ok(Replacements(tracks))
            }
        }

        decoder.deserialize_map(Tracks)
    }
}

struct TrackString<'a>(&'a Track);

impl Serialize for TrackString<'_> {
    fn serialize<S: Serializer>(&self, encoder: S) -> Result<S::Ok, S::Error> {
        encode(&self.0.snapshot_bytes(), encoder)
    }
}

struct SavedTrack(Track);

impl<'de> Deserialize<'de> for SavedTrack {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        read_string(decoder, |text| {
            let bytes = decode(text, &[TRACK_BYTES, RECORDED_TRACK_BYTES])?;
            Track::from_snapshot_bytes(&bytes)
                .map(Self)
                .ok_or_else(|| de::Error::custom("Invalid modified track representation"))
        })
    }
}
