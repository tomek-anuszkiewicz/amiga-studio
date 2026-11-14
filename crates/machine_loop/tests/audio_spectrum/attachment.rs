//! DMA-driven volume and period attachment observed in decoded output PCM.

use super::{amplitude, waveform, write_and_read_wav, RATE, TABLE_SIZE};
use config::{A500Config, VideoStandard};
use machine_loop::A500Machine;
use paula::audio::StereoSample;
use std::{fs, path::Path};

const CONTROL_PERIOD: u16 = 4096;
const CARRIER_PERIOD: u16 = 256;
const PLATEAU_WORDS: usize = 64;
const PLATEAU_CCK: u64 = PLATEAU_WORDS as u64 * 2 * CONTROL_PERIOD as u64;
const ANALYSIS_SAMPLES: usize = 4096;
const CARRIER_HARMONIC: usize = 11;

#[derive(Clone, Copy)]
enum Attachment {
    Volume,
    Period,
}

impl Attachment {
    fn name(self) -> &'static str {
        match self {
            Self::Volume => "volume",
            Self::Period => "period",
        }
    }

    fn values(self) -> [u16; 2] {
        match self {
            Self::Volume => [16, 48],
            Self::Period => [CARRIER_PERIOD, 2 * CARRIER_PERIOD],
        }
    }
}

fn capture(standard: VideoStandard, mode: Attachment) -> Vec<StereoSample> {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    // Default CIA-A input pins leave the LED filter off, as in the spectrum fixture.
    let source = waveform(&[(CARRIER_HARMONIC, 48.0)]);
    for (index, &value) in mode.values().iter().enumerate() {
        for word in 0..PLATEAU_WORDS {
            let address = 0x1000 + 2 * (index * PLATEAU_WORDS + word);
            machine.physical_memory.chip_ram[address..address + 2]
                .copy_from_slice(&value.to_be_bytes());
        }
    }
    for (offset, sample) in source.into_iter().enumerate() {
        machine.physical_memory.chip_ram[0x2000 + offset] = sample as u8;
    }
    let initial_volume = match mode {
        Attachment::Volume => 0,
        Attachment::Period => 48,
    };
    for (channel, address, length, period, volume) in [
        (0u16, 0x1000, (2 * PLATEAU_WORDS) as u16, CONTROL_PERIOD, 64),
        (
            1,
            0x2000,
            (TABLE_SIZE / 2) as u16,
            CARRIER_PERIOD,
            initial_volume,
        ),
    ] {
        let register = 0xa0 + channel * 16;
        for (offset, value) in [(0, 0), (2, address), (4, length), (6, period), (8, volume)] {
            assert!(machine
                .memory_bus()
                .write_custom_word(register + offset, value));
        }
    }
    let control = match mode {
        Attachment::Volume => 0x8001,
        Attachment::Period => 0x8010,
    };
    assert!(machine.memory_bus().write_custom_word(0x9e, control));
    assert!(machine.memory_bus().write_custom_word(0x96, 0x8203));
    // Three plateaus exercise both transition directions and the DMA block loop.
    // Once started, only DMA supplies control words and carrier bytes.
    let mut samples = Vec::new();
    while machine.cck < 3 * PLATEAU_CCK {
        machine.step_frame();
        let mut buffer = [StereoSample::default(); 1024];
        let count = machine.read_audio_samples(&mut buffer);
        samples.extend_from_slice(&buffer[..count]);
        assert_eq!(machine.paula.audio.samples_available(), 0);
        assert_eq!(
            samples.len() as u64,
            machine.cck * 44_100 / u64::from(standard.color_clock_hz())
        );
    }
    assert_eq!(machine.paula.interrupts.intreq & 0x0180, 0x0180);
    let name = match standard {
        VideoStandard::Pal => "pal",
        VideoStandard::Ntsc => "ntsc",
    };
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/tmp/audio-spectrum");
    fs::create_dir_all(&output).unwrap();
    let samples = write_and_read_wav(
        &output.join(format!("{name}-{}-attachment.wav", mode.name())),
        &samples,
    );
    assert!(
        samples.iter().all(|sample| sample.right == 0),
        "Attached channel 0 must remain silent, including transitions"
    );
    samples
}

#[derive(Debug)]
struct Measurement {
    rms: f64,
    frequency: f64,
}

fn measure_plateau(
    samples: &[StereoSample],
    standard: VideoStandard,
    plateau: usize,
    period: u16,
) -> Measurement {
    // Start well after DMA startup, parameter latching and the 2048-CCK BLEP tail.
    // Boundaries come from the stimulus clock, never from observed output state.
    let start_cck = plateau as u64 * PLATEAU_CCK + PLATEAU_CCK / 6;
    let first = (start_cck * 44_100 / u64::from(standard.color_clock_hz())) as usize;
    let last = first + ANALYSIS_SAMPLES;
    assert!(
        (last as u64 * u64::from(standard.color_clock_hz())).div_ceil(44_100)
            < (plateau as u64 + 1) * PLATEAU_CCK
    );
    let signal: Vec<_> = samples[first..last]
        .iter()
        .map(|sample| f64::from(sample.left))
        .collect();
    let rms = (signal.iter().map(|value| value * value).sum::<f64>() / signal.len() as f64).sqrt();
    assert!(
        rms > 100.0,
        "The destination must produce a substantial carrier"
    );
    let crossings: Vec<_> = signal
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            (pair[0] <= 0.0 && pair[1] > 0.0).then(|| index as f64 - pair[0] / (pair[1] - pair[0]))
        })
        .collect();
    assert!(crossings.len() >= 20);
    let frequency =
        RATE * (crossings.len() - 1) as f64 / (crossings.last().unwrap() - crossings[0]);
    let expected = f64::from(standard.color_clock_hz()) / f64::from(period)
        * CARRIER_HARMONIC as f64
        / TABLE_SIZE as f64;
    assert!(
        (frequency / expected - 1.0).abs() < 0.002,
        "{standard:?}, plateau {plateau}: frequency {frequency}, expected {expected}"
    );
    // A frequency match alone could accept distorted output. Check tone dominance too.
    let tone_rms = amplitude(&signal, expected, RATE, true) / 2.0f64.sqrt();
    assert!(
        (tone_rms / rms - 1.0).abs() < 0.02,
        "{standard:?}, plateau {plateau}: expected tone must dominate PCM"
    );
    Measurement { rms, frequency }
}

#[test]
fn dma_volume_attachment_changes_pcm_amplitude_and_keeps_source_silent() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let samples = capture(standard, Attachment::Volume);
        let measurements: Vec<_> = (0..3)
            .map(|plateau| measure_plateau(&samples, standard, plateau, CARRIER_PERIOD))
            .collect();
        let rise = measurements[1].rms / measurements[0].rms;
        let restored = measurements[2].rms / measurements[0].rms;
        assert!(
            (rise / 3.0 - 1.0).abs() < 0.01,
            "16 -> 48 must triple amplitude: {measurements:?}"
        );
        assert!(
            (restored - 1.0).abs() < 0.01,
            "48 -> 16 must restore amplitude: {measurements:?}"
        );
        println!("{standard:?} volume attachment: {measurements:?}; gain ratio={rise:.6}, return ratio={restored:.6}");
    }
}

#[test]
fn dma_period_attachment_changes_pcm_frequency_and_keeps_source_silent() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let samples = capture(standard, Attachment::Period);
        let measurements: Vec<_> = [CARRIER_PERIOD, 2 * CARRIER_PERIOD, CARRIER_PERIOD]
            .into_iter()
            .enumerate()
            .map(|(plateau, period)| measure_plateau(&samples, standard, plateau, period))
            .collect();
        let ratio = measurements[0].frequency / measurements[1].frequency;
        let restored = measurements[2].frequency / measurements[0].frequency;
        assert!(
            (ratio / 2.0 - 1.0).abs() < 0.002,
            "P -> 2P must halve frequency: {measurements:?}"
        );
        assert!(
            (restored - 1.0).abs() < 0.002,
            "2P -> P must restore frequency: {measurements:?}"
        );
        println!("{standard:?} period attachment: {measurements:?}; frequency ratio={ratio:.6}, return ratio={restored:.6}");
    }
}
