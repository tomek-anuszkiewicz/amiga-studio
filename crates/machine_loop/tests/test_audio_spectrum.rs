#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Chip RAM -> Agnus DMA -> Paula -> BLEP -> frame drains -> WAV -> spectrum.
//! The oracle uses circuit transfer functions, never the runtime BLEP tables.

use config::{A500Config, VideoStandard};
use machine_loop::A500Machine;
use paula::audio::StereoSample;
use physical_memory::BusResult;
use serde::Serialize;
use std::{f64::consts::TAU, fs, path::Path};

#[path = "audio_spectrum/attachment.rs"]
mod attachment;
#[path = "common/cia.rs"]
mod cia;

const RATE: f64 = 44_100.0;
const PERIOD: u16 = 128;
const TABLE_SIZE: usize = 256;
const SETTLING: usize = 4410;
const WINDOW: usize = 16384;
const RIGHT_TONES: [(usize, f64); 3] = [(5, 36.0), (37, 28.0), (83, 20.0)];
const LEFT_TONES: [(usize, f64); 1] = [(11, 48.0)];

fn waveform(tones: &[(usize, f64)]) -> Vec<i8> {
    (0..TABLE_SIZE)
        .map(|i| {
            tones
                .iter()
                .map(|&(bin, amplitude)| {
                    amplitude * (TAU * bin as f64 * i as f64 / TABLE_SIZE as f64).sin()
                })
                .sum::<f64>()
                .round() as i8
        })
        .collect()
}

fn capture(standard: VideoStandard, filter: bool, right: &[i8], left: &[i8]) -> Vec<StereoSample> {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    assert_eq!(
        cia::write_byte(&mut machine, 0xbfe001, if filter { 0xfd } else { 0xff }),
        BusResult::Ready(())
    );
    assert_eq!(
        cia::write_byte(&mut machine, 0xbfe201, 2),
        BusResult::Ready(())
    );
    for (channel, samples) in [right, left].into_iter().enumerate() {
        let address = 0x1000 + channel * TABLE_SIZE;
        for (offset, &sample) in samples.iter().enumerate() {
            machine.physical_memory.chip_ram[address + offset] = sample as u8;
        }
        let register = 0xa0 + channel as u16 * 16;
        for (offset, value) in [(0, 0), (2, address as u16), (4, 128), (6, PERIOD), (8, 64)] {
            assert!(machine
                .memory_bus()
                .write_custom_word(register + offset, value));
        }
    }
    // No AUDxDAT injection: every sample word must arrive through real DMA.
    assert!(machine.memory_bus().write_custom_word(0x96, 0x8203));
    let mut result = Vec::new();
    while result.len() < SETTLING + WINDOW {
        machine.step_frame();
        let mut buffer = [StereoSample::default(); 1024];
        let count = machine.read_audio_samples(&mut buffer);
        result.extend_from_slice(&buffer[..count]);
        assert_eq!(machine.paula.audio.samples_available(), 0);
        assert_eq!(
            result.len() as u64,
            machine.cck * 44_100 / u64::from(standard.color_clock_hz())
        );
    }
    assert_eq!(machine.paula.interrupts.intreq & 0x0180, 0x0180);
    result
}

fn write_and_read_wav(path: &Path, samples: &[StereoSample]) -> Vec<StereoSample> {
    let size = (samples.len() * 4) as u32;
    let mut bytes = Vec::with_capacity(44 + size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + size).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    for value in [1u16, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [44_100u32, 44_100 * 4] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [4u16, 16] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.left.to_le_bytes());
        bytes.extend_from_slice(&sample.right.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(&bytes[20..24], &[1, 0, 2, 0]);
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        44_100
    );
    assert_eq!(&bytes[32..40], &[4, 0, 16, 0, b'd', b'a', b't', b'a']);
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
        bytes.len() - 8
    );
    assert_eq!(
        u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize,
        bytes.len() - 44
    );
    let decoded: Vec<_> = bytes[44..]
        .chunks_exact(4)
        .map(|pair| StereoSample {
            left: i16::from_le_bytes([pair[0], pair[1]]),
            right: i16::from_le_bytes([pair[2], pair[3]]),
        })
        .collect();
    assert_eq!(decoded, samples);
    decoded
}

/// Fourier projection at the exact frequency; periodic Hann coherent-gain correction.
fn projection(samples: &[f64], frequency: f64, rate: f64, hann: bool) -> (f64, f64) {
    let (mut cosine, mut sine, mut weight_sum) = (0.0, 0.0, 0.0);
    for (n, &sample) in samples.iter().enumerate() {
        let weight = if hann {
            0.5 - 0.5 * (TAU * n as f64 / samples.len() as f64).cos()
        } else {
            1.0
        };
        let phase = TAU * frequency * n as f64 / rate;
        cosine += weight * sample * phase.cos();
        sine += weight * sample * phase.sin();
        weight_sum += weight;
    }
    (2.0 * cosine / weight_sum, 2.0 * sine / weight_sum)
}

fn amplitude(samples: &[f64], frequency: f64, rate: f64, hann: bool) -> f64 {
    let (cosine, sine) = projection(samples, frequency, rate, hann);
    cosine.hypot(sine)
}

fn db(ratio: f64) -> f64 {
    20.0 * ratio.max(1e-12).log10()
}

/// Direct frequency-domain oracle for the specified 21 kHz Kaiser-windowed sinc.
/// Minimum-phase conversion preserves its magnitude. No BLEP data is consulted.
fn anti_alias_gain(frequency: f64, clock: f64) -> f64 {
    let i0 = |x: f64| {
        let (mut sum, mut term) = (1.0, 1.0);
        for k in 1..32 {
            term *= (x / (2.0 * f64::from(k))).powi(2);
            sum += term;
        }
        sum
    };
    let mut dc = 0.0;
    let mut response = 0.0;
    for n in 0..2048 {
        let offset = f64::from(n) - 2047.0 / 2.0;
        let x = TAU * 21_000.0 * offset / clock;
        let position = 2.0 * f64::from(n) / 2047.0 - 1.0;
        let weight = i0(8.0 * (1.0 - position * position).sqrt());
        let impulse = x.sin() / x * weight;
        dc += impulse;
        response += impulse * (TAU * frequency * offset / clock).cos();
    }
    (response / dc).abs()
}

/// DAC zero-order hold, fixed RC, optional Sallen-Key circuit magnitude.
fn transfer(frequency: f64, dac_rate: f64, filter: bool) -> f64 {
    let x = std::f64::consts::PI * frequency / dac_rate;
    let hold = x.sin() / x;
    let rc = 1.0 / (1.0 + (frequency / 4900.0).powi(2)).sqrt();
    let led = if filter {
        let ratio = frequency / 3275.0;
        1.0 / ((1.0 - ratio * ratio).powi(2) + (ratio / 0.65).powi(2)).sqrt()
    } else {
        1.0
    };
    hold * rc * led * anti_alias_gain(frequency, dac_rate * f64::from(PERIOD))
}

#[derive(Debug, Serialize)]
struct Tone {
    frequency_hz: f64,
    expected_amplitude: f64,
    measured_amplitude: f64,
    error_db: f64,
    peak_offset_hz: f64,
}

#[derive(Debug, Serialize)]
struct Spectrum {
    tones: Vec<Tone>,
    images: Vec<Tone>,
    residual_db: f64,
    alias_frequency_hz: f64,
    alias_db: f64,
    foreign_tone_db: f64,
}

impl Spectrum {
    fn passes(&self) -> bool {
        self.tones.iter().chain(&self.images).all(|tone| {
            tone.error_db.abs() < 0.5 && tone.peak_offset_hz.abs() <= RATE / WINDOW as f64 / 4.0
        }) && self.residual_db < -35.0
            && self.alias_db < -45.0
            && self.foreign_tone_db < -42.0
    }
}

fn analyze(
    samples: &[f64],
    source: &[i8],
    tones: &[(usize, f64)],
    foreign_bin: usize,
    dac_rate: f64,
    filter: bool,
) -> Spectrum {
    let source: Vec<_> = source.iter().map(|&v| f64::from(v)).collect();
    let fundamental = dac_rate / TABLE_SIZE as f64;
    let mut remainder = samples.to_vec();
    let mut measurements = Vec::new();
    let mut images = Vec::new();
    let mut components = Vec::new();
    for &(bin, _) in tones {
        components.push((bin, fundamental * bin as f64, false));
        let image = dac_rate - fundamental * bin as f64;
        if image < RATE / 2.0 {
            components.push((bin, image, true));
        }
    }
    for (bin, frequency, is_image) in components {
        let expected = amplitude(&source, bin as f64, TABLE_SIZE as f64, false)
            * 128.0
            * transfer(frequency, dac_rate, filter);
        let (cosine, sine) = projection(samples, frequency, RATE, true);
        let measured = cosine.hypot(sine);
        for (n, value) in remainder.iter_mut().enumerate() {
            let phase = TAU * frequency * n as f64 / RATE;
            *value -= cosine * phase.cos() + sine * phase.sin();
        }
        let quarter_bin = RATE / samples.len() as f64 / 4.0;
        let mut peak = (measured, 0.0);
        for offset in [-4, -3, -2, -1, 1, 2, 3, 4] {
            let delta = f64::from(offset) * quarter_bin;
            let level = amplitude(samples, frequency + delta, RATE, true);
            if level > peak.0 {
                peak = (level, delta);
            }
        }
        let tone = Tone {
            frequency_hz: frequency,
            expected_amplitude: expected,
            measured_amplitude: measured,
            error_db: db(measured / expected),
            peak_offset_hz: peak.1,
        };
        if is_image {
            images.push(tone);
        } else {
            measurements.push(tone);
        }
    }
    let power = |values: &[f64]| values.iter().map(|value| value * value).sum::<f64>();
    let last = measurements.last().unwrap();
    let image = dac_rate + last.frequency_hz;
    let alias = (image - (image / RATE).round() * RATE).abs();
    Spectrum {
        residual_db: 10.0 * (power(&remainder) / power(samples)).log10(),
        alias_frequency_hz: alias,
        alias_db: db(amplitude(samples, alias, RATE, true) / last.measured_amplitude),
        foreign_tone_db: db(
            amplitude(samples, fundamental * foreign_bin as f64, RATE, true)
                / measurements[0].measured_amplitude,
        ),
        tones: measurements,
        images,
    }
}

#[test]
fn dma_waveforms_reconstruct_expected_spectra_in_exported_wav() {
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/tmp/audio-spectrum");
    fs::create_dir_all(&output).unwrap();
    let right_source = waveform(&RIGHT_TONES);
    let left_source = waveform(&LEFT_TONES);
    let mut report = Vec::new();
    for (name, standard) in [("pal", VideoStandard::Pal), ("ntsc", VideoStandard::Ntsc)] {
        for filter in [false, true] {
            let case = format!("{name}-filter-{}", if filter { "on" } else { "off" });
            let samples = capture(standard, filter, &right_source, &left_source);
            let decoded = write_and_read_wav(&output.join(format!("{case}.wav")), &samples);
            let window = &decoded[SETTLING..SETTLING + WINDOW];
            let right: Vec<_> = window.iter().map(|s| f64::from(s.right)).collect();
            let left: Vec<_> = window.iter().map(|s| f64::from(s.left)).collect();
            let dac_rate = f64::from(standard.color_clock_hz()) / f64::from(PERIOD);
            let right_spectrum = analyze(&right, &right_source, &RIGHT_TONES, 11, dac_rate, filter);
            let left_spectrum = analyze(&left, &left_source, &LEFT_TONES, 5, dac_rate, filter);
            println!("{case}: right={right_spectrum:#?}; left={left_spectrum:#?}");
            report.push(serde_json::json!({ "case": case, "sample_count": decoded.len(), "right": right_spectrum, "left": left_spectrum }));
            fs::write(
                output.join("report.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            assert!(right_spectrum.passes(), "{case}: {right_spectrum:#?}");
            assert!(left_spectrum.passes(), "{case}: {left_spectrum:#?}");
            if standard == VideoStandard::Pal && !filter {
                let reduced: Vec<_> = right.iter().map(|s| s * 0.5).collect();
                assert!(
                    !analyze(&reduced, &right_source, &RIGHT_TONES, 11, dac_rate, filter).passes()
                );
                assert!(
                    !analyze(&left, &right_source, &RIGHT_TONES, 11, dac_rate, filter).passes()
                );
                let mut missing = right.clone();
                missing.drain(4000..4882);
                missing.resize(WINDOW, 0.0);
                assert!(
                    !analyze(&missing, &right_source, &RIGHT_TONES, 11, dac_rate, filter).passes()
                );
                let alias = right_spectrum.alias_frequency_hz;
                let level = right_spectrum.tones.last().unwrap().measured_amplitude * 0.02;
                let contaminated: Vec<_> = right
                    .iter()
                    .enumerate()
                    .map(|(n, &s)| s + level * (TAU * alias * n as f64 / RATE).sin())
                    .collect();
                assert!(!analyze(
                    &contaminated,
                    &right_source,
                    &RIGHT_TONES,
                    11,
                    dac_rate,
                    filter
                )
                .passes());
            }
        }
    }
}
