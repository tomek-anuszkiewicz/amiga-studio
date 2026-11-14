#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, VideoStandard};
use machine_loop::{A500Machine, A500State};
use paula::audio::{Audio, StereoSample};

fn machine(standard: VideoStandard) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    for channel in 0..4 {
        let reg = 0xa0 + channel * 16;
        let mut bus = machine.memory_bus();
        assert!(bus.write_custom_word(reg + 6, 300));
        assert!(bus.write_custom_word(reg + 8, 64));
        assert!(bus.write_custom_word(reg + 10, 0x4080));
    }
    machine
}

#[test]
fn one_frame_drains_fit_the_ring_for_pal_ntsc_and_interlaced_fields() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for interlace in [false, true] {
            let mut machine = machine(standard);
            assert!(machine
                .memory_bus()
                .write_custom_word(0x100, if interlace { 4 } else { 0 }));
            let mut drained = 0;
            let mut nonzero = false;
            for _ in 0..6 {
                machine.step_frame();
                let expected = machine.cck * 44_100 / u64::from(standard.color_clock_hz());
                let pending = expected - drained;
                assert!(pending > 700 && pending < 900);
                assert_eq!(machine.paula.audio.samples_available(), pending as usize);
                let mut buffer = [StereoSample::default(); 1024];
                let count = machine.read_audio_samples(&mut buffer);
                assert_eq!(count, pending as usize);
                assert_eq!(machine.read_audio_samples(&mut buffer[..0]), 0);
                assert_eq!(machine.paula.audio.samples_available(), 0);
                nonzero |= buffer[..count].iter().any(|s| s.left != 0 || s.right != 0);
                drained += count as u64;
            }
            assert!(nonzero);
        }
    }
}

#[test]
fn full_pcm_ring_dense_blep_history_and_fractional_phase_survive_machine_restore() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = machine(standard);
        machine.step_cycles(90_000);
        // More changes than the entire retained BLEP tail, including wraparound.
        for tick in 0..3000 {
            assert!(machine
                .memory_bus()
                .write_custom_word(0x9e, if tick % 2 == 0 { 0x800f } else { 0x000f }));
            machine.step_cck();
        }
        assert_eq!(machine.paula.audio.samples_available(), 1024);
        let json = serde_json::to_string(&machine.save_state()).unwrap();
        let state: A500State = serde_json::from_str(&json).unwrap();
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&state);
        assert_eq!(machine.save_state(), restored.save_state());
        for tick in 0..4000 {
            machine.step_cck();
            restored.step_cck();
            if tick % 137 == 0 {
                let mut left = [StereoSample::default(); 113];
                let mut right = left;
                assert_eq!(
                    machine.read_audio_samples(&mut left),
                    restored.read_audio_samples(&mut right)
                );
                assert_eq!(left, right);
            }
        }
        assert_eq!(machine.save_state(), restored.save_state());
        restored.reset_warm();
        assert_eq!(restored.paula.audio, Audio::new(standard));
        restored.step_cycles(1000);
        assert_eq!(
            restored.paula.audio.samples_available(),
            (1000 * 44_100 / standard.color_clock_hz()) as usize
        );
    }
}

#[test]
fn cia_direction_and_active_low_led_control_select_the_filter() {
    let mut machine = machine(VideoStandard::Pal);
    machine.cia_a.pra &= !2;
    machine.step_cck();
    let state = serde_json::to_value(&machine.paula.audio).unwrap();
    assert_eq!(
        state["output"]["filter_enabled"], false,
        "Input direction leaves PA1 pulled high"
    );
    machine.cia_a.ddra |= 2;
    machine.step_cck();
    let state = serde_json::to_value(&machine.paula.audio).unwrap();
    assert_eq!(state["output"]["filter_enabled"], true);
    machine.cia_a.pra |= 2;
    machine.step_cck();
    let state = serde_json::to_value(&machine.paula.audio).unwrap();
    assert_eq!(state["output"]["filter_enabled"], false);
}

#[test]
fn malformed_audio_phase_history_and_ring_positions_are_rejected() {
    let mut machine = machine(VideoStandard::Pal);
    machine.step_cycles(1000);
    let original = serde_json::to_value(&machine.paula.audio).unwrap();
    for (field, invalid) in [
        ("phase", 3_546_895),
        ("step_head", 2048),
        ("step_count", 2049),
        ("read_pos", 1024),
        ("write_pos", 1024),
        ("sample_count", 1025),
        ("left", i64::from(i32::MAX)),
        ("right", i64::from(i32::MIN)),
    ] {
        let mut value = original.clone();
        value["output"][field] = invalid.into();
        assert!(serde_json::from_value::<Audio>(value).is_err(), "{field}");
    }
    let mut value = original.clone();
    value["output"]["sample_count"] = 0.into();
    assert!(serde_json::from_value::<Audio>(value).is_err());
    for field in ["cck", "left", "right"] {
        let mut value = original.clone();
        let head = value["output"]["step_head"].as_u64().unwrap() as usize;
        value["output"]["steps"][head][field] = 1_000_000.into();
        assert!(serde_json::from_value::<Audio>(value).is_err());
    }
    let mut value = original.clone();
    value["output"]["steps"][0] = "Invalid step".into();
    assert!(serde_json::from_value::<Audio>(value).is_err());
    let steps = serde_json::to_string(&original["output"]["steps"]).unwrap();
    let truncated = format!("{},", &steps[..steps.len() - 1]);
    let json = serde_json::to_string(&original)
        .unwrap()
        .replace(&steps, &truncated);
    assert!(serde_json::from_str::<Audio>(&json).is_err());
    let mut value = original;
    for extra in [false, true] {
        let mut malformed = value.clone();
        let steps = malformed["output"]["steps"].as_array_mut().unwrap();
        if extra {
            steps.push(steps[0].clone());
        } else {
            steps.pop();
        }
        assert!(serde_json::from_value::<Audio>(malformed).is_err());
    }
    value["output"].as_object_mut().unwrap().remove("phase");
    assert!(serde_json::from_value::<Audio>(value).is_err());
}

#[test]
fn blep_history_age_remains_correct_when_the_cck_counter_wraps() {
    let mut audio = Audio::default();
    audio.set_vol(0, 64);
    audio.set_per(0, 200);
    audio.set_dat(0, 0x7f80, false);
    for _ in 0..1000 {
        audio.step_cck(0);
    }
    let mut value = serde_json::to_value(&audio).unwrap();
    let shift = u64::MAX - 1016;
    value["output"]["cck"] = (u64::MAX - 16).into();
    for event in value["output"]["steps"].as_array_mut().unwrap() {
        event["cck"] = event["cck"].as_u64().unwrap().wrapping_add(shift).into();
    }
    let mut wrapped: Audio = serde_json::from_value(value).unwrap();
    for _ in 0..3000 {
        audio.step_cck(0);
        wrapped.step_cck(0);
        assert_eq!(audio.channels, wrapped.channels);
        while let Some(sample) = audio.pop_sample() {
            assert_eq!(wrapped.pop_sample(), Some(sample));
        }
        assert_eq!(wrapped.pop_sample(), None);
    }
}
