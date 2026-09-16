use blitter::{apply_fill, barrel_shift, eval_minterm, Blitter};

#[test]
fn test_blitter_reset_and_start() {
    let mut blit = Blitter::new();
    assert!(!blit.is_busy);

    blit.start_blit(0x0408); // 16 rows, 8 words
    assert!(blit.is_busy);
    assert_eq!(blit.bltsize, 0x0408);

    blit.reset();
    assert!(!blit.is_busy);
    assert_eq!(blit.bltsize, 0);
}

#[test]
fn test_minterm_logic_truth_tables() {
    let a = 0xF0F0u16;
    let b = 0xCCCCu16;
    let c = 0xAAAAu16;

    // Standard Copy D = A ($09F0 -> LF = 0xF0)
    assert_eq!(eval_minterm(a, b, c, 0xF0), a);

    // Invert D = !A ($090F -> LF = 0x0F)
    assert_eq!(eval_minterm(a, b, c, 0x0F), !a);

    // Copy D = B ($05CC -> LF = 0xCC)
    assert_eq!(eval_minterm(a, b, c, 0xCC), b);

    // Invert D = !B ($0533 -> LF = 0x33)
    assert_eq!(eval_minterm(a, b, c, 0x33), !b);

    // Copy D = C ($03AA -> LF = 0xAA)
    assert_eq!(eval_minterm(a, b, c, 0xAA), c);

    // Invert D = !C ($0355 -> LF = 0x55)
    assert_eq!(eval_minterm(a, b, c, 0x55), !c);

    // Cookie-cut D = (A & B) | (!A & C) ($0FCA -> LF = 0xCA)
    let expected_cookie = (a & b) | (!a & c);
    assert_eq!(eval_minterm(a, b, c, 0xCA), expected_cookie);

    // Clear D = 0 (LF = 0x00)
    assert_eq!(eval_minterm(a, b, c, 0x00), 0x0000);

    // Set D = 1 (LF = 0xFF)
    assert_eq!(eval_minterm(a, b, c, 0xFF), 0xFFFF);

    // XOR D = A ^ C (LF = 0x5A)
    assert_eq!(eval_minterm(a, b, c, 0x5A), a ^ c);
}

#[test]
fn test_barrel_shifter_and_carry() {
    // Ascending mode shifts
    let w0 = 0x1234u16;
    let w1 = 0x5678u16;

    // Shift 0 bits
    assert_eq!(barrel_shift(w0, 0, 0, false), w0);

    // Shift 4 bits: word 0 has old=0, new=0x1234 -> (0x0000_1234 >> 4) = 0x0123
    let r0 = barrel_shift(w0, 0, 4, false);
    assert_eq!(r0, 0x0123);

    // Shift 4 bits: word 1 has old=0x1234, new=0x5678 -> (0x1234_5678 >> 4) = 0x4567
    let r1 = barrel_shift(w1, w0, 4, false);
    assert_eq!(r1, 0x4567);

    // Descending mode shift 4 bits
    // Word 0: (w0 << 16 | 0) >> 12 = 0x2340
    let rd0 = barrel_shift(w0, 0, 4, true);
    assert_eq!(rd0, 0x2340);

    // Word 1: (w1 << 16 | w0) >> 12 = 0x6781
    let rd1 = barrel_shift(w1, w0, 4, true);
    assert_eq!(rd1, 0x6781);
}

#[test]
fn test_first_and_last_word_masking() {
    let mut ram = vec![0u8; 1024];

    // Source data at 0x100: 3 words of 0xFFFF
    ram[0x100..0x106].copy_from_slice(&[0xFF; 6]);

    let mut blit = Blitter::new();
    blit.bltcon0 = 0x09F0; // USEA, USED, LF = 0xF0 (Copy A)
    blit.bltcon1 = 0;
    blit.bltafwm = 0x00FF;
    blit.bltalwm = 0xFF00;
    blit.bltapt = 0x100;
    blit.bltdpt = 0x200;
    blit.bltsize = (1 << 6) | 3; // 1 row, 3 words

    blit.execute_blit(&mut ram);

    // Word 0 masked by BLTAFWM (0x00FF)
    assert_eq!(u16::from_be_bytes([ram[0x200], ram[0x201]]), 0x00FF);
    // Word 1 unmasked (0xFFFF)
    assert_eq!(u16::from_be_bytes([ram[0x202], ram[0x203]]), 0xFFFF);
    // Word 2 masked by BLTALWM (0xFF00)
    assert_eq!(u16::from_be_bytes([ram[0x204], ram[0x205]]), 0xFF00);

    // Single-word row: both masks combined
    blit.bltapt = 0x100;
    blit.bltdpt = 0x300;
    blit.bltsize = (1 << 6) | 1; // 1 row, 1 word
    blit.execute_blit(&mut ram);

    // 0x00FF & 0xFF00 = 0x0000
    assert_eq!(u16::from_be_bytes([ram[0x300], ram[0x301]]), 0x0000);
}

#[test]
fn test_area_copy_ascending_with_modulos() {
    let mut ram = vec![0u8; 1024];

    // Source 4x4 word grid at 0x100 (stride 8 bytes per row)
    // Row 0: 0x1111, 0x2222, 0xAAAA, 0xBBBB
    // Row 1: 0x3333, 0x4444, 0xCCCC, 0xDDDD
    let src = [
        0x1111u16.to_be_bytes(),
        0x2222u16.to_be_bytes(),
        0xAAAAu16.to_be_bytes(),
        0xBBBBu16.to_be_bytes(),
        0x3333u16.to_be_bytes(),
        0x4444u16.to_be_bytes(),
        0xCCCCu16.to_be_bytes(),
        0xDDDDu16.to_be_bytes(),
    ];
    for (i, pair) in src.iter().enumerate() {
        ram[0x100 + i * 2..0x102 + i * 2].copy_from_slice(pair);
    }

    let mut blit = Blitter::new();
    blit.bltcon0 = 0x09F0; // USEA, USED, Copy A
    blit.bltcon1 = 0;
    blit.bltafwm = 0xFFFF;
    blit.bltalwm = 0xFFFF;
    blit.bltamod = 4; // Stride 8 bytes - 2 words (4 bytes) = 4 bytes modulo
    blit.bltdmod = 4;
    blit.bltapt = 0x100;
    blit.bltdpt = 0x200;
    blit.bltsize = (2 << 6) | 2; // 2 rows, 2 words

    blit.execute_blit(&mut ram);

    // Row 0
    assert_eq!(u16::from_be_bytes([ram[0x200], ram[0x201]]), 0x1111);
    assert_eq!(u16::from_be_bytes([ram[0x202], ram[0x203]]), 0x2222);
    // Row 1 (at offset 0x200 + 4 + 4 = 0x208)
    assert_eq!(u16::from_be_bytes([ram[0x208], ram[0x209]]), 0x3333);
    assert_eq!(u16::from_be_bytes([ram[0x20A], ram[0x20B]]), 0x4444);

    // Pointer values after 2 rows
    assert_eq!(blit.bltapt, 0x110);
    assert_eq!(blit.bltdpt, 0x210);
    assert!(!blit.is_busy);
    assert!(blit.blit_irq);
}

#[test]
fn test_area_copy_descending_overlapping() {
    let mut ram = vec![0u8; 1024];

    // Source data at 0x100: [0x1111, 0x2222, 0x3333, 0x4444]
    let src = [0x1111u16, 0x2222, 0x3333, 0x4444];
    for (i, val) in src.iter().enumerate() {
        ram[0x100 + i * 2..0x102 + i * 2].copy_from_slice(&val.to_be_bytes());
    }

    // Shift data 2 bytes forward (overlapping destination at 0x102)
    // In ascending mode this would overwrite 0x2222 with 0x1111 before reading 0x2222.
    // In descending mode (DESC = 1), start at end (0x106 -> 0x108) and copy backwards.
    let mut blit = Blitter::new();
    blit.bltcon0 = 0x09F0; // USEA, USED, Copy A
    blit.bltcon1 = 0x0002; // DESC = 1
    blit.bltafwm = 0xFFFF;
    blit.bltalwm = 0xFFFF;
    blit.bltamod = 0;
    blit.bltdmod = 0;
    blit.bltapt = 0x106; // Last word of source
    blit.bltdpt = 0x108; // Last word of destination
    blit.bltsize = (1 << 6) | 4; // 1 row, 4 words

    blit.execute_blit(&mut ram);

    // Verify destination data at 0x102..0x10A
    assert_eq!(u16::from_be_bytes([ram[0x102], ram[0x103]]), 0x1111);
    assert_eq!(u16::from_be_bytes([ram[0x104], ram[0x105]]), 0x2222);
    assert_eq!(u16::from_be_bytes([ram[0x106], ram[0x107]]), 0x3333);
    assert_eq!(u16::from_be_bytes([ram[0x108], ram[0x109]]), 0x4444);
}

#[test]
fn test_fill_mode_inclusive_and_exclusive() {
    // 16-bit word with boundary pixels at bit 2 and bit 6
    // Word: 0x0044 = 0b0000_0000_0100_0100
    let data = 0x0044u16;

    // Inclusive Fill: bits 2 through 6 filled -> 0b0000_0000_0111_1100 = 0x007C
    let mut carry = false;
    let incl = apply_fill(data, &mut carry, false);
    assert_eq!(incl, 0x007C);
    assert!(!carry);

    // Exclusive Fill: bits 2 through 5 filled, boundary bit 6 cleared -> 0b0000_0000_0011_1100 = 0x003C
    let mut carry = false;
    let excl = apply_fill(data, &mut carry, true);
    assert_eq!(excl, 0x003C);
    assert!(!carry);
}

#[test]
fn test_bresenham_line_mode() {
    let mut ram = vec![0u8; 1024];

    let mut blit = Blitter::new();
    // Line Mode: BLTCON1 bit 0 = 1, Octant 0: SUD=1, SUL=1, AUL=0 (0x0019)
    blit.bltcon1 = 0x0019;
    // Line Mode: BLTCON0: USEA=1, USEC=1, USED=1, LF=0xCA (0x0BCA)
    blit.bltcon0 = 0x0BCA;
    blit.bltafwm = 0xFFFF;
    blit.bltadat = 0x8000; // Single pixel mask
    blit.bltbdat = 0xFFFF; // Solid line pattern
    blit.bltcmod = 40; // 40 bytes per line (320-pixel display)

    let dx = 10i16;
    let dy = 5i16;
    let apt = 4 * dy - 2 * dx; // 20 - 20 = 0
    blit.bltapt = apt as u16 as u32;
    blit.bltamod = 4 * (dy - dx); // 4 * (-5) = -20
    blit.bltbmod = 4 * dy; // 20
    blit.bltcpt = 0x100;
    blit.bltdpt = 0x100;
    blit.bltsize = (11 << 6) | 2; // Length 11 pixels (dx + 1), H = 2

    blit.execute_blit(&mut ram);

    assert!(!blit.is_busy);
    assert!(blit.blit_irq);
    assert!(!blit.is_zero); // Plotted pixels should clear zero flag
}

#[test]
fn test_zero_detect_flag() {
    let mut ram = vec![0u8; 512];

    let mut blit = Blitter::new();
    blit.bltcon0 = 0x09F0; // Copy A
    blit.bltafwm = 0xFFFF;
    blit.bltalwm = 0xFFFF;
    blit.bltapt = 0x00; // Zeroed memory
    blit.bltdpt = 0x100;
    blit.bltsize = (2 << 6) | 2;

    blit.execute_blit(&mut ram);
    assert!(blit.is_zero);

    // Non-zero source
    ram[0x02] = 0x40;
    blit.bltapt = 0x00;
    blit.bltdpt = 0x100;
    blit.bltsize = (2 << 6) | 2;

    blit.execute_blit(&mut ram);
    assert!(!blit.is_zero);
}

#[test]
fn test_cycle_by_cycle_stepping() {
    let mut ram = vec![0u8; 512];
    ram[0x10] = 0xBE;
    ram[0x11] = 0xEF;

    let mut blit = Blitter::new();
    blit.set_dma_enabled(true);
    blit.bltcon0 = 0x09F0; // USEA, USED (2 channels per word)
    blit.bltafwm = 0xFFFF;
    blit.bltalwm = 0xFFFF;
    blit.bltapt = 0x10;
    blit.bltdpt = 0x40;

    // Start 1 row of 1 word (1 startup cycle + 2 memory cycles)
    blit.start_blit((1 << 6) | 1);
    assert!(blit.is_busy);
    assert!(!blit.poll_blit_irq());

    // Startup Cycle 1: BLT_STRT
    blit.step_cck_ram(&mut ram);
    assert!(blit.is_busy);

    // Word Cycle 1: Fetch A
    blit.step_cck_ram(&mut ram);
    assert!(blit.is_busy);

    // Word Cycle 2: Write D and complete
    blit.step_cck_ram(&mut ram);
    assert!(!blit.is_busy);
    assert!(blit.poll_blit_irq());

    // Verify copied word
    assert_eq!(u16::from_be_bytes([ram[0x40], ram[0x41]]), 0xBEEF);
}

#[test]
fn test_blitter_unconnected_channel_latch_defaults() {
    let mut blit = Blitter::new();
    assert_eq!(blit.bltadat, 0xAAAA);
    assert_eq!(blit.bltbdat, 0xAAAA);
    assert_eq!(blit.bltcdat, 0x5555);
    assert_eq!(blit.anew, 0xAAAA);
    assert_eq!(blit.bnew, 0xAAAA);
    assert_eq!(blit.ahold, 0xAAAA);
    assert_eq!(blit.bhold, 0xAAAA);
    assert_eq!(blit.chold, 0x5555);

    blit.bltadat = 0x1234;
    blit.bltbdat = 0x5678;
    blit.bltcdat = 0x9ABC;
    blit.reset();

    assert_eq!(blit.bltadat, 0xAAAA);
    assert_eq!(blit.bltbdat, 0xAAAA);
    assert_eq!(blit.bltcdat, 0x5555);
    assert_eq!(blit.chold, 0x5555);
}

#[test]
fn test_blitter_unconnected_channels_cookie_cut() {
    let mut blit = Blitter::new();
    let mut ram = vec![0u8; 1024];

    // sblit1 (D only, LF=$CA):
    // When A, B, C are disabled, A=0xAAAA, B=0xAAAA, C=0x5555
    // D = (A & B) | (!A & C) = (0xAAAA & 0xAAAA) | (0x5555 & 0x5555) = 0xAAAA | 0x5555 = 0xFFFF
    blit.set_dma_enabled(true);
    blit.bltcon0 = 0x01CA; // USED only, LF=$CA
    blit.bltafwm = 0xFFFF;
    blit.bltalwm = 0xFFFF;
    blit.bltdpt = 0x20;

    blit.start_blit((1 << 6) | 1);
    // Startup
    blit.step_cck_ram(&mut ram);
    // Phase 0: BusIdle
    blit.step_cck_ram(&mut ram);
    // Phase 1: WriteD
    blit.step_cck_ram(&mut ram);
    assert!(!blit.is_busy);

    let written = u16::from_be_bytes([ram[0x20], ram[0x21]]);
    assert_eq!(written, 0xFFFF);
}
