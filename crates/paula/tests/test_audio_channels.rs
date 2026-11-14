use paula::audio::{Audio, AudioState, StereoSample};

#[test]
fn all_channels_play_signed_high_then_low_with_latched_volume() {
    for channel in 0..4 {
        for volume in [0u8, 1, 32, 63, 64, 65, 127] {
            let mut audio = Audio::default();
            audio.set_per(channel, 3);
            audio.set_vol(channel, volume);
            audio.set_dat(channel, 0x807f, false);
            assert!(audio.poll_channel_irq(channel));
            for sample in [-128i16, -128, 127, 127, 127] {
                audio.step_cck(1 << channel);
                assert_eq!(audio.channels[channel].current_sample, sample as i8);
                assert_eq!(
                    audio.channels[channel].output_level(),
                    sample * i16::from(volume.min(64))
                );
            }
            audio.step_cck(1 << channel);
            assert_eq!(audio.channels[channel].state, AudioState::Idle);
            // A pending manual interrupt prevents a new AUDxDAT from starting.
            audio.set_dat(channel, 0x1234, true);
            assert_eq!(audio.channels[channel].state, AudioState::Idle);
            audio.set_dat(channel, 0x1234, false);
            assert_eq!(audio.channels[channel].current_sample, 0x12);
        }
    }
}

#[test]
fn dma_discards_first_word_then_keeps_output_and_reload_latches_independent() {
    let mut audio = Audio::default();
    audio.set_len(0, 3);
    audio.set_per(0, 2);
    audio.set_channel_dma(0, true);
    assert!(audio.channels[0].dma_request);
    assert!(!audio.channels[0].restart_request);
    for _ in 0..20 {
        audio.step_cck(0);
    }
    assert_eq!(audio.channels[0].state, AudioState::FirstWord);
    audio.set_dat(0, 0xdead, false);
    assert_eq!(audio.channels[0].current_sample, 0);
    assert_eq!(audio.channels[0].state, AudioState::SecondWord);
    assert!(audio.channels[0].restart_request);
    assert!(audio.poll_channel_irq(0));
    audio.set_len(0, 7);
    assert_eq!(audio.channels[0].words_remaining, 2);
    audio.set_dat(0, 0x1234, true);
    assert_eq!(audio.channels[0].current_sample, 0x12);
    audio.set_dat(0, 0x5678, true);
    assert_eq!(audio.channels[0].words_remaining, 1);
    assert_eq!(audio.channels[0].buffer, 0x1234);
    audio.step_cck(0);
    audio.step_cck(0);
    assert_eq!(audio.channels[0].current_sample, 0x34);
    audio.step_cck(0);
    audio.step_cck(0);
    assert_eq!(audio.channels[0].current_sample, 0x56);
    assert!(audio.channels[0].dma_request);
    audio.set_dat(0, 0x9abc, false);
    assert_eq!(audio.channels[0].words_remaining, 7);
    assert!(audio.channels[0].restart_request);
    assert!(!audio.poll_channel_irq(0));
    for _ in 0..4 {
        audio.step_cck(0);
    }
    assert_eq!(audio.channels[0].current_sample, 0x9au8 as i8);
    assert!(audio.poll_channel_irq(0));
    // Disabling DMA finishes the current word; it does not erase its low byte.
    audio.set_channel_dma(0, false);
    assert!(!audio.channels[0].dma_request);
    audio.step_cck(1);
    audio.step_cck(1);
    assert_eq!(audio.channels[0].current_sample, 0xbcu8 as i8);
    audio.step_cck(1);
    audio.step_cck(1);
    assert_eq!(audio.channels[0].state, AudioState::Idle);
}

#[test]
fn attachments_use_their_sample_transition_and_mute_every_source_channel() {
    for channel in 0..4 {
        for mode in [1u16, 16, 17] {
            let mut audio = Audio::default();
            audio.set_adkcon(mode << channel);
            audio.set_per(channel, 2);
            audio.set_vol(channel, 64);
            audio.set_dat(channel, 32, false);
            if channel < 3 {
                assert_eq!(
                    audio.channels[channel + 1].vol,
                    if mode & 1 != 0 { 32 } else { 0 }
                );
                assert_eq!(audio.channels[channel + 1].per, 0);
            }
            audio.set_dat(channel, 100, false);
            audio.step_cck(0);
            audio.step_cck(0);
            if channel < 3 {
                assert_eq!(
                    audio.channels[channel + 1].per,
                    if mode & 16 != 0 { 100 } else { 0 }
                );
            }
            audio.set_dat(channel, 7, false);
            audio.step_cck(0);
            audio.step_cck(0);
            if channel < 3 && mode & 1 != 0 {
                assert_eq!(audio.channels[channel + 1].vol, 7);
            }
            for _ in 0..4096 {
                audio.step_cck(0);
            }
            assert!(audio.samples_available() > 0);
            while let Some(sample) = audio.pop_sample() {
                assert_eq!(sample, StereoSample::default());
            }
        }
    }
}

#[test]
fn zero_counters_wrap_and_period_writes_wait_for_the_next_sample_boundary() {
    let mut audio = Audio::default();
    audio.set_per(0, 0);
    audio.set_dat(0, 0x1234, false);
    for _ in 0..65535 {
        audio.step_cck(0);
    }
    assert_eq!(audio.channels[0].current_sample, 0x12);
    audio.set_per(0, 4);
    assert_eq!(audio.channels[0].counter, 1);
    audio.step_cck(0);
    assert_eq!(audio.channels[0].current_sample, 0x34);
    assert_eq!(audio.channels[0].counter, 4);
    audio.reset();
    audio.set_channel_dma(0, true);
    audio.set_dat(0, 0xdead, false);
    assert_eq!(audio.channels[0].words_remaining, 65535);
    assert!(audio.channels[0].restart_request);
    audio.set_channel_dma(0, false);
    assert_eq!(audio.channels[0].state, AudioState::Idle);
    assert!(!audio.channels[0].dma_request);
}
