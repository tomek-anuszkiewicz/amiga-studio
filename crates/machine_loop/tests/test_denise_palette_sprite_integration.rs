//! Denise Palette & Sprite Machine Loop Integration Tests
//!
//! Tests full 32-entry color palette batch writes through the mutation delay pipeline,
//! hardware sprite channel position comparators, sprite data arming, and DMA activation.

mod common;
use common::MachineHarness;

#[test]
fn test_denise_full_32_color_palette_batch_mutation() {
    let mut harness = MachineHarness::new();

    // 1. Write unique RGB values into all 32 hardware color registers (COLOR00..COLOR31 at $DFF180..$DFF1BE)
    for i in 0..32u16 {
        let reg_offset = 0x180 + i * 2;
        let color_val = 0x0100 + i; // Distinct 12-bit RGB444 color
        harness.machine.dispatch_custom_write(reg_offset, color_val);
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
fn test_sprite_channel_vertical_window_and_data_arming() {
    let mut harness = MachineHarness::new();

    // Sprite 0: VSTART = 40, VSTOP = 55, HSTART = 120 ($78)
    // SPR0POS ($140): VSTART in bits 15..8 (40 = $28), HSTART in bits 7..0 ($78) -> $2878
    // SPR0CTL ($142): VSTOP in bits 15..8 (55 = $37), bits 7..0 = 0 -> $3700
    harness.machine.dispatch_custom_write(0x140, 0x2878);
    harness.machine.dispatch_custom_write(0x142, 0x3700);
    harness.step_cck(2);

    assert_eq!(harness.machine.denise.sprites.channels[0].vstart(), 40);
    assert_eq!(harness.machine.denise.sprites.channels[0].vstop(), 55);
    assert_eq!(harness.machine.denise.sprites.channels[0].hstart(), 240);

    // Writing CTL disarms the channel
    assert!(!harness.machine.denise.sprites.channels[0].is_armed);

    // Arm Sprite 0 by writing image data (SPR0DATA at $144, SPR0DATB at $146)
    harness.machine.dispatch_custom_write(0x144, 0xCCCC);
    harness.machine.dispatch_custom_write(0x146, 0x3333);
    harness.step_cck(2);

    // Channel should now be armed
    assert!(harness.machine.denise.sprites.channels[0].is_armed);
    assert_eq!(harness.machine.denise.sprites.channels[0].data_a, 0xCCCC);
    assert_eq!(harness.machine.denise.sprites.channels[0].data_b, 0x3333);

    // Step scanlines to line 30: before vertical start, is_active_line must be false
    harness.step_until_vpos(30, 8000);
    assert!(
        !harness.machine.denise.sprites.channels[0].is_active_line,
        "Sprite channel should be inactive before VSTART"
    );

    // Step scanlines into the vertical window: line 45
    harness.step_until_vpos(45, 8000);
    assert!(
        harness.machine.denise.sprites.channels[0].is_active_line,
        "Sprite channel should be active within [VSTART..VSTOP)"
    );

    // Step scanlines past vertical stop: line 60
    harness.step_until_vpos(60, 8000);
    assert!(
        !harness.machine.denise.sprites.channels[0].is_active_line,
        "Sprite channel should be inactive after VSTOP"
    );
}

#[test]
fn test_sprite_dma_toggle_disarms_channels() {
    let mut harness = MachineHarness::new();

    // Enable Sprite DMA via DMACON: Master Enable + SPREN -> $8220
    harness.machine.dispatch_custom_write(0x096, 0x8220);
    harness.step_cck(2);
    assert!(harness.machine.denise.sprites.dma_enabled);

    // Arm Sprite 0
    harness.machine.dispatch_custom_write(0x140, 0x1020);
    harness.machine.dispatch_custom_write(0x144, 0xAAAA);
    harness.step_cck(2);
    assert!(harness.machine.denise.sprites.channels[0].is_armed);

    // Disable Sprite DMA via DMACON ($DFF096): write $0020 (clear SPREN)
    harness.machine.dispatch_custom_write(0x096, 0x0020);
    harness.step_cck(2);

    // When Sprite DMA is disabled via DMACON, sprite channels should disarm
    assert!(!harness.machine.denise.sprites.dma_enabled);
    assert!(
        !harness.machine.denise.sprites.channels[0].is_armed,
        "Disabling sprite DMA must disarm sprite channels"
    );
}
