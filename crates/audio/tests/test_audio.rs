use audio::{Audio, StereoSample, AUDIO_RING_BUFFER_CAPACITY};

#[test]
fn test_audio_channel_configuration_and_reset() {
    let mut audio = Audio::new();
    audio.set_loc(0, 0x00020000);
    audio.set_len(0, 128);
    audio.set_per(0, 350);
    audio.set_vol(0, 100); // Exceeds 64, should be clamped
    audio.set_dat(0, 0x1234);

    assert_eq!(audio.channels[0].lc, 0x00020000);
    assert_eq!(audio.channels[0].len, 128);
    assert_eq!(audio.channels[0].per, 350);
    assert_eq!(audio.channels[0].vol, 64);
    assert!(audio.channels[0].active);

    audio.reset();
    assert!(!audio.channels[0].active);
    assert_eq!(audio.channels[0].vol, 0);
    assert_eq!(audio.channels[0].len, 0);
    assert_eq!(audio.samples_available(), 0);
}

#[test]
fn test_audio_pcm_sample_streaming_and_period() {
    let mut audio = Audio::new();
    audio.set_per(0, 4); // Period = 4 CCK ticks
    audio.set_vol(0, 64); // Full volume (64/64 = 1.0)
                          // 2 signed 8-bit samples: +127 (0x7F) and -128 (0x80)
    audio.set_dat(0, 0x7F80);

    // Initial state before clock tick
    assert_eq!(audio.channels[0].current_sample, 0);

    // Step 4 CCKs: first sample byte (+127) should become active
    for _ in 0..4 {
        audio.step_cck();
    }
    assert_eq!(audio.channels[0].current_sample, 127);
    assert_eq!(audio.channels[0].output_scaled(), 127);

    // Step another 4 CCKs: second sample byte (-128) should become active
    for _ in 0..4 {
        audio.step_cck();
    }
    assert_eq!(audio.channels[0].current_sample, -128);
    assert_eq!(audio.channels[0].output_scaled(), -128);

    // In manual mode (non-DMA), completing the 2nd sample requests an interrupt
    assert!(audio.poll_channel_irq(0));
}

#[test]
fn test_audio_volume_scaling() {
    let mut audio = Audio::new();
    audio.set_per(0, 1);
    audio.set_dat(0, 0x40C0); // Sample 1 = +64, Sample 2 = -64

    // Full volume (64): +64 * 64 / 64 = +64
    audio.set_vol(0, 64);
    audio.step_cck();
    assert_eq!(audio.channels[0].current_sample, 64);
    assert_eq!(audio.channels[0].output_scaled(), 64);

    // Half volume (32): +64 * 32 / 64 = +32
    audio.set_vol(0, 32);
    assert_eq!(audio.channels[0].output_scaled(), 32);

    // Zero volume: 0
    audio.set_vol(0, 0);
    assert_eq!(audio.channels[0].output_scaled(), 0);

    // Step to second sample (-64) at volume 32: -64 * 32 / 64 = -32
    audio.set_vol(0, 32);
    audio.step_cck();
    assert_eq!(audio.channels[0].current_sample, -64);
    assert_eq!(audio.channels[0].output_scaled(), -32);
}

#[test]
fn test_audio_adkcon_volume_and_period_modulation() {
    let mut audio = Audio::new();

    // 1. Volume modulation: Channel 0 modulates Channel 1 (ADKCON bit 0 USE0V1)
    audio.set_adkcon(1 << 0);
    audio.load_word(0, 0x0028); // Value 40
    assert_eq!(audio.channels[1].vol, 40);

    // 2. Period modulation: Channel 0 modulates Channel 1 (ADKCON bit 4 USE0P1)
    audio.set_adkcon(1 << 4);
    audio.load_word(0, 440);
    assert_eq!(audio.channels[1].per, 440);
    assert_eq!(audio.channels[1].counter, 440);

    // 3. Channel 2 modulating Channel 3 period (ADKCON bit 6 USE2P3)
    audio.set_adkcon(1 << 6);
    audio.load_word(2, 220);
    assert_eq!(audio.channels[3].per, 220);
}

#[test]
fn test_audio_dma_looping_and_interrupt() {
    let mut audio = Audio::new();
    let mut chip_ram = vec![0u8; 0x10000];

    // Prepare 2 words (4 samples) in Chip RAM at 0x1000:
    // Word 0: 0x1020 (samples 16, 32)
    // Word 1: 0x3040 (samples 48, 64)
    chip_ram[0x1000] = 0x10;
    chip_ram[0x1001] = 0x20;
    chip_ram[0x1002] = 0x30;
    chip_ram[0x1003] = 0x40;

    audio.set_loc(0, 0x1000);
    audio.set_len(0, 2); // 2 words
    audio.set_per(0, 1); // Fast clocking
    audio.set_vol(0, 64);
    audio.set_channel_dma(0, true);

    // Prime initial word fetch
    audio.step_cck_ram(&chip_ram);
    assert_eq!(audio.channels[0].current_sample, 0x10);

    // Next sample: 0x20
    audio.step_cck_ram(&chip_ram);
    assert_eq!(audio.channels[0].current_sample, 0x20);

    // Completing word 0 requests word 1 from DMA
    audio.step_cck_ram(&chip_ram);
    assert_eq!(audio.channels[0].current_sample, 0x30);

    // Next sample: 0x40
    audio.step_cck_ram(&chip_ram);
    assert_eq!(audio.channels[0].current_sample, 0x40);

    // Buffer length exhausted: Level 4 IRQ asserted and restart strobe set
    assert!(audio.poll_channel_irq(0));
    assert!(audio.poll_restart_strobe(0));
    // Pointer reloaded to start
    assert_eq!(audio.channels[0].current_pt, 0x1000);
}

#[test]
fn test_audio_stereo_mixing_and_ring_buffer() {
    let mut audio = Audio::new();

    // Channels 0 & 3 -> Right
    audio.set_per(0, 1);
    audio.set_vol(0, 64);
    audio.set_dat(0, 0x2020); // +32

    audio.set_per(3, 1);
    audio.set_vol(3, 64);
    audio.set_dat(3, 0x1010); // +16
                              // Right total: 32 + 16 = 48

    // Channels 1 & 2 -> Left
    audio.set_per(1, 1);
    audio.set_vol(1, 64);
    audio.set_dat(1, 0xE0E0); // -32 (0xE0 as i8 = -32)

    audio.set_per(2, 1);
    audio.set_vol(2, 64);
    audio.set_dat(2, 0xF0F0); // -16 (0xF0 as i8 = -16)
                              // Left total: -32 + -16 = -48

    // Tick audio
    audio.step_cck();

    assert_eq!(audio.samples_available(), 1);
    let sample = audio.pop_sample().unwrap();
    assert_eq!(sample.left, -48);
    assert_eq!(sample.right, 48);
}

#[test]
fn test_audio_ring_buffer_wrapping() {
    let mut audio = Audio::new();

    for i in 0..(AUDIO_RING_BUFFER_CAPACITY + 10) {
        audio.push_sample(StereoSample {
            left: i as i16,
            right: -(i as i16),
        });
    }

    // Capacity clamped to AUDIO_RING_BUFFER_CAPACITY
    assert_eq!(audio.samples_available(), AUDIO_RING_BUFFER_CAPACITY);

    // First popped sample should be sample 10 (oldest 10 overwritten)
    let first = audio.pop_sample().unwrap();
    assert_eq!(first.left, 10);
    assert_eq!(first.right, -10);
}
