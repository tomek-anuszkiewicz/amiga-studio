#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Denise Palette & Sprite Machine Loop Integration Tests
//!
//! Tests full 32-entry color palette batch writes through the mutation delay pipeline,
//! hardware sprite channel position comparators, sprite data arming, and DMA activation.

use config::custom_reg;
mod common;
use common::MachineHarness;

#[test]
fn sprite_decode_restore_preserves_pending_writes_and_disarmed_serializers() {
    let mut d = MachineHarness::new().machine.denise;
    d.set_sprpos(0, 40);
    d.set_sprpos(1, 40);
    d.set_sprctl(1, 0x81);
    d.set_sprdata(0, 0xffff);
    d.set_sprdata(1, 0xffff);
    d.set_sprdatb(1, 0xffff);
    d.set_sprpos(3, 40);
    d.set_sprdata(3, 0xffff);
    d.step_cck();
    assert_eq!(d.sprites.channels()[1].hstart(), 81);
    assert_eq!(
        d.sprites
            .evaluate_pixel(80, &d.clxcon, &mut d.clxdat)
            .unwrap()
            .color_index,
        17
    );
    assert_eq!(
        d.sprites
            .evaluate_pixel(81, &d.clxcon, &mut d.clxdat)
            .unwrap()
            .color_index,
        29
    );
    assert_eq!(d.sprites.collision_pairs, 1);

    d.set_sprctl(1, 0);
    d.set_sprpos(0, 48);
    d.set_clxcon(0x2000);
    let mut restored: denise::Denise =
        serde_json::from_slice(&serde_json::to_vec(&d).unwrap()).unwrap();
    assert_eq!(d, restored);
    // The queued writes have not changed the cached comparator or ATTACH yet.
    assert_eq!(d.sprites.channels()[0].hstart(), 80);
    assert_eq!(d.sprites.channels()[1].hstart(), 81);
    d.step_cck();
    restored.step_cck();
    assert_eq!(d, restored);
    assert_eq!(d.sprites.channels()[0].hstart(), 96);
    assert_eq!(d.sprites.channels()[1].hstart(), 80);
    assert!(!d.sprites.channels()[1].is_armed);
    assert!(d.sprites.channels()[1].pixel_counter > 0);
    // Restore again with an unarmed channel still shifting, and changed CLXCON.
    restored = serde_json::from_slice(&serde_json::to_vec(&d).unwrap()).unwrap();
    assert_eq!(d, restored);
    for horizontal in 82..115 {
        let actual = d
            .sprites
            .evaluate_pixel(horizontal, &d.clxcon, &mut d.clxdat);
        let resumed =
            restored
                .sprites
                .evaluate_pixel(horizontal, &restored.clxcon, &mut restored.clxdat);
        assert_eq!(actual, resumed);
        assert_eq!(d, restored);
        if horizontal == 82 {
            assert_eq!(actual.unwrap().color_index, 17);
            assert_eq!(d.sprites.collision_pairs, 3);
            assert_eq!(d.clxdat & 0x200, 0x200);
        }
        if horizontal == 96 {
            assert_eq!(actual.unwrap().color_index, 17);
            assert_eq!(d.sprites.channels()[0].pixel_counter, 15);
        }
    }
    d.reset();
    assert_eq!(d.sprites, denise::Sprites::new());
    assert_eq!(d.clxcon.value(), 0);
}

#[test]
fn test_denise_full_32_color_palette_batch_mutation() {
    let mut harness = MachineHarness::new();

    // 1. Write unique RGB values into all 32 hardware color registers (COLOR00..COLOR31 at $DFF180..$DFF1BE)
    for i in 0..32u16 {
        let reg_offset = custom_reg::denise::REGISTER_COLOR00 + i * 2;
        let color_val = 0x0100 + i; // Distinct 12-bit RGB444 color
        harness
            .machine
            .memory_bus()
            .write_custom_word(reg_offset, color_val);
    }

    // 2. Step 2 Color Clocks to allow the delayed mutation pipeline to mature and commit
    harness.step_cck(2);

    // 3. Verify that all 32 colors committed successfully without any entry being dropped
    for i in 0..32 {
        let expected = 0x0100 + i as u16;
        let actual = harness.machine.denise.color[i];
        assert_eq!(
            actual, expected,
            "Denise COLOR{:02} mismatch after batch write: expected 0x{:04X}, found 0x{:04X}",
            i, expected, actual
        );
    }
}

#[test]
fn test_manual_sprite_repeats_outside_dma_vertical_window() {
    let mut harness = MachineHarness::new();

    // Sprite 0: VSTART = 40, VSTOP = 55, HSTART = 120 ($78)
    // SPR0POS ($140): VSTART in bits 15..8 (40 = $28), HSTART in bits 7..0 ($78) -> $2878
    // SPR0CTL ($142): VSTOP in bits 15..8 (55 = $37), bits 7..0 = 0 -> $3700
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_SPR0POS, 0x2878);
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_SPR0CTL, 0x3700);
    harness.step_cck(2);

    assert_eq!(harness.machine.denise.sprites.channels()[0].vstart(), 40);
    assert_eq!(harness.machine.denise.sprites.channels()[0].vstop(), 55);
    assert_eq!(harness.machine.denise.sprites.channels()[0].hstart(), 240);

    // Writing CTL disarms the channel
    assert!(!harness.machine.denise.sprites.channels()[0].is_armed);

    // Arm Sprite 0 by writing image data (SPR0DATA at $144, SPR0DATB at $146)
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_SPR0DATA, 0xCCCC);
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_SPR0DATB, 0x3333);
    harness.step_cck(2);

    // Channel should now be armed
    assert!(harness.machine.denise.sprites.channels()[0].is_armed);
    assert_eq!(harness.machine.denise.sprites.channels()[0].data_a, 0xCCCC);
    assert_eq!(harness.machine.denise.sprites.channels()[0].data_b, 0x3333);

    // HRM 4, Manual Mode: DATA arms output on every line, independently of VSTART/VSTOP.
    // Agnus DMA owns the vertical comparisons; this test supplies data manually.
    for vertical in [30, 45, 60] {
        harness.step_until_vpos(vertical, 8000);
        let denise = &mut harness.machine.denise;
        let pixel = denise
            .sprites
            .evaluate_pixel(240, &denise.clxcon, &mut denise.clxdat);
        assert_eq!(pixel.map(|value| value.color_index), Some(17));
        assert!(denise.sprites.channels()[0].is_armed);
    }
}

#[test]
fn test_sprite_dma_toggle_disarms_channels() {
    let mut harness = MachineHarness::new();

    // Enable Sprite DMA via DMACON: Master Enable + SPREN -> $8220
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x8220);
    harness.step_cck(2);
    assert!(harness
        .machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_SPREN));

    // Arm Sprite 0
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_SPR0POS, 0x1020);
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_SPR0DATA, 0xAAAA);
    harness.step_cck(2);
    assert!(harness.machine.denise.sprites.channels()[0].is_armed);

    // Disable Sprite DMA via DMACON ($DFF096): write $0020 (clear SPREN)
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x0020);
    harness.step_cck(2);
    assert!(!harness
        .machine
        .agnus
        .is_dma_enabled(config::mask::dmacon::BIT_SPREN));

    // Writing SPR0CTL ($DFF142) disarms Sprite 0 per hardware silicon spec
    harness
        .machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::REGISTER_SPR0CTL, 0x0000);
    harness.step_cck(2);
    assert!(
        !harness.machine.denise.sprites.channels()[0].is_armed,
        "Writing SPR0CTL must disarm sprite 0"
    );
}
