#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Denise Bitplane & Display Window Whole-Machine Integration Tests
//!
//! Verifies display window (DIWSTRT/DIWSTOP) clipping, BPLCON0 bitplane mode configuration,
//! bitplane serialization and palette lookup, and scanline pixel rasterization in the machine loop.

use config::custom_reg;
mod common;
use common::MachineHarness;
use denise::frame_builder;
use machine_loop::{A500Machine, A500State};

fn step_to_beam(machine: &mut A500Machine, line: u16, hpos: u16) {
    for _ in 0..32 * 227 {
        if (machine.agnus.vpos, machine.agnus.hpos) == (line, hpos) {
            return;
        }
        machine.step_cck();
    }
    panic!("Agnus did not reach line {line}, CCK {hpos}");
}

#[test]
fn test_kit_dma_draws_the_final_word_across_the_right_edge_on_consecutive_rows() {
    use custom_reg::{agnus as a, denise as d};

    let mut machine = MachineHarness::new().machine;
    machine.cpu.state.set_stopped(true);
    // $20 through $D8 inclusive gives 24 eight-CCK groups, one word per group.
    let base = 0x4000u16;
    let stride = 24 * 2;
    let final_words = [0x0000u16, 0xa6d3, 0x5b2d];
    for (row, word) in final_words.into_iter().enumerate() {
        let address = u32::from(base) + (row as u32 + 1) * stride - 2;
        machine.physical_memory.write_word_debug(address, word);
    }
    for (register, value) in [
        (a::REGISTER_BPL1PTH, 0),
        (a::REGISTER_BPL1PTL, base),
        (d::REGISTER_DIWSTRT, 0x1b51),
        (d::REGISTER_DIWSTOP, 0x37d1),
        (a::REGISTER_DDFSTRT, 0x20),
        (a::REGISTER_DDFSTOP, 0xd8),
        (a::REGISTER_BPL1MOD, 0),
        (a::REGISTER_BPL2MOD, 0),
        (d::REGISTER_BPLCON0, 0x1200),
        (d::REGISTER_BPLCON1, 0),
        (d::REGISTER_COLOR00, 0),
        (d::REGISTER_COLOR01, 0x0fff),
        (a::REGISTER_DMACON, 0x8300),
    ] {
        machine
            .memory_bus()
            .write_custom_word(register, value, "DMA");
    }

    let mut snapshots = Vec::new();
    for (row, word) in final_words.into_iter().enumerate() {
        let line = 27 + row as u16;
        let next_address = u32::from(base) + (row as u32 + 1) * stride;
        step_to_beam(&mut machine, line, 0xde);
        assert_eq!(machine.agnus.bplpt[0], next_address - 2);
        machine.step_cck();
        assert_eq!(machine.agnus.bplpt[0], next_address);
        machine.step_cck();
        assert_eq!(machine.denise.bpldat[0], word);
        if line == 28 {
            // Agnus asserts STRHOR at CCK 1. Denise consumes it one CCK later.
            for hpos in [1, 2] {
                step_to_beam(&mut machine, line + 1, hpos);
                snapshots.push(serde_json::to_vec(&machine.save_state()).unwrap());
            }
        }
    }
    step_to_beam(&mut machine, 30, 20);
    let finished = machine.save_state();
    for json in snapshots {
        let state: A500State = serde_json::from_slice(&json).unwrap();
        let mut restored = MachineHarness::new().machine;
        restored.load_state(&state);
        step_to_beam(&mut restored, 30, 20);
        assert!(
            restored.save_state() == finished,
            "snapshot continuation diverged"
        );
    }

    // The last fetch is at Agnus $DF. The word starts shifting at receiver
    // CCK 223. Its first stored pair is (223 - 49) * 4 + 2 = column 698.
    for (row, word) in [(0, final_words[1]), (2, final_words[2])] {
        for x in 0..720 {
            let set = x >= 698 && word & (0x8000 >> ((x - 698) / 2)) != 0;
            let expected = if set { 0xffffffff } else { 0xff000000 };
            for y in row..row + 2 {
                assert_eq!(
                    machine.denise.frame_builder.get_pixel(x, y),
                    expected,
                    "x={x}, y={y}"
                );
            }
        }
    }
}

#[test]
fn test_display_window_register_configuration_and_geometry() {
    let mut harness = MachineHarness::new();

    // Standard PAL window: VSTART = 44 ($2C), VSTOP = 300 ($12C -> $2C with low-byte wrap)
    // HSTART = 129 ($81), HSTOP = 449 ($1C1 -> $C1)
    let diwstrt = 0x2C81;
    let diwstop = 0x2CC1;

    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_DIWSTRT,
        diwstrt,
        "DMA",
    );
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_DIWSTOP,
        diwstop,
        "DMA",
    );
    harness.step_cck(2);

    assert_eq!(harness.machine.denise.diwstrt, diwstrt);
    assert_eq!(harness.machine.denise.diwstop, diwstop);

    // Verify logical window coordinates from the committed register words.
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
fn test_display_window_vertical_bounds_gate_bitplane_dma() {
    let mut harness = MachineHarness::new();
    let diwstrt = 0x6481; // VSTART = 100, HSTART = 129
    let diwstop = 0xC8C1; // VSTOP = 200, HSTOP = 449

    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_DIWSTRT,
        diwstrt,
        "DMA",
    );
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_DIWSTOP,
        diwstop,
        "DMA",
    );
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_BPLCON0,
        0x1000,
        "DMA",
    );
    harness.machine.memory_bus().write_custom_word(
        custom_reg::agnus::REGISTER_DMACON,
        0x8300,
        "DMA",
    );
    harness.step_cck(5);

    assert_eq!(harness.machine.denise.diwstrt, diwstrt);
    assert_eq!(harness.machine.denise.diwstop, diwstop);

    let dma = &harness.machine.agnus.dma;
    let fetch_slot = 0x3F; // First low-resolution BPL1 slot after DDFSTRT = $38.
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 99), None);
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 100), Some(0));
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 199), Some(0));
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 200), None);

    harness
        .machine
        .physical_memory
        .write_chip_byte_debug((0x1000) as u32, 0xBE);
    harness
        .machine
        .physical_memory
        .write_chip_byte_debug((0x1001) as u32, 0xEF);
    harness.machine.agnus.bplpt[0] = 0x1000;
    harness.machine.agnus.hpos = fetch_slot - 1;
    harness.machine.agnus.vpos = 99;
    harness.machine.step_cck();
    assert_eq!(harness.machine.agnus.bplpt[0], 0x1000);
    assert_eq!(harness.machine.denise.bpldat[0], 0);

    harness.machine.agnus.hpos = fetch_slot - 1;
    harness.machine.agnus.vpos = 100;
    harness.machine.step_cck();
    assert_eq!(harness.machine.agnus.bplpt[0], 0x1002);
    assert_eq!(harness.machine.denise.bpldat[0], 0);
    harness.machine.step_cck();
    assert_eq!(harness.machine.denise.bpldat[0], 0xBEEF);

    // Denise sees the horizontal window earlier than Agnus updates the DMA boundary.
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_DIWSTRT,
        0x2C81,
        "DMA",
    );
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_DIWSTOP,
        0x2CC1,
        "DMA",
    );
    harness.step_cck(1);
    assert_eq!(harness.machine.denise.diwstrt, 0x2C81);
    assert_eq!(harness.machine.agnus.diwstrt, diwstrt);
    let dma = &harness.machine.agnus.dma;
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 99), None);

    harness.step_cck(3);
    assert_eq!(harness.machine.agnus.diwstrt, 0x2C81);
    assert_eq!(harness.machine.agnus.diwstop, 0x2CC1);
    let dma = &harness.machine.agnus.dma;
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 99), Some(0));
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 299), Some(0));
    assert_eq!(dma.bitplane_channel_for_slot(fetch_slot, 300), None);
}

#[test]
fn test_denise_bitplane_mode_and_serialization() {
    let mut harness = MachineHarness::new();

    // Configure 1 bitplane in Low-Res (BPU = 1 -> 0x1200)
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_BPLCON0,
        0x1200,
        "DMA",
    );

    // Setup COLOR00 = Black ($000), COLOR01 = Green ($0F0)
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_COLOR00,
        0x0000,
        "DMA",
    );
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_COLOR01,
        0x00F0,
        "DMA",
    );
    harness.step_cck(2);

    assert_eq!(harness.machine.denise.bplcon0.value(), 0x1200);
    assert!(!harness.machine.denise.is_hires());
    assert_eq!(harness.machine.denise.color[0], 0x0000);
    assert_eq!(harness.machine.denise.color[1], 0x00F0);
    assert_eq!(harness.machine.denise.decode_pixel(0b111110), 0x0000);
    assert_eq!(harness.machine.denise.decode_pixel(0b111111), 0x00F0);

    // Write Bitplane 1 data latch (0x8000 -> first pixel 1, next 15 pixels 0)
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_BPL1DAT,
        0x8000,
        "DMA",
    );
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
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_COLOR00,
        0x000F,
        "DMA",
    );
    harness.step_cck(2);

    // Step across to line 30 (active visible scanline outside VBlank lines 0..25)
    harness.step_until_vpos(30, 50_000);
    harness.step_scanlines(1);

    // Full-range expansion maps RGB444 $00F to opaque blue at 255.
    let expected_argb = frame_builder::rgb444_to_argb32(0x000F);
    assert_eq!(expected_argb, 0xFF00_00FF);

    // Verify FrameBuilder captured the backdrop color on the scanned line (line 30)
    let pixel = harness
        .machine
        .denise
        .frame_builder
        .get_pixel(4, (30 - 28) * 2);
    assert_eq!(
        pixel, expected_argb,
        "FrameBuilder should contain rendered backdrop pixels on active scanlines"
    );
}

#[test]
fn test_machine_loop_agnus_bpl_dma_routed_to_denise() {
    let mut harness = MachineHarness::new();

    // Prepare test pattern in Chip RAM at $1000
    harness
        .machine
        .physical_memory
        .write_chip_byte_debug((0x1000) as u32, 0xBE);
    harness
        .machine
        .physical_memory
        .write_chip_byte_debug((0x1001) as u32, 0xEF);

    // Point BPL1PTH/L to $1000
    harness.machine.agnus.bplpt[0] = 0x1000;
    // Enable Master DMA + Bitplane DMA ($8300)
    harness.machine.memory_bus().write_custom_word(
        custom_reg::agnus::REGISTER_DMACON,
        0x8300,
        "DMA",
    );
    // 1 bitplane in BPLCON0 ($1200)
    harness.machine.memory_bus().write_custom_word(
        custom_reg::denise::REGISTER_BPLCON0,
        0x1200,
        "DMA",
    );
    harness.step_cck(4);

    harness.machine.agnus.dma.ddfstrt = 0x38;
    harness.machine.agnus.dma.ddfstop = 0xD0;
    harness.machine.agnus.hpos = 0x3E;
    harness.machine.agnus.vpos = 50;

    // Step machine 1 CCK: Agnus reaches slot 0x3F (phase 7) and fetches $BEEF for plane 0, routed to Denise
    harness.machine.step_cck();

    assert_eq!(harness.machine.denise.bpldat[0], 0);
    harness.machine.step_cck();
    assert_eq!(harness.machine.denise.bpldat[0], 0xBEEF);
}
