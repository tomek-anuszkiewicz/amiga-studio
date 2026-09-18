//! Unit tests for Denise pixel decoding routines (HAM6, EHB, Dual Playfield)

use denise::{decode_dual_playfield, decode_ehb, decode_ham6, COLOR_PALETTE_SIZE};

#[test]
fn test_decode_ham6_modes() {
    let mut palette = [0u16; COLOR_PALETTE_SIZE];
    palette[1] = 0x0F00; // Red
    palette[2] = 0x00F0; // Green

    let mut held_rgb = 0x0000;

    // Mode 0: load from palette
    let rgb = decode_ham6(0x01, &palette, &mut held_rgb);
    assert_eq!(rgb, 0x0F00);
    assert_eq!(held_rgb, 0x0F00);

    // Mode 1: modify blue
    let rgb = decode_ham6(0x1A, &palette, &mut held_rgb);
    assert_eq!(rgb, 0x0F0A);
    assert_eq!(held_rgb, 0x0F0A);

    // Mode 2: modify red
    let rgb = decode_ham6(0x27, &palette, &mut held_rgb);
    assert_eq!(rgb, 0x070A);
    assert_eq!(held_rgb, 0x070A);

    // Mode 3: modify green
    let rgb = decode_ham6(0x3C, &palette, &mut held_rgb);
    assert_eq!(rgb, 0x07CA);
    assert_eq!(held_rgb, 0x07CA);
}

#[test]
fn test_decode_ehb_halving() {
    let mut palette = [0u16; COLOR_PALETTE_SIZE];
    palette[1] = 0x0ECE; // R=14, G=12, B=14

    // Plane 5 == 0 -> normal color
    assert_eq!(decode_ehb(0x01, &palette), 0x0ECE);

    // Plane 5 == 1 -> half-bright (R=7, G=6, B=7 -> 0x0767)
    assert_eq!(decode_ehb(0x21, &palette), 0x0767);
}

#[test]
fn test_decode_dual_playfield_priorities() {
    let mut palette = [0u16; COLOR_PALETTE_SIZE];
    palette[0] = 0x0000; // Background
    palette[1] = 0x0F00; // PF1 (planes 0, 2, 4)
    palette[9] = 0x00F0; // PF2 (planes 1, 3, 5 -> index 8 + 1)

    // PF1 only (data = 0b000001)
    assert_eq!(decode_dual_playfield(0b000001, &palette, false), 0x0F00);

    // PF2 only (data = 0b000010)
    assert_eq!(decode_dual_playfield(0b000010, &palette, false), 0x00F0);

    // Both present, PF1 priority (pf2_priority = false)
    assert_eq!(decode_dual_playfield(0b000011, &palette, false), 0x0F00);

    // Both present, PF2 priority (pf2_priority = true)
    assert_eq!(decode_dual_playfield(0b000011, &palette, true), 0x00F0);
}
