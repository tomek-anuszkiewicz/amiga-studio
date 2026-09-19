//! Denise Bitplane & Display Window Whole-Machine Integration Tests
//!
//! Verifies display window (DIWSTRT/DIWSTOP) clipping, BPLCON0 bitplane mode configuration,
//! bitplane serialization and palette lookup, and scanline pixel rasterization in the machine loop.

mod common;
use common::MachineHarness;
use denise::frame_builder;

#[test]
fn test_denise_display_window_configuration_and_clipping() {
    let mut harness = MachineHarness::new();

    // Standard PAL window: VSTART = 44 ($2C), VSTOP = 300 ($12C -> $2C with low-byte wrap)
    // HSTART = 129 ($81), HSTOP = 449 ($1C1 -> $C1)
    let diwstrt = 0x2C81;
    let diwstop = 0x2CC1;

    harness.machine.write_custom_word(0x08E, diwstrt);
    harness.machine.write_custom_word(0x090, diwstop);
    harness.step_cck(2);

    assert_eq!(harness.machine.denise.diwstrt, diwstrt);
    assert_eq!(harness.machine.denise.diwstop, diwstop);

    // Verify window coordinate checks on committed registers
    assert!(frame_builder::is_in_display_window(
        130,
        45,
        harness.machine.denise.diwstrt,
        harness.machine.denise.diwstop
    ));
    assert!(frame_builder::is_in_display_window(
        200,
        150,
        harness.machine.denise.diwstrt,
        harness.machine.denise.diwstop
    ));

    // Outside window
    assert!(!frame_builder::is_in_display_window(
        120,
        45,
        harness.machine.denise.diwstrt,
        harness.machine.denise.diwstop
    )); // Left of HSTART
    assert!(!frame_builder::is_in_display_window(
        200,
        40,
        harness.machine.denise.diwstrt,
        harness.machine.denise.diwstop
    )); // Above VSTART
}

#[test]
fn test_denise_bitplane_mode_and_serialization() {
    let mut harness = MachineHarness::new();

    // Configure 1 bitplane in Low-Res (BPU = 1 -> 0x1200)
    harness.machine.write_custom_word(0x100, 0x1200);

    // Setup COLOR00 = Black ($000), COLOR01 = Green ($0F0)
    harness.machine.write_custom_word(0x180, 0x0000);
    harness.machine.write_custom_word(0x182, 0x00F0);
    harness.step_cck(2);

    assert_eq!(harness.machine.denise.bitplane_count(), 1);
    assert!(!harness.machine.denise.is_hires());
    assert_eq!(harness.machine.denise.color[0], 0x0000);
    assert_eq!(harness.machine.denise.color[1], 0x00F0);

    // Write Bitplane 1 data latch (0x8000 -> first pixel 1, next 15 pixels 0)
    harness.machine.write_custom_word(0x110, 0x8000);
    harness.step_cck(2);

    // Load bitplane data into Denise shifters
    let bpldat = harness.machine.denise.bpldat;
    harness.machine.denise.shifters = bpldat;

    // Pixel 0: bit is 1 -> maps to COLOR01 ($00F0)
    let p0 = harness.machine.denise.shift_pixel();
    assert_eq!(p0, 1);
    assert_eq!(harness.machine.denise.decode_pixel(p0), 0x00F0);

    // Pixel 1: bit is 0 -> maps to COLOR00 ($0000)
    let p1 = harness.machine.denise.shift_pixel();
    assert_eq!(p1, 0);
    assert_eq!(harness.machine.denise.decode_pixel(p1), 0x0000);
}

#[test]
fn test_denise_frame_builder_raster_scanline_generation() {
    let mut harness = MachineHarness::new();

    // Set background COLOR00 to Blue ($00F)
    harness.machine.write_custom_word(0x180, 0x000F);
    harness.step_cck(2);

    // Step across to line 30 (active visible scanline outside VBlank lines 0..25)
    harness.step_until_vpos(30, 50_000);
    harness.step_scanlines(1);

    // In linear DAC quantization, RGB444 $00F produces ARGB $FF0000F0
    let expected_argb = frame_builder::rgb444_to_argb32(0x000F);
    assert_eq!(expected_argb, 0xFF00_00F0);

    // Verify FrameBuilder captured the backdrop color on the scanned line (line 30)
    let pixel = harness.machine.denise.frame_builder.get_pixel(200, 30);
    assert_eq!(
        pixel, expected_argb,
        "FrameBuilder should contain rendered backdrop pixels on active scanlines"
    );
}

#[test]
fn test_machine_loop_agnus_bpl_dma_routed_to_denise() {
    let mut harness = MachineHarness::new();

    // Prepare test pattern in Chip RAM at $1000
    harness.machine.physical_memory.chip_ram[0x1000] = 0xBE;
    harness.machine.physical_memory.chip_ram[0x1001] = 0xEF;

    // Point BPL1PTH/L to $1000
    harness.machine.agnus.bplpt[0] = 0x1000;
    // Enable Master DMA + Bitplane DMA ($8300)
    harness.machine.write_custom_word(0x096, 0x8300);
    // 1 bitplane in BPLCON0 ($1200)
    harness.machine.write_custom_word(0x100, 0x1200);
    harness.step_cck(4);

    harness.machine.agnus.ddfstrt = 0x38;
    harness.machine.agnus.ddfstop = 0xD0;
    harness.machine.agnus.hpos = 0x3E;
    harness.machine.agnus.vpos = 50;

    // Step machine 1 CCK: Agnus reaches slot 0x3F (phase 7) and fetches $BEEF for plane 0, routed to Denise
    harness.machine.step_cck();

    assert_eq!(harness.machine.denise.bpldat[0], 0xBEEF);
}
