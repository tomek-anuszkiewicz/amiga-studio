//! Chip RAM bus helpers for autonomous DMA channels and custom chip co-processors.
//!
//! Provides canonical Chip RAM address masking, word alignment, Big-Endian decoding/encoding,
//! and bounds safety across Agnus DMA channels (Blitter, Copper, Bitplanes, Sprites, Audio, Disk).

/// Reads a 16-bit Big-Endian word from Chip RAM at the given byte address.
///
/// Hardware invariants:
/// - Addresses are automatically word-aligned (bit 0 cleared).
/// - Out-of-bounds addresses wrap around within the power-of-two Chip RAM allocation mask.
/// - Returns `$FFFF` on empty or inaccessible memory (floating open bus pulled high).
#[inline(always)]
pub fn read_u16(ram: &[u8], addr: u32) -> u16 {
    if ram.is_empty() {
        return 0xFFFF;
    }
    let mask = ram.len().wrapping_sub(1);
    let offset = (addr as usize & mask) & !1;
    if offset + 1 < ram.len() {
        u16::from_be_bytes([ram[offset], ram[offset + 1]])
    } else {
        0xFFFF
    }
}

/// Writes a 16-bit Big-Endian word to Chip RAM at the given byte address.
///
/// Hardware invariants:
/// - Addresses are automatically word-aligned (bit 0 cleared).
/// - Out-of-bounds addresses wrap around within the power-of-two Chip RAM allocation mask.
/// - Writes to empty memory buffers are silently ignored.
#[inline(always)]
pub fn write_u16(ram: &mut [u8], addr: u32, val: u16) {
    if ram.is_empty() {
        return;
    }
    let mask = ram.len().wrapping_sub(1);
    let offset = (addr as usize & mask) & !1;
    if offset + 1 < ram.len() {
        let bytes = val.to_be_bytes();
        ram[offset] = bytes[0];
        ram[offset + 1] = bytes[1];
    }
}
