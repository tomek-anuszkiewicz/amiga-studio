//! Paula Audio Machine Loop Integration Tests
//!
//! Tests audio channel DMA streaming from Chip RAM, period clock division,
//! sample buffer generation, and Level 4 interrupt signaling (AUD0DSR) to the CPU.

mod common;
use common::MachineHarness;

#[test]
fn test_audio_dma_playback_and_interrupt_propagation() {
    let mut harness = MachineHarness::new();

    let sample_addr: u32 = 0x005000;
    let sample_words = [0x7F80, 0x7F80, 0x7F80, 0x7F80]; // +127, -128 alternating

    // 1. Write PCM audio samples into Chip RAM
    for (i, &w) in sample_words.iter().enumerate() {
        harness
            .machine
            .physical_memory
            .write_word_debug(sample_addr + (i as u32) * 2, w);
    }

    // 2. Program Paula Channel 0 registers:
    // AUD0LCH / AUD0LCL ($DFF0A0 / $DFF0A2)
    harness
        .machine
        .write_custom_word(0x0A0, (sample_addr >> 16) as u16);
    harness
        .machine
        .write_custom_word(0x0A2, (sample_addr & 0xFFFF) as u16);
    // AUD0LEN ($DFF0A4): 4 words
    harness.machine.write_custom_word(0x0A4, 4);
    // AUD0PER ($DFF0A6): Period = 5 CCKs per sample byte
    harness.machine.write_custom_word(0x0A6, 5);
    // AUD0VOL ($DFF0A8): Full volume = 64
    harness.machine.write_custom_word(0x0A8, 64);
    // Initial sample word write to prime DAT (AUD0DAT at $DFF0AA)
    harness.machine.write_custom_word(0x0AA, 0x7F80);

    // 3. Enable Audio 0 Level 4 Interrupt in INTENA ($DFF09A): Master + AUD0 bit 7 -> $C080
    harness.machine.write_custom_word(0x09A, 0xC080);

    // 4. Enable Audio 0 DMA in DMACON ($DFF096): Master + AUD0EN bit 0 -> $8201
    harness.machine.write_custom_word(0x096, 0x8201);
    harness.step_cck(2); // Commit pipeline mutations

    // Verify channel is active and configured
    assert_eq!(harness.machine.paula.audio.channels[0].len, 4);
    assert_eq!(harness.machine.paula.audio.channels[0].per, 5);
    assert_eq!(harness.machine.paula.audio.channels[0].vol, 64);

    // 5. Step machine loop across several periods
    let mut cck = 0;
    while (harness.machine.paula.intreq & 0x0080) == 0 {
        harness.machine.step_cck();
        cck += 1;
        assert!(
            cck < 1000,
            "Timed out waiting for Paula Audio Channel 0 interrupt request"
        );
    }

    // 6. Verify Paula INTREQ bit 7 is set and CPU IPL resolves to Level 4
    assert_eq!(
        harness.machine.paula.intreq & 0x0080,
        0x0080,
        "Audio Channel 0 buffer finish did not assert INTREQ bit 7"
    );
    assert_eq!(
        harness.machine.resolve_ipl(),
        4,
        "CPU IPL should be Level 4 following Audio 0 interrupt"
    );

    // 7. Verify audio engine generated output samples into its buffer
    assert!(
        harness.machine.paula.audio.samples_available() > 0,
        "Paula audio engine should have generated audio samples during playback"
    );
}

#[test]
fn test_audio_register_modifications_with_pipeline_delay() {
    let mut harness = MachineHarness::new();

    // Write period 120 and volume 45 to Channel 1 ($0B6, $0B8)
    harness.machine.write_custom_word(0x0B6, 120);
    harness.machine.write_custom_word(0x0B8, 45);

    // Before stepping CCK, values are not committed
    harness.step_cck(2);

    // After 2 CCKs, values must be committed into Paula audio channel 1
    assert_eq!(harness.machine.paula.audio.channels[1].per, 120);
    assert_eq!(harness.machine.paula.audio.channels[1].vol, 45);
}
