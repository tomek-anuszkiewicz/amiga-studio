//! Unit Tests for Agnus Blitter Bresenham Line Drawing Engine
//!
//! Validates `LineDrawer`, octant direction stepping, error accumulator progression,
//! sign bit updates, and single-point line blits.

use blitter::{Blitter, LineDrawer};

#[test]
fn test_line_drawer_initialization_and_reset() {
    let mut drawer = LineDrawer::new();
    assert!(drawer.first_pixel_on_line);

    drawer.first_pixel_on_line = false;
    assert!(!drawer.first_pixel_on_line);

    drawer.reset();
    assert!(drawer.first_pixel_on_line);
}

#[test]
fn test_line_drawer_step_pixel_horizontal_octant() {
    let mut drawer = LineDrawer::new();

    // Setup: Octant 0 (SUD=0, SUL=0, AUL=0)
    // Primary axis: horizontal (X+), secondary axis: vertical (Y+)
    let mut bltcon0 = 0x0BFA; // USEA, USEC, USED, minterm 0xFA, ASH = 0
    let mut bltcon1 = 0x0001; // LINE=1, SIGN=0, SUD=0, SUL=0, AUL=0
    let mut bltapt = 0x0000; // initial error accumulator
    let bltamod: i16 = -10; // 4 * (dy - dx)
    let bltbmod: i16 = 20; // 4 * dy
    let bltcmod: i16 = 40; // modulo (scanline pitch)
    let mut bltcpt = 0x1000; // memory pointer

    // Step 1: accumulator is 0 (non-negative, SIGN=0)
    drawer.step_pixel(
        &mut bltcon0,
        &mut bltcon1,
        &mut bltapt,
        bltamod,
        bltbmod,
        bltcmod,
        &mut bltcpt,
    );

    // Error accumulator should add bltamod (-10) -> wraps in u32
    assert_eq!(bltapt as i16, -10);
    // Sign bit 6 should now be set because bltapt < 0
    assert_ne!(bltcon1 & 0x0040, 0);

    // Step 2: accumulator is negative (SIGN=1)
    drawer.step_pixel(
        &mut bltcon0,
        &mut bltcon1,
        &mut bltapt,
        bltamod,
        bltbmod,
        bltcmod,
        &mut bltcpt,
    );

    // Error accumulator should add bltbmod (+20) -> -10 + 20 = 10
    assert_eq!(bltapt as i16, 10);
    // Sign bit 6 should now be cleared
    assert_eq!(bltcon1 & 0x0040, 0);
}

#[test]
fn test_line_drawer_shift_wrap_and_pointer_advance() {
    let mut drawer = LineDrawer::new();

    // Start with ASH = 15
    let mut bltcon0 = 0xFBFA; // ASH = 15
    let mut bltcon1 = 0x0001; // LINE=1
    let mut bltapt = 0x0000;
    let bltamod: i16 = 0;
    let bltbmod: i16 = 0;
    let bltcmod: i16 = 40;
    let mut bltcpt = 0x1000;

    // In horizontal mode (SUD=0), stepping X+ with SUD=0 and non-negative error
    drawer.step_pixel(
        &mut bltcon0,
        &mut bltcon1,
        &mut bltapt,
        bltamod,
        bltbmod,
        bltcmod,
        &mut bltcpt,
    );

    // In SUD=0 mode, primary axis adds bltcmod (+40) and secondary axis wraps ASH 15->0 adding (+2)
    // 0x1000 + 40 + 2 = 0x102A (4138)
    let ash = (bltcon0 >> 12) & 0xF;
    assert_eq!(ash, 0);
    assert_eq!(bltcpt, 0x102A);
}

#[test]
fn test_blitter_line_mode_execution() {
    let mut blit = Blitter::new();
    let mut ram = vec![0u8; 1024];

    // Configure for line blit
    blit.bltcon0 = 0x0BFA; // Channels A, C, D enabled, minterm 0xFA
    blit.bltcon1 = 0x0001; // LINE mode enabled
    blit.bltapt = 0x0000;
    blit.bltamod = -10;
    blit.bltbmod = 20;
    blit.bltcmod = 40;
    blit.bltcpt = 0x0010;
    blit.bltdpt = 0x0010;
    blit.bltbdat = 0xFFFF; // Solid line pattern
    blit.bltafwm = 0xFFFF;
    blit.bltalwm = 0xFFFF;

    // Start line blit with 10 pixels (length 10)
    blit.start_blit((10 << 6) | 2); // 10 lines, width 2 words
    assert!(blit.is_busy);
    assert_ne!(blit.bltcon1 & 0x0001, 0);

    // Execute line blit
    blit.execute_line_blit(&mut ram);

    // Blitter should have finished and asserted BLITINT
    assert!(!blit.is_busy);
    assert!(blit.poll_blit_irq());
}
