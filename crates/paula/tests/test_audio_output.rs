use config::VideoStandard;
use paula::audio::{Audio, StereoSample, AUDIO_RING_BUFFER_CAPACITY, AUDIO_SAMPLE_RATE};

#[path = "../src/audio_tables.rs"]
mod tables;

fn table(standard: VideoStandard, filter: bool) -> &'static [i32; 2048] {
    match (standard, filter) {
        (VideoStandard::Pal, false) => &tables::BLEP_A500_PAL_FILTER_OFF,
        (VideoStandard::Pal, true) => &tables::BLEP_A500_PAL_FILTER_ON,
        (VideoStandard::Ntsc, false) => &tables::BLEP_A500_NTSC_FILTER_OFF,
        (VideoStandard::Ntsc, true) => &tables::BLEP_A500_NTSC_FILTER_ON,
    }
}

fn start(audio: &mut Audio, channel: usize, sample: i8, volume: u8, period: u16) {
    audio.set_per(channel, period);
    audio.set_vol(channel, volume);
    audio.set_dat(channel, u16::from_be_bytes([sample as u8; 2]), false);
}

/// Independent superposition of full step responses, including old DC steps.
fn reference(events: &[(u64, i32)], sample: u64, clock: u32, table: &[i32]) -> i16 {
    let rate = 44_100i64;
    let scale = 131_072 * rate;
    let time = sample as i64 * i64::from(clock);
    let mut sum = 0i64;
    for &(cck, delta) in events {
        let age = time - cck as i64 * rate;
        if age < 0 {
            continue;
        }
        let index = (age / rate) as usize;
        let correction = if index >= table.len() {
            0
        } else {
            let weight = age % rate;
            i64::from(table[index]) * (rate - weight)
                + i64::from(table.get(index + 1).copied().unwrap_or(0)) * weight
        };
        sum += i64::from(delta) * (scale - correction);
    }
    (sum * 2 / scale).clamp(-32768, 32767) as i16
}

#[test]
fn all_channels_reconstruct_signed_steps_at_fractional_pcm_instants() {
    assert_eq!(tables::BLEP_TABLE_LENGTH, 2048);
    assert_eq!(AUDIO_SAMPLE_RATE, 44_100);
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for filter in [false, true] {
            for channel in 0..4 {
                for amplitude in [-128i8, -3, 1, 127] {
                    let mut audio = Audio::new(standard);
                    audio.set_filter_enabled(filter);
                    start(&mut audio, channel, amplitude, 63, 0);
                    let mut produced = 0;
                    for cck in 1..=3000u64 {
                        audio.step_cck(0);
                        if let Some(sample) = audio.pop_sample() {
                            produced += 1;
                            let expected = reference(
                                &[(0, i32::from(amplitude) * 63)],
                                produced,
                                standard.color_clock_hz(),
                                table(standard, filter),
                            );
                            let pair = if channel == 1 || channel == 2 {
                                StereoSample {
                                    left: expected,
                                    right: 0,
                                }
                            } else {
                                StereoSample {
                                    left: 0,
                                    right: expected,
                                }
                            };
                            assert_eq!(
                                sample, pair,
                                "{standard:?} filter={filter} channel={channel} CCK={cck}"
                            );
                        }
                        assert_eq!(
                            produced,
                            cck * 44_100 / u64::from(standard.color_clock_hz())
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn full_scale_stereo_mixing_clips_filter_overshoot_after_reconstruction() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for filter in [false, true] {
            for amplitude in [-128i8, 127] {
                let mut audio = Audio::new(standard);
                audio.set_filter_enabled(filter);
                for channel in 0..4 {
                    start(&mut audio, channel, amplitude, 64, 0);
                }
                let mut ordinal = 0;
                let mut reached_limit = false;
                let mut last = StereoSample::default();
                for _ in 0..3000 {
                    audio.step_cck(0);
                    if let Some(sample) = audio.pop_sample() {
                        ordinal += 1;
                        let expected = reference(
                            &[(0, i32::from(amplitude) * 64 * 2)],
                            ordinal,
                            standard.color_clock_hz(),
                            table(standard, filter),
                        );
                        assert_eq!(
                            sample,
                            StereoSample {
                                left: expected,
                                right: expected
                            }
                        );
                        reached_limit |= expected == i16::MIN || expected == i16::MAX;
                        last = sample;
                    }
                }
                assert_eq!(last.left, i16::from(amplitude) * 256);
                if amplitude < 0 || filter {
                    assert!(
                        reached_limit,
                        "{standard:?} filter={filter} amplitude={amplitude}"
                    );
                }
            }
        }
    }
}

#[test]
fn volume_latches_and_attachment_changes_leave_their_complete_filter_tail() {
    let mut audio = Audio::default();
    start(&mut audio, 0, 64, 64, 50);
    let events = [(0, 4096), (100, -2048), (125, -2048), (200, 2048)];
    let mut produced = 0;
    for cck in 0..5000 {
        match cck {
            20 => audio.set_vol(0, 32),
            125 => audio.set_adkcon(1),
            200 => audio.set_adkcon(0),
            _ => {}
        }
        audio.step_cck(0);
        if let Some(sample) = audio.pop_sample() {
            produced += 1;
            assert_eq!(sample.left, 0);
            assert_eq!(
                sample.right,
                reference(
                    &events,
                    produced,
                    3_546_895,
                    table(VideoStandard::Pal, false)
                )
            );
        }
    }
    assert_eq!(audio.channels[0].output_volume, 32);
}

#[test]
fn dense_steps_coalesce_at_one_cck_and_expire_without_losing_recent_history() {
    let mut audio = Audio::default();
    start(&mut audio, 0, 32, 64, 0);
    let mut events = vec![(0, 2048)];
    let mut produced = 0;
    for cck in 0..6500 {
        if cck != 0 && cck < 4400 {
            let mute = cck % 2 != 0;
            audio.set_adkcon(u16::from(mute));
            events.push((cck, if mute { -2048 } else { 2048 }));
            // Cancel two extra transitions at exactly the same time.
            audio.set_adkcon(u16::from(!mute));
            audio.set_adkcon(u16::from(mute));
        }
        audio.step_cck(0);
        if let Some(sample) = audio.pop_sample() {
            produced += 1;
            assert_eq!(
                sample.right,
                reference(
                    &events,
                    produced,
                    3_546_895,
                    table(VideoStandard::Pal, false)
                )
            );
            assert_eq!(sample.left, 0);
        }
    }
    assert_eq!(audio.pop_sample(), None);
}

#[test]
fn a_step_exactly_one_table_old_has_no_correction_at_an_integer_pcm_instant() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let clock = standard.color_clock_hz();
        // Both nominal CCK rates share a factor of five with 44,100.
        let coincident_cck = clock / 5;
        let mut audio = Audio::new(standard);
        let mut ordinal = 0;
        for _ in 0..coincident_cck - 2048 {
            audio.step_cck(0);
            if let Some(sample) = audio.pop_sample() {
                ordinal += 1;
                assert_eq!(sample, StereoSample::default());
            }
        }
        start(&mut audio, 0, 40, 32, 0);
        let event = [(u64::from(coincident_cck - 2048), 1280)];
        let mut last = StereoSample::default();
        for _ in 0..2048 {
            audio.step_cck(0);
            if let Some(sample) = audio.pop_sample() {
                ordinal += 1;
                assert_eq!(sample.left, 0);
                assert_eq!(
                    sample.right,
                    reference(&event, ordinal, clock, table(standard, false))
                );
                last = sample;
            }
        }
        assert_eq!(ordinal, 44_100 / 5);
        assert_eq!(last.right, 2560);
    }
}

#[test]
fn rolling_pcm_retains_latest_pairs_and_partial_drains_preserve_order() {
    let mut audio = Audio::default();
    start(&mut audio, 1, 100, 64, 0);
    let mut events = vec![(0, 6400)];
    let mut total = 0u64;
    for cck in 0..90_000u64 {
        if cck != 0 && cck % 97 == 0 {
            let mute = (cck / 97) % 2 != 0;
            audio.set_adkcon(if mute { 2 } else { 0 });
            events.push((cck, if mute { -6400 } else { 6400 }));
        }
        audio.step_cck(0);
        total = (cck + 1) * 44_100 / 3_546_895;
    }
    assert!(total > 1024);
    assert_eq!(audio.samples_available(), 1024);
    assert_eq!(audio.read_samples(&mut []), 0);
    let mut buffer = [StereoSample::default(); 173];
    let mut ordinal = total - 1024;
    loop {
        let count = audio.read_samples(&mut buffer);
        for sample in &buffer[..count] {
            ordinal += 1;
            assert_eq!(
                sample.left,
                reference(
                    &events,
                    ordinal,
                    3_546_895,
                    table(VideoStandard::Pal, false)
                )
            );
            assert_eq!(sample.right, 0);
        }
        if count < buffer.len() {
            break;
        }
    }
    assert_eq!(ordinal, total);
    assert_eq!(audio.samples_available(), 0);
    assert_eq!(audio.pop_sample(), None);
    assert_eq!(AUDIO_RING_BUFFER_CAPACITY, 1024);
    audio.reset();
    assert_eq!(audio, Audio::default());
}

#[test]
fn exact_one_second_rate_reset_and_filter_selection_work_for_both_standards() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut audio = Audio::new(standard);
        let mut count = 0;
        for _ in 0..standard.color_clock_hz() {
            audio.step_cck(0);
            if let Some(sample) = audio.pop_sample() {
                count += 1;
                assert_eq!(sample, StereoSample::default());
            }
        }
        assert_eq!(count, 44_100);
        audio.reset();
        assert_eq!(audio, Audio::new(standard));
        start(&mut audio, 0, 127, 64, 0);
        let mut ordinal = 0;
        for cck in 0..3000 {
            let enabled = (cck / 100) % 2 != 0;
            audio.set_filter_enabled(enabled);
            audio.step_cck(0);
            if let Some(sample) = audio.pop_sample() {
                ordinal += 1;
                assert_eq!(
                    sample.right,
                    reference(
                        &[(0, 8128)],
                        ordinal,
                        standard.color_clock_hz(),
                        table(standard, enabled)
                    )
                );
            }
        }
    }
}
