//! Amiga MFM (Modified Frequency Modulation) Track & Sector Encoding
//!
//! Implements standard 880 KB Double Density AmigaDOS track formatting,
//! split odd/even MFM encoding/decoding, 32-bit XOR checksums, and sync word ($4489) detection.

const HEADER_INFO_MFM_BYTES: usize = 8;
const CHECKSUM_MFM_BYTES: usize = 8;

const MASK_MFM_DATA_BITS: u32 = 0x5555_5555;
const MASK_MFM_CLOCK_BITS: u32 = 0xAAAA_AAAA;
const SECTOR_FORMAT: u8 = 0xFF;
pub(super) const MFM_GAP_BYTE: u8 = 0xAA;
const SYNC_BYTES: usize = 4;
const MFM_SYNC_BYTES: [u8; SYNC_BYTES] = [
    (MFM_SYNC_WORD >> 8) as u8,
    MFM_SYNC_WORD as u8,
    (MFM_SYNC_WORD >> 8) as u8,
    MFM_SYNC_WORD as u8,
];
const LABEL_LONGWORDS: usize = 4;
const LABEL_MFM_LONGWORDS: usize = LABEL_LONGWORDS * 2;
const HEADER_MFM_LONGWORDS: usize = LABEL_MFM_LONGWORDS + 2;
const LABEL_MFM_BYTES: usize = LABEL_MFM_LONGWORDS * 4;
const PAYLOAD_LONGWORDS: usize = SECTOR_DATA_BYTES / 4;
const PAYLOAD_MFM_LONGWORDS: usize = PAYLOAD_LONGWORDS * 2;
const TRAILING_GAP_BYTES: usize = 4;
const ENCODED_SECTOR_BODY_BYTES: usize = 1080;
const TRACK_LEADING_GAP_BYTES: usize = 500;

/// Standard Amiga sync mark word ($4489)
const MFM_SYNC_WORD: u16 = 0x4489;

/// Standard raw MFM sector size in bytes (sync + header + label + checksums + data)
pub const RAW_MFM_SECTOR_BYTES: usize = 1088;

/// Standard Amiga unencoded sector payload size in bytes
use super::{SECTORS_PER_TRACK, SECTOR_DATA_BYTES};

/// Standard number of sectors per track in Double Density disks

/// Standard raw MFM track size in bytes (11 sectors + inter-sector gaps)
pub(crate) const RAW_MFM_TRACK_BYTES: usize = 12668;

/// Errors that can occur during MFM decoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfmError {
    /// Sync pattern ($4489 $4489) was not found
    SyncNotFound,
    /// Sector format byte was not $FF (AmigaDOS standard)
    InvalidFormat,
    /// Header checksum does not match computed XOR sum
    HeaderChecksumMismatch,
    /// Data checksum does not match computed XOR sum
    DataChecksumMismatch,
    /// Input buffer is shorter than required MFM structure
    BufferTooShort,
}

/// Decodes a 32-bit payload longword from odd and even MFM longwords
#[inline]
pub fn decode_mfm_long(odd: u32, even: u32) -> u32 {
    ((odd & MASK_MFM_DATA_BITS) << 1) | (even & MASK_MFM_DATA_BITS)
}

/// Encodes a 32-bit payload longword into odd and even MFM longwords with clock bits
#[inline]
pub fn encode_mfm_long(payload: u32) -> (u32, u32) {
    let odd_data = (payload >> 1) & MASK_MFM_DATA_BITS;
    let even_data = payload & MASK_MFM_DATA_BITS;

    // Standard clock bit synthesis: clock bit is 1 if adjacent data bits are 0
    let odd_clk = (!((odd_data << 1) | (odd_data >> 1))) & MASK_MFM_CLOCK_BITS;
    let even_clk = (!((even_data << 1) | (even_data >> 1))) & MASK_MFM_CLOCK_BITS;

    (odd_data | odd_clk, even_data | even_clk)
}

/// Computes the 32-bit XOR checksum over an array of 32-bit MFM longwords
#[inline]
fn calculate_mfm_checksum(words: &[u32]) -> u32 {
    words.iter().fold(0u32, |acc, &val| acc ^ val)
}

/// Encodes a single 512-byte sector into a standard 1088-byte raw MFM block
pub fn encode_amiga_sector(track: u8, sector: u8, data: &[u8; SECTOR_DATA_BYTES]) -> Vec<u8> {
    let mut out = Vec::with_capacity(RAW_MFM_SECTOR_BYTES);

    // 1. Sync mark: $4489 $4489 (4 bytes)
    out.extend_from_slice(&MFM_SYNC_WORD.to_be_bytes());
    out.extend_from_slice(&MFM_SYNC_WORD.to_be_bytes());

    // 2. Header info: format $FF, track, sector, sectors until gap
    let sectors_to_gap = (SECTORS_PER_TRACK - (sector as usize)) as u8;
    let header_val = u32::from_be_bytes([SECTOR_FORMAT, track, sector, sectors_to_gap]);
    let (header_odd, header_even) = encode_mfm_long(header_val);

    out.extend_from_slice(&header_odd.to_be_bytes());
    out.extend_from_slice(&header_even.to_be_bytes());

    // 3. Sector label: 16 bytes (4 longwords), reserved for OS / recovery
    let mut label_mfm_words = [0u32; LABEL_MFM_LONGWORDS];
    for i in 0..LABEL_LONGWORDS {
        let (odd, even) = encode_mfm_long(0);
        label_mfm_words[i * 2] = odd;
        label_mfm_words[i * 2 + 1] = even;
        out.extend_from_slice(&odd.to_be_bytes());
        out.extend_from_slice(&even.to_be_bytes());
    }

    // 4. Header Checksum: XOR of header and label MFM words
    let mut header_xor_words = [0u32; HEADER_MFM_LONGWORDS];
    header_xor_words[0] = header_odd;
    header_xor_words[1] = header_even;
    header_xor_words[2..HEADER_MFM_LONGWORDS].copy_from_slice(&label_mfm_words);
    let header_checksum = calculate_mfm_checksum(&header_xor_words);
    let (h_chk_odd, h_chk_even) = encode_mfm_long(header_checksum);
    out.extend_from_slice(&h_chk_odd.to_be_bytes());
    out.extend_from_slice(&h_chk_even.to_be_bytes());

    // 5. Encode 512 bytes of data (128 longwords) into odd and even buffers
    let mut data_odd = Vec::with_capacity(PAYLOAD_LONGWORDS);
    let mut data_even = Vec::with_capacity(PAYLOAD_LONGWORDS);

    for chunk in data.chunks_exact(4) {
        let val = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let (odd, even) = encode_mfm_long(val);
        data_odd.push(odd);
        data_even.push(even);
    }

    // 6. Data Checksum: XOR of all 128 odd + 128 even MFM words
    let mut data_xor_words = Vec::with_capacity(PAYLOAD_MFM_LONGWORDS);
    data_xor_words.extend_from_slice(&data_odd);
    data_xor_words.extend_from_slice(&data_even);
    let data_checksum = calculate_mfm_checksum(&data_xor_words);
    let (d_chk_odd, d_chk_even) = encode_mfm_long(data_checksum);
    out.extend_from_slice(&d_chk_odd.to_be_bytes());
    out.extend_from_slice(&d_chk_even.to_be_bytes());

    // 7. Write data payload: 512 bytes odd bits followed by 512 bytes even bits
    for odd in data_odd {
        out.extend_from_slice(&odd.to_be_bytes());
    }
    for even in data_even {
        out.extend_from_slice(&even.to_be_bytes());
    }

    // 8. Trailing gap words (4 bytes zeros)
    out.extend_from_slice(&[MFM_GAP_BYTE; TRAILING_GAP_BYTES]);

    out
}

#[inline]
fn read_be_u32(slice: &[u8], offset: usize) -> u32 {
    if offset + 4 <= slice.len() {
        u32::from_be_bytes([
            slice[offset],
            slice[offset + 1],
            slice[offset + 2],
            slice[offset + 3],
        ])
    } else {
        0
    }
}

/// Decodes an Amiga sector from raw MFM bytes starting at the sync mark
pub fn decode_amiga_sector(raw: &[u8]) -> Result<(u8, u8, [u8; SECTOR_DATA_BYTES]), MfmError> {
    if raw.len() < RAW_MFM_SECTOR_BYTES - TRAILING_GAP_BYTES {
        return Err(MfmError::BufferTooShort);
    }

    // Locate sync mark $4489 $4489
    let mut sync_idx = None;
    for i in 0..(raw.len().saturating_sub(SYNC_BYTES)) {
        let is_sector_sync = raw[i..i + SYNC_BYTES] == MFM_SYNC_BYTES;
        if is_sector_sync {
            sync_idx = Some(i);
            break;
        }
    }

    let start = match sync_idx {
        Some(i) => i + SYNC_BYTES,
        None => return Err(MfmError::SyncNotFound),
    };
    if start + ENCODED_SECTOR_BODY_BYTES > raw.len() {
        return Err(MfmError::BufferTooShort);
    }

    // Decode Header Info (8 bytes: odd long, even long)
    let h_odd = read_be_u32(raw, start);
    let h_even = read_be_u32(raw, start + 4);
    let header_val = decode_mfm_long(h_odd, h_even);
    let header_bytes = header_val.to_be_bytes();

    if header_bytes[0] != SECTOR_FORMAT {
        return Err(MfmError::InvalidFormat);
    }
    let track = header_bytes[1];
    let sector = header_bytes[2];

    // Label (32 bytes = 8 longwords)
    let label_start = start + HEADER_INFO_MFM_BYTES;
    let mut header_xor_words = [0u32; HEADER_MFM_LONGWORDS];
    header_xor_words[0] = h_odd;
    header_xor_words[1] = h_even;
    for i in 0..LABEL_MFM_LONGWORDS {
        let pos = label_start + i * 4;
        header_xor_words[2 + i] = read_be_u32(raw, pos);
    }

    // Header Checksum (8 bytes)
    let chk_start = label_start + LABEL_MFM_BYTES;
    let h_chk_odd = read_be_u32(raw, chk_start);
    let h_chk_even = read_be_u32(raw, chk_start + 4);
    let expected_h_chk = calculate_mfm_checksum(&header_xor_words);
    let actual_h_chk = decode_mfm_long(h_chk_odd, h_chk_even);
    if actual_h_chk != expected_h_chk {
        return Err(MfmError::HeaderChecksumMismatch);
    }

    // Data Checksum (8 bytes)
    let d_chk_start = chk_start + CHECKSUM_MFM_BYTES;
    let d_chk_odd = read_be_u32(raw, d_chk_start);
    let d_chk_even = read_be_u32(raw, d_chk_start + 4);
    let actual_d_chk = decode_mfm_long(d_chk_odd, d_chk_even);

    // Data payload: 512 bytes odd bits (128 longs) + 512 bytes even bits (128 longs)
    let data_start = d_chk_start + CHECKSUM_MFM_BYTES;
    let mut data_xor = Vec::with_capacity(PAYLOAD_MFM_LONGWORDS);
    let mut payload = [0u8; SECTOR_DATA_BYTES];

    for i in 0..PAYLOAD_LONGWORDS {
        let odd_pos = data_start + i * 4;
        let even_pos = data_start + SECTOR_DATA_BYTES + i * 4;
        let odd = read_be_u32(raw, odd_pos);
        let even = read_be_u32(raw, even_pos);
        data_xor.push(odd);
        data_xor.push(even);

        let decoded = decode_mfm_long(odd, even);
        payload[i * 4..i * 4 + 4].copy_from_slice(&decoded.to_be_bytes());
    }

    let expected_d_chk = calculate_mfm_checksum(&data_xor);
    if actual_d_chk != expected_d_chk {
        return Err(MfmError::DataChecksumMismatch);
    }

    Ok((track, sector, payload))
}

/// Encodes an entire 5632-byte unencoded track into a standard raw MFM track buffer
pub(crate) fn encode_amiga_track(track: u8, track_data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(RAW_MFM_TRACK_BYTES);

    // Pre-track gap preamble (approx 500 bytes of MFM clock pattern $AAAA)
    out.extend_from_slice(&[MFM_GAP_BYTE; TRACK_LEADING_GAP_BYTES]);

    for sector in 0..SECTORS_PER_TRACK {
        let start = sector * SECTOR_DATA_BYTES;
        let end = start + SECTOR_DATA_BYTES;
        let mut sector_data = [0u8; SECTOR_DATA_BYTES];
        if end <= track_data.len() {
            sector_data.copy_from_slice(&track_data[start..end]);
        }
        let mfm_sector = encode_amiga_sector(track, sector as u8, &sector_data);
        out.extend_from_slice(&mfm_sector);
    }

    // Trailing gap to fill standard track length
    if out.len() < RAW_MFM_TRACK_BYTES {
        out.resize(RAW_MFM_TRACK_BYTES, MFM_GAP_BYTE);
    }

    out
}
