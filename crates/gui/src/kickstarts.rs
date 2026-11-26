//! Kickstart identification and firmware supplied to the frontend.

use physical_memory::KICKSTART_ROM_SIZE;
use sha1::Sha1;
use sha2::{Digest, Sha256};

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod view;

#[derive(Debug)]
struct OfficialRelease {
    name: &'static str,
    part: &'static str,
    size: usize,
    sha1: &'static str,
    crc32: &'static str,
}

// WinUAE rommgr.cpp, ee784914c8d61bb503d137e358b22fcc4758a294, ROM ID 6.
const RELEASE: OfficialRelease = OfficialRelease {
    name: "Kickstart 1.3 rev 34.5 (A500/A1000/A2000)",
    part: "315093-02",
    size: 262_144,
    sha1: "891e9a547772fe0c6c19b610baf8bc4ea7fcb785",
    crc32: "c4f0f55f",
};

// DIARY.md records the PAL A500 OCS cross1 boots with this exact input.
const VERIFIED_PAL_BOOT: &str = "ee05862d8102a08436ac4056da7d549db31625c7d47b24dfb7b3c9a5c113ca53";

#[derive(Debug)]
pub struct Entry {
    pub(crate) reference: String,
    filename: String,
    release: Option<&'static OfficialRelease>,
    verified_pal_boot: bool,
    #[cfg(target_arch = "wasm32")]
    pub(crate) bytes: Vec<u8>,
}

impl Entry {
    #[cfg(not(target_arch = "wasm32"))]
    fn native(reference: String, filename: String, bytes: &[u8]) -> Self {
        Self {
            reference,
            filename,
            release: identify(bytes),
            verified_pal_boot: sha256(bytes) == VERIFIED_PAL_BOOT,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn browser(filename: String, bytes: Vec<u8>) -> Result<Self, String> {
        validate_size(&filename, bytes.len() as u64)?;
        let digest = sha256(&bytes);
        Ok(Self {
            reference: format!("sha256:{digest}/{filename}"),
            filename,
            release: identify(&bytes),
            verified_pal_boot: digest == VERIFIED_PAL_BOOT,
            bytes,
        })
    }

    pub(crate) fn name(&self) -> &str {
        self.release.map_or(&self.filename, |release| release.name)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn source_image(&self) -> debugger::SourceImage {
        debugger::SourceImage {
            name: Some(self.filename.clone()),
            bytes: self.bytes.clone(),
        }
    }

    fn status(&self) -> &'static str {
        if self.release.is_some() {
            "Recognized official release"
        } else {
            "Unrecognized / custom"
        }
    }

    fn support(&self) -> &'static str {
        if self.verified_pal_boot {
            "Supported (PAL A500 boot)"
        } else {
            "Emulator support unverified"
        }
    }
}

fn identify(bytes: &[u8]) -> Option<&'static OfficialRelease> {
    (bytes.len() == RELEASE.size && lowercase_hex(&Sha1::digest(bytes)) == RELEASE.sha1)
        .then_some(&RELEASE)
}

fn sha256(bytes: &[u8]) -> String {
    lowercase_hex(&Sha256::digest(bytes))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut hex = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        hex.push(char::from(DIGITS[usize::from(byte >> 4)]));
        hex.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    hex
}

pub fn validate_size(name: &str, actual: u64) -> Result<(), String> {
    if actual == KICKSTART_ROM_SIZE as u64 {
        return Ok(());
    }
    Err(format!(
        "Unsupported Kickstart {name}: expected {KICKSTART_ROM_SIZE} bytes (256 KiB), got {actual} bytes."
    ))
}
