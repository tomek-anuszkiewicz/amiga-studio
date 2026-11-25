#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use base64::{engine::general_purpose::STANDARD, Engine};
use config::{A500Config, A500Preset, VideoStandard};
use machine_loop::{A500Machine, A500State};
use save_state::{MediaSources, SaveAssociation, SaveState, SaveStateError, SourceImage};
use serde_json::{json, Value};

fn capture_synthetic(machine: &A500Machine) -> SaveState {
    SaveState::capture(machine, &MediaSources::default()).unwrap()
}

fn restore_synthetic(saved: SaveState) -> Box<A500State> {
    saved.restore(&MediaSources::default()).unwrap()
}

fn roundtrip_synthetic(machine: &A500Machine) -> Box<A500State> {
    let json = capture_synthetic(machine).to_json().unwrap();
    restore_synthetic(SaveState::from_json(&json).unwrap())
}

const ROM_BYTES: usize = 262_144;
const ADF_BYTES: usize = 901_120;

fn patterned_bytes(size: usize, seed: u8) -> Vec<u8> {
    (0..size)
        .map(|index| (index as u8).wrapping_mul(17).wrapping_add(seed))
        .collect()
}

fn image(name: &str, bytes: Vec<u8>) -> SourceImage {
    SourceImage {
        name: Some(name.into()),
        bytes,
    }
}

fn canonical_bytes(value: &Value, expected: &[u8], field: &str) {
    let encoded = value
        .as_str()
        .unwrap_or_else(|| panic!("{field} must be a string"));
    let actual = STANDARD.decode(encoded).unwrap();
    assert_eq!(actual.len(), expected.len(), "{field}");
    assert!(actual == expected, "{field} contents differ");
    assert!(
        STANDARD.encode(actual) == encoded,
        "{field} is not canonical Base64"
    );
}

#[test]
fn snapshot_after_video_handoff_keeps_the_current_partial_image_and_continuation() {
    use machine_loop::denise::frame_builder::FrameBuilder;

    let mut machine = A500Machine::new(A500Config::default());
    let frame = &mut machine.denise.frame_builder;
    frame.set_field_mode(true, false);
    frame.write_pixel_pair(0, 0, [0xffff_0000; 2]);
    frame.write_pixel_pair(0, 2, [0xff88_0000; 2]);
    frame.finish_field(2);
    let mut published = FrameBuilder::new_buffer();
    assert!(frame.exchange_buffer(&mut published));
    frame.set_field_mode(true, true);
    frame.write_pixel_pair(0, 1, [0xff00_00ff; 2]);

    let saved = capture_synthetic(&machine);
    let preview = image::load_from_memory(&saved.header.screenshot_png)
        .unwrap()
        .to_rgba8();
    assert_eq!(preview.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(preview.get_pixel(0, 1).0, [0, 0, 255, 255]);
    assert_eq!(published[0], 0xffff_0000);
    assert_eq!(published[720], 0xff00_0000);

    let restored = SaveState::from_json(&saved.to_json().unwrap()).unwrap();
    let mut resumed = A500Machine::new(A500Config::default());
    resumed.load_state(&restore_synthetic(restored));
    assert_eq!(machine.denise, resumed.denise);
    for target in [&mut machine, &mut resumed] {
        target
            .denise
            .frame_builder
            .write_pixel_pair(0, 3, [0xff00_0088; 2]);
        target.denise.frame_builder.finish_field(2);
        target.step_cycles(300);
    }
    assert_eq!(machine.cck, resumed.cck);
    assert_eq!(machine.denise, resumed.denise);
    assert_eq!(published[720], 0xff00_0000);
}

#[test]
fn frame_coordinates_resume_at_visible_and_blanking_boundaries() {
    use machine_loop::denise::{Denise, DeniseModel};

    for ccks in [227, 228] {
        for interlaced in [false, true] {
            let mut denise = Denise::new(DeniseModel::Ocs8362);
            denise.bplcon0 = (if interlaced { 4 } else { 0 }).into();
            denise.sync.field_odd = interlaced;
            denise.color[0] = 0xf00;
            denise.strobe_strhor();
            denise.sync.vpos = 28;
            for _ in 0..ccks {
                denise.step_cck();
            }
            assert_eq!(
                denise.frame_builder.get_pixel(0, usize::from(interlaced)),
                0xffff0000
            );
            for hpos in [0, 1, 2, 17, 18, 35, 36, 48, 49, 50, 100] {
                let mut original = denise.clone();
                original.strobe_strhor();
                for _ in 0..hpos {
                    original.step_cck();
                }
                let json = serde_json::to_vec(&original).unwrap();
                let mut restored: Denise = serde_json::from_slice(&json).unwrap();
                assert_eq!(original, restored);
                for step in hpos..ccks + 37 {
                    for target in [&mut original, &mut restored] {
                        if step == ccks {
                            target.strobe_strhor();
                        }
                        assert!(target.set_color(0, step));
                        target.step_cck();
                    }
                    assert_eq!(original.frame_cursor, restored.frame_cursor);
                }
                assert_eq!(original, restored);
            }
        }
    }
}

#[test]
fn denise_display_controls_roundtrip_with_a_pending_mode_change() {
    use machine_loop::denise::{Denise, DeniseModel};
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.sync.hpos = 40;
    for value in [0x6800, 0x2400, 0xe800, 0x7000] {
        assert!(denise.set_bplcon0(value));
        denise.step_cck();
        assert!(denise.set_bplcon0(0x1000));
        let json = serde_json::to_value(&denise).unwrap();
        assert_eq!(json["bplcon0"], value);
        let mut restored: Denise = serde_json::from_value(json).unwrap();
        assert_eq!(restored, denise);
        denise.step_cck();
        restored.step_cck();
        assert_eq!(restored, denise);
        assert_eq!(restored.bplcon0.value(), 0x1000);
        assert!(!restored.is_hires());
        restored.color[0] = 0x0123;
        restored.color[1] = 0x0abc;
        for planes in 0..64 {
            let expected = if planes & 1 == 0 { 0x0123 } else { 0x0abc };
            assert_eq!(restored.decode_pixel(planes), expected);
        }
    }
}

#[test]
fn file_restore_validates_external_media_before_producing_a_machine() {
    let mut machine = A500Machine::new(A500Config::default());
    let rom = image("fixture.rom", patterned_bytes(ROM_BYTES, 25));
    let disk = image("fixture.adf", patterned_bytes(ADF_BYTES, 49));
    machine
        .physical_memory
        .write_bytes_debug(config::address_map::KICKSTART_BASE, &rom.bytes)
        .unwrap();
    assert!(machine.floppy.drives[0].insert_disk(&disk.bytes));
    let mut sources = MediaSources {
        rom: Some(rom),
        disks: [Some(disk), None, None, None],
    };
    let before = machine.save_state();
    let bytes = SaveState::capture(&machine, &sources)
        .unwrap()
        .to_json()
        .unwrap();
    let saved = SaveState::from_bytes(bytes.as_bytes()).unwrap();
    let correct_rom = sources.rom.take();
    let error = saved.clone().restore(&sources).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert!(error.to_string().contains("Kickstart ROM"));

    for size in [0, 262_143, 524_288] {
        sources.rom = Some(image("fixture.rom", vec![0; size]));
        let error = saved.clone().restore(&sources).unwrap_err();
        assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
        assert!(error
            .to_string()
            .contains("Kickstart ROM must be exactly 256 KiB (262144 bytes)"));
        assert!(error
            .to_string()
            .contains(&format!("received {size} bytes")));
    }
    sources.rom = Some(image("fixture.rom", vec![0; ROM_BYTES]));
    let error = saved.clone().restore(&sources).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert!(error
        .to_string()
        .contains("Kickstart ROM (fixture.rom) content does not match"));
    sources.rom = correct_rom;

    let correct_disk = sources.disks[0].take();
    let error = SaveState::capture(&machine, &sources).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert_eq!(error.to_string(), "ADF for DF0 source is unavailable");

    let error = saved.clone().restore(&sources).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert!(error
        .to_string()
        .contains("ADF for DF0 (fixture.adf) source is unavailable"));
    sources.disks[0] = Some(image("fixture.adf", vec![0; ADF_BYTES - 1]));
    let error = SaveState::capture(&machine, &sources).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert_eq!(
        error.to_string(),
        "ADF for DF0 must contain 901120 bytes; received 901119 bytes"
    );

    let error = saved.clone().restore(&sources).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert!(error
        .to_string()
        .contains("ADF for DF0 (fixture.adf) must contain 901120 bytes; received 901119 bytes"));
    sources.disks[0] = Some(image("fixture.adf", vec![0; ADF_BYTES]));
    let error = saved.clone().restore(&sources).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert!(error
        .to_string()
        .contains("ADF for DF0 (fixture.adf) content does not match"));
    sources.disks[0] = correct_disk;

    assert!(*saved.restore(&sources).unwrap() == before);
    assert!(machine.save_state() == before);
}

#[test]
fn test_save_state_metadata_and_header() {
    let machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Standard1Mb,
        VideoStandard::Pal,
    ));
    let saved = capture_synthetic(&machine);
    let state = restore_synthetic(saved.clone());

    assert_eq!(saved.header.version, 17);
    assert!(!saved.header.screenshot_png.is_empty());
    // PNG magic header: \x89PNG\r\n\x1a\n
    assert_eq!(&saved.header.screenshot_png[0..4], b"\x89PNG");
    assert_eq!(state.config.chip_ram(), config::ChipRamSize::Kb512);
    assert_eq!(state.config.slow_ram(), config::SlowRamSize::Kb512);
    assert_eq!(state.config.fast_ram(), config::FastRamSize::None);
    assert_eq!(state.cck, 0);
}

#[test]
fn screenshot_preserves_the_complete_fixed_rectangle_for_each_standard() {
    for (standard, height) in [(VideoStandard::Pal, 568), (VideoStandard::Ntsc, 480)] {
        let mut machine = A500Machine::new(A500Config::standard_1mb(standard));
        let frame = &mut machine.denise.frame_builder;
        assert_eq!([frame.width, frame.height], [720, height]);
        assert_eq!(frame.row_stride(), 720);
        for (y, row) in frame.frame_buffer_mut().chunks_exact_mut(720).enumerate() {
            row.fill(if y % 2 == 0 { 0xffff0000 } else { 0xff0000ff });
        }
        let saved = capture_synthetic(&machine);
        let preview = image::load_from_memory(&saved.header.screenshot_png)
            .unwrap()
            .to_rgba8();
        assert_eq!(preview.dimensions(), (720, height));
        for y in 0..height {
            for x in 0..720 {
                assert_eq!(
                    preview.get_pixel(x, y).0,
                    if y % 2 == 0 {
                        [255, 0, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    }
                );
            }
        }
    }
}

#[test]
fn test_save_state_json_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(200);

    let state = capture_synthetic(&machine);
    let json_str = state.to_json().expect("JSON serialization failed");
    let restored = SaveState::from_json(&json_str).expect("JSON deserialization failed");

    assert_eq!(state.header, restored.header);
    assert_eq!(state, restored);
    assert!(*restore_synthetic(restored) == machine.save_state());
}
#[test]
fn test_save_state_bytes_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(500);

    let state = capture_synthetic(&machine);
    let json_bytes = serde_json::to_vec(&state).expect("Serialization failed");

    let restored = SaveState::from_bytes(&json_bytes).expect("Deserialization failed");
    assert_eq!(state, restored);
}
#[test]
fn test_save_state_file_persistence() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut machine = A500Machine::new(config.clone());
    machine.cpu.state.set_stopped(true);
    machine.step_cycles(150);
    machine.cpu.state.set_d_long(3, 0x1234_abcd);

    let rom = image("original.rom", patterned_bytes(ROM_BYTES, 25));
    let first_disk = image("first.adf", patterned_bytes(ADF_BYTES, 49));
    let second_disk = image("second.adf", patterned_bytes(ADF_BYTES, 157));
    let memory = &mut machine.physical_memory;
    memory
        .write_bytes_debug(config::address_map::KICKSTART_BASE, &rom.bytes)
        .unwrap();
    memory.map_chip_ram_to_low_memory();
    let chip = patterned_bytes(512 * 1024, 0xfc);
    let slow = patterned_bytes(512 * 1024, 0xad);
    memory.write_bytes_debug(0, &chip).unwrap();
    memory.write_bytes_debug(0xc0_0000, &slow).unwrap();
    let frame = &mut machine.denise.frame_builder;
    let full_frame: Vec<_> = (0..720 * 568)
        .map(|index| [0xa1b2_c3d4u32, 0x12e3_709f, 0xff20_4080][index % 3])
        .collect();
    frame.frame_buffer_mut().copy_from_slice(&full_frame);
    let frame_bytes: Vec<_> = full_frame
        .iter()
        .flat_map(|pixel| pixel.to_le_bytes())
        .collect();

    machine
        .floppy
        .set_connections([true, true, true, false])
        .unwrap();
    assert!(machine.floppy.drives[0].insert_disk(&first_disk.bytes));
    assert!(machine.floppy.drives[0].insert_disk(&second_disk.bytes));
    assert!(machine.floppy.drives[1].insert_disk(&first_disk.bytes));
    let mut sources = MediaSources {
        rom: Some(rom),
        disks: [Some(second_disk), Some(first_disk), None, None],
    };
    let mut saved = SaveState::capture(&machine, &sources).unwrap();
    saved.header.timestamp = 1_700_001_234;
    let association = SaveAssociation {
        game_id: Some("fixture-game".into()),
        mounted: [Some("second".into()), None, None, None],
    };
    saved.header.association = association.clone();
    let preview = image::load_from_memory(&saved.header.screenshot_png)
        .unwrap()
        .to_rgba8();
    assert_eq!(preview.dimensions(), (720, 568));
    assert_eq!(preview.get_pixel(0, 0).0, [0xb2, 0xc3, 0xd4, 0xa1]);
    assert_eq!(preview.get_pixel(1, 0).0, [0xe3, 0x70, 0x9f, 0x12]);

    let temp_dir = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let temp_path = temp_dir.join(format!(
        "amiga-save-state-{}-{}.json",
        std::process::id(),
        stamp.as_nanos()
    ));
    let pretty = saved.to_json_pretty().unwrap();
    std::fs::write(&temp_path, &pretty).expect("Failed to write save state file");
    let file = std::fs::read(&temp_path).expect("Failed to read save state file");
    std::fs::remove_file(&temp_path).unwrap();
    assert_eq!(file, pretty.as_bytes());

    // The original labels are no longer available when restoring this file.
    sources.rom.as_mut().unwrap().name = Some("moved.rom".into());
    sources.disks[0].as_mut().unwrap().name = Some("renamed-second.adf".into());
    sources.disks[1].as_mut().unwrap().name = Some("renamed-first.adf".into());
    let original = machine.save_state();
    for bytes in [
        saved.to_json().unwrap().into_bytes(),
        file,
        serde_json::to_vec(&saved).unwrap(),
    ] {
        let text = std::str::from_utf8(&bytes).unwrap();
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(document["header"]["version"], 17);
        assert_eq!(document["header"]["timestamp"], 1_700_001_234);
        assert_eq!(
            document["header"]["association"],
            json!({
                "game_id": "fixture-game",
                "mounted": ["second", null, null, null],
            })
        );
        canonical_bytes(
            &document["header"]["screenshot_png"],
            &saved.header.screenshot_png,
            "PNG",
        );
        let state = &document["state"];
        assert_eq!(state["cck"], 150);
        assert_eq!(state["config"], serde_json::to_value(&config).unwrap());
        let memory = &state["physical_memory"];
        canonical_bytes(&memory["chip_ram"], &chip, "Chip RAM");
        canonical_bytes(&memory["slow_ram"], &slow, "Slow RAM");
        assert_eq!(memory.get("fast_ram"), Some(&Value::Null));
        assert_eq!(
            memory["kickstart_rom"],
            json!({
                "sha256": "1435ae74255cda6413f2ddd79da49c45976b947ea9794c66ebcb2ba1da62fae1",
                "name": "original.rom",
                "synthetic": false,
            })
        );
        let frame = &state["denise"]["frame_builder"]["buffer"];
        canonical_bytes(frame, &frame_bytes, "frame buffer");
        assert_eq!(
            &STANDARD.decode(&frame.as_str().unwrap()[..8]).unwrap()[..4],
            &[0xd4, 0xc3, 0xb2, 0xa1]
        );

        let drives = state["floppy"]["drives"].as_array().unwrap();
        assert_eq!(drives.len(), 4);
        for (drive, (connected, inserted)) in
            drives
                .iter()
                .zip([(true, true), (true, true), (true, false), (false, false)])
        {
            assert_eq!(drive["connected"], connected);
            assert_eq!(drive["disk_inserted"], inserted);
        }
        assert_eq!(
            drives[0]["track_data"],
            json!({
                "sha256": "708597cb9e6996f58d671f251d90fa3cc74dbe4d0d6a02b37d55b37da5125a8d",
                "name": "second.adf",
            })
        );
        assert_eq!(
            drives[1]["track_data"],
            json!({
                "sha256": "f0fdbb654fd226e44e9cc4bc9a5fbbb09fcb01feab1c4a26839d4440534e17d2",
                "name": "first.adf",
            })
        );
        assert_eq!(drives[2].get("track_data"), Some(&Value::Null));
        assert_eq!(drives[3].get("track_data"), Some(&Value::Null));

        for parsed in [
            SaveState::from_json(text).unwrap(),
            SaveState::from_bytes(&bytes).unwrap(),
            serde_json::from_slice::<SaveState>(&bytes).unwrap(),
        ] {
            assert_eq!(parsed.header, saved.header);
            assert_eq!(parsed.cck(), 150);
            assert!(parsed.disk_inserted(0) && parsed.disk_inserted(1));
            assert!(!parsed.disk_inserted(2) && !parsed.disk_inserted(3));
            let restored = parsed.restore(&sources).unwrap();
            assert_eq!(restored.cpu.d_long(3), 0x1234_abcd);
            assert!(
                *restored == original,
                "restored machine differs from saved machine"
            );
            let mut resumed = A500Machine::new(A500Config::default());
            resumed.load_state(&restored);
            assert!(
                resumed.save_state() == original,
                "loading changed the saved machine"
            );
        }
    }
}

#[test]
fn file_keeps_every_configured_bank_and_full_frame_through_restore_and_resave() {
    for (config, height, slow, fast) in [
        (A500Config::bare_512k(VideoStandard::Pal), 568, 0, 0),
        (
            A500Config::standard_1mb(VideoStandard::Pal),
            568,
            512 * 1024,
            0,
        ),
        (
            A500Config::expanded_power_user(VideoStandard::Ntsc),
            480,
            512 * 1024,
            4 * 1024 * 1024,
        ),
    ] {
        let mut machine = A500Machine::new(config.clone());
        machine.physical_memory.map_chip_ram_to_low_memory();
        let banks = [
            ("chip_ram", 0x00_0000, 512 * 1024),
            ("slow_ram", 0xc0_0000, slow),
            ("fast_ram", 0x20_0000, fast),
        ]
        .map(|(name, address, size)| {
            let marker = [0x5e, 0xad, 0xbe, 0xef];
            let expected = (size != 0).then(|| {
                let mut bytes = vec![0; size];
                bytes[..4].copy_from_slice(&marker);
                bytes[size - 4..].copy_from_slice(&[0xfe, 0xdc, 0xba, 0x98]);
                machine
                    .physical_memory
                    .write_bytes_debug(address, &bytes)
                    .unwrap();
                bytes
            });
            (name, expected)
        });
        let mut pixels = vec![0xff20_4080u32; 720 * 568];
        pixels[720 * height - 1] = 0xff90_3010;
        if height == 480 {
            pixels[720 * height] = 0xff0a_91c2;
            pixels[720 * 568 - 1] = 0xa1b2_c3d4;
        }
        let frame = &mut machine.denise.frame_builder;
        *frame = frame.clone().map_storage(|_| pixels.clone());
        let expected_frame: Vec<_> = pixels.iter().flat_map(|word| word.to_le_bytes()).collect();
        let file = capture_synthetic(&machine).to_json().unwrap();
        let restored = restore_synthetic(SaveState::from_bytes(file.as_bytes()).unwrap());
        assert!(*restored == machine.save_state(), "{config:?}");
        let mut resumed = A500Machine::new(A500Config::default());
        resumed.load_state(&restored);
        let resaved = capture_synthetic(&resumed).to_json().unwrap();

        for text in [file, resaved] {
            let document: Value = serde_json::from_str(&text).unwrap();
            let memory = &document["state"]["physical_memory"];
            for (name, expected) in &banks {
                let value = memory.get(name).unwrap();
                if let Some(expected) = expected {
                    canonical_bytes(value, expected, name);
                } else {
                    assert_eq!(value, &Value::Null, "{name}");
                }
            }
            canonical_bytes(
                &document["state"]["denise"]["frame_builder"]["buffer"],
                &expected_frame,
                "full frame buffer",
            );
            let png = STANDARD
                .decode(document["header"]["screenshot_png"].as_str().unwrap())
                .unwrap();
            let preview = image::load_from_memory(&png).unwrap().to_rgba8();
            assert_eq!(preview.dimensions(), (720, height as u32));
            assert_eq!(preview.get_pixel(0, 0).0, [0x20, 0x40, 0x80, 0xff]);
            assert_eq!(
                preview.get_pixel(719, height as u32 - 1).0,
                [0x90, 0x30, 0x10, 0xff]
            );
        }
    }
}

#[test]
fn unsupported_file_version_is_rejected_before_restore() {
    let version_16_state = json!({
        "floppy": {"drives": [{"track_data": {
            "sha256": "0".repeat(64), "name": "old.adf", "modified_tracks": {"0": "AA=="}
        }}]}
    })
    .to_string();
    for version in [1, 15, 16, 18] {
        let header = format!(r#"{{"version":{version},"screenshot_png":[137,80,78,71]}}"#);
        let state = if version == 16 {
            &version_16_state
        } else {
            r#"{"physical_memory":{"chip_ram":[1,2,3],"kickstart_rom":[4,5,6]}}"#
        };
        let expected = SaveStateError::IncompatibleVersion {
            found: version,
            supported: 17,
        };
        for json in [
            format!(r#"{{"header":{header},"state":{state}}}"#),
            format!(r#"{{"state":{state},"header":{header}}}"#),
        ] {
            assert_eq!(SaveState::from_json(&json).unwrap_err(), expected);
            assert_eq!(
                SaveState::from_bytes(json.as_bytes()).unwrap_err(),
                expected
            );
            let direct_decode = serde_json::from_str::<SaveState>(&json)
                .unwrap_err()
                .to_string();
            assert!(direct_decode.contains(&expected.to_string()));
        }
    }

    let machine = A500Machine::new(A500Config::default());
    let saved = capture_synthetic(&machine);
    let document = serde_json::to_value(&saved).unwrap();
    let reordered = format!(
        r#"{{"state":{},"header":{}}}"#,
        document["state"], document["header"]
    );
    assert_eq!(SaveState::from_json(&reordered).unwrap(), saved);
    assert_eq!(SaveState::from_bytes(reordered.as_bytes()).unwrap(), saved);
    assert_eq!(
        serde_json::from_str::<SaveState>(&reordered).unwrap(),
        saved
    );
    assert!(*restore_synthetic(saved) == machine.save_state());
}

#[test]
fn invalid_current_fields_fail_both_file_readers() {
    const PREVIEW: &str = "/header/screenshot_png";
    const CHIP: &str = "/state/physical_memory/chip_ram";
    const SLOW: &str = "/state/physical_memory/slow_ram";
    const FAST: &str = "/state/physical_memory/fast_ram";
    const FRAME: &str = "/state/denise/frame_builder/buffer";
    const ROM: &str = "/state/physical_memory/kickstart_rom";
    const ROM_DIGEST: &str = "/state/physical_memory/kickstart_rom/sha256";
    const EMPTY_DRIVE: &str = "/state/floppy/drives/0/track_data";
    const INSERTED_DISK: &str = "/state/floppy/drives/1/track_data";
    let mut machine = A500Machine::new(A500Config::standard_1mb(VideoStandard::Pal));
    machine
        .floppy
        .set_connections([true, true, false, false])
        .unwrap();
    let disk = SourceImage {
        name: None,
        bytes: patterned_bytes(ADF_BYTES, 49),
    };
    assert!(machine.floppy.drives[1].insert_disk(&disk.bytes));
    let sources = MediaSources {
        disks: [None, Some(disk), None, None],
        ..MediaSources::default()
    };
    let original = serde_json::to_value(SaveState::capture(&machine, &sources).unwrap()).unwrap();
    let reference = original.pointer(INSERTED_DISK).unwrap();
    assert_eq!(reference.get("name"), Some(&Value::Null));
    let mut rom_with_bytes = original.pointer(ROM).unwrap().clone();
    rom_with_bytes["bytes"] = json!("AA==");
    let mut disk_with_bytes = reference.clone();
    disk_with_bytes["bytes"] = json!("AA==");
    let mut disk_with_empty_map = reference.clone();
    disk_with_empty_map["modified_tracks"] = json!({});
    let mut disk_with_track = reference.clone();
    disk_with_track["modified_tracks"] = json!({"0": STANDARD.encode([0xaa; 12_500])});
    let preview = original.pointer(PREVIEW).unwrap().as_str().unwrap();
    let frame = original.pointer(FRAME).unwrap().as_str().unwrap();
    let chip = original.pointer(CHIP).unwrap().as_str().unwrap();
    let cases = [
        ("bad alphabet", PREVIEW, Some(json!("_w=="))),
        ("missing padding", PREVIEW, Some(json!("/w"))),
        ("superfluous padding", PREVIEW, Some(json!("QUJD="))),
        ("nonzero trailing bits", PREVIEW, Some(json!("/x=="))),
        ("embedded whitespace", PREVIEW, Some(json!("/w\n=="))),
        ("numeric array", PREVIEW, Some(json!([137, 80, 78, 71]))),
        (
            "fragment array",
            PREVIEW,
            Some(json!([&preview[..4], &preview[4..]])),
        ),
        ("missing preview", PREVIEW, None),
        ("null preview", PREVIEW, Some(Value::Null)),
        (
            "truncated Chip RAM",
            CHIP,
            Some(json!(&chip[..chip.len() - 4])),
        ),
        ("missing Chip RAM", CHIP, None),
        ("null Chip RAM", CHIP, Some(Value::Null)),
        ("absent configured Slow RAM", SLOW, Some(Value::Null)),
        ("short Slow RAM", SLOW, Some(json!("AAAA"))),
        ("missing absent Fast RAM field", FAST, None),
        ("populated unconfigured Fast RAM", FAST, Some(json!(chip))),
        ("missing frame", FRAME, None),
        ("null frame", FRAME, Some(Value::Null)),
        ("misaligned frame", FRAME, Some(json!("AAAA"))),
        (
            "wrong-sized whole frame",
            FRAME,
            Some(json!(format!("{frame}AAAAAA=="))),
        ),
        (
            "wrong frame geometry",
            "/state/denise/frame_builder/height",
            Some(json!(480)),
        ),
        ("short ROM digest", ROM_DIGEST, Some(json!("00"))),
        (
            "uppercase ROM digest",
            ROM_DIGEST,
            Some(json!("A".repeat(64))),
        ),
        (
            "missing ROM display name",
            "/state/physical_memory/kickstart_rom/name",
            None,
        ),
        (
            "positional ROM reference",
            ROM,
            Some(json!([original.pointer(ROM_DIGEST).unwrap(), null, true])),
        ),
        ("null ROM reference", ROM, Some(Value::Null)),
        ("ROM reference with source bytes", ROM, Some(rom_with_bytes)),
        ("missing empty drive reference field", EMPTY_DRIVE, None),
        (
            "missing disk display name",
            "/state/floppy/drives/1/track_data/name",
            None,
        ),
        (
            "short disk digest",
            "/state/floppy/drives/1/track_data/sha256",
            Some(json!("00")),
        ),
        (
            "malformed drive reference",
            INSERTED_DISK,
            Some(json!([1, 2, 3])),
        ),
        (
            "obsolete empty track map",
            INSERTED_DISK,
            Some(disk_with_empty_map),
        ),
        (
            "obsolete populated track map",
            INSERTED_DISK,
            Some(disk_with_track),
        ),
        (
            "disk reference with source bytes",
            INSERTED_DISK,
            Some(disk_with_bytes),
        ),
        (
            "inserted disk without reference",
            "/state/floppy/drives/0/disk_inserted",
            Some(json!(true)),
        ),
        (
            "empty drive with reference",
            EMPTY_DRIVE,
            Some(reference.clone()),
        ),
        (
            "empty drive with library disk ID",
            "/header/association/mounted/0",
            Some(json!("disk")),
        ),
    ];
    for (case, pointer, replacement) in cases {
        let mut malformed = original.clone();
        if let Some(value) = replacement {
            *malformed.pointer_mut(pointer).unwrap() = value;
        } else {
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            assert!(malformed
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key)
                .is_some());
        }
        let text = malformed.to_string();
        for result in [
            SaveState::from_json(&text),
            SaveState::from_bytes(text.as_bytes()),
        ] {
            let Err(error) = result else {
                panic!("{case}: malformed file was accepted");
            };
            assert!(
                matches!(
                    error,
                    SaveStateError::DeserializationFailed(_) | SaveStateError::CorruptedData(_)
                ),
                "{case}: {error}"
            );
        }
        assert!(serde_json::from_str::<SaveState>(&text).is_err(), "{case}");
    }

    let mut empty_preview = original;
    *empty_preview.pointer_mut(PREVIEW).unwrap() = json!("");
    let decoded = SaveState::from_json(&empty_preview.to_string()).unwrap();
    assert!(decoded.header.screenshot_png.is_empty());
    assert_eq!(decoded.disk_name(1), None);
    assert!(*decoded.restore(&sources).unwrap() == machine.save_state());
}

#[test]
fn dma_registers_roundtrip_in_their_single_owner() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    assert!(machine.agnus.set_dmacon(0x8300));
    assert!(machine.agnus.set_bplcon0(0x2004));
    assert!(machine.agnus.set_ddfstrt(0x0039));
    assert!(machine.agnus.set_ddfstop(0x00d3));
    for _ in 0..4 {
        machine.step_cck();
    }
    assert_eq!(machine.agnus.read_dmaconr() & 0x07ff, 0x0300);
    assert_eq!(machine.agnus.dma.planecount(), 2);
    let saved = capture_synthetic(&machine);
    let json = serde_json::to_value(&saved).unwrap();
    assert_eq!(json["header"]["version"], 17);
    for (name, value) in [
        ("dmacon", 0x0300),
        ("bplcon0", 0x2004),
        ("ddfstrt", 0x0038),
        ("ddfstop", 0x00d0),
    ] {
        assert!(json["state"]["agnus"].get(name).is_none());
        assert_eq!(json["state"]["agnus"]["dma"][name], value);
    }
    let restored = SaveState::from_json(&json.to_string()).unwrap();
    let mut resumed = A500Machine::new(A500Config::default());
    resumed.load_state(&restore_synthetic(restored));
    for _ in 0..128 {
        machine.step_cck();
        resumed.step_cck();
    }
    assert_eq!(machine.save_state(), resumed.save_state());
}

#[test]
fn file_roundtrip_resumes_an_undelivered_rga_event() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine.agnus.vpos = 100;
    machine.step_cck();
    assert_eq!(
        machine.denise.pins.incoming.as_ref().unwrap().rga_strobe,
        Some(config::custom_reg::denise::REGISTER_STRHOR)
    );
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&roundtrip_synthetic(&machine));
    assert_eq!(machine.save_state(), restored.save_state());
    assert!(!restored.denise.sync.synchronized);
    for _ in 0..8 {
        machine.step_cck();
        restored.step_cck();
        assert_eq!(machine.save_state(), restored.save_state());
    }
    assert!(restored.denise.sync.synchronized);
}

#[test]
fn file_roundtrip_rebuilds_sprite_comparisons_with_an_active_vertical_latch() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine.agnus.vpos = 100;
    machine.agnus.hpos = 20;
    machine.agnus.dma.write_dmacon(0x8220);
    machine.agnus.sprite_dma.set_position(0, 0x6440);
    machine.agnus.sprite_dma.set_control(0, 0x6500);
    machine.step_cck();
    assert!(machine.agnus.sprite_dma.state().sprite_dma_active[0]);

    let mut restored = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    restored.load_state(&roundtrip_synthetic(&machine));
    assert_eq!(machine.save_state(), restored.save_state());
    for _ in 0..500 {
        machine.step_cck();
        restored.step_cck();
    }
    assert!(!machine.agnus.sprite_dma.state().sprite_dma_active[0]);
    assert_eq!(machine.save_state(), restored.save_state());
}

#[test]
fn file_roundtrip_resumes_a_partial_copper_instruction_after_a_denied_fetch() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.cpu.state.set_stopped(true);
        for (offset, byte) in [0x01, 0x80, 0x0F, 0x00, 0xFF, 0xFF, 0xFF, 0xFE]
            .iter()
            .copied()
            .enumerate()
        {
            machine
                .physical_memory
                .write_chip_byte_debug(((0) as u32).wrapping_add(offset as u32), byte);
        }
        machine.agnus.dma.write_dmacon(0x8280);
        machine.agnus.copper.set_dma_enabled(true);
        machine.agnus.copper.restart_list1();
        machine.agnus.hpos = 0x3F;
        machine.agnus.vpos = 100;
        machine.step_cck();
        assert_eq!(machine.agnus.copper.cop_pc, 2);
        assert_eq!(machine.agnus.pending_copper_write, None);

        // A four-plane hires fetch denies the second Copper word.
        machine.agnus.dma.write_dmacon(0x8100);
        machine.agnus.dma.set_bplcon0(0xC000);
        machine.step_cck();
        assert_eq!(machine.agnus.copper.cop_pc, 2);
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&roundtrip_synthetic(&machine));
        machine.agnus.dma.write_dmacon(0x0100);
        restored.agnus.dma.write_dmacon(0x0100);
        for _ in 0..12 {
            machine.step_cck();
            restored.step_cck();
            assert_eq!(machine.save_state(), restored.save_state());
        }
        assert_eq!(restored.denise.color[0], 0x0F00);
    }
}

#[test]
fn file_roundtrip_resumes_blitter_source_carry_and_pending_destination() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.cpu.state.set_stopped(true);
        machine.agnus.hpos = 0x3f;
        machine.agnus.dma.write_dmacon(0x8640);
        let b = &mut machine.agnus.blitter;
        b.bltcon0 = 0x49f0;
        b.bltafwm = 0xffff;
        b.bltalwm = 0xffff;
        b.bltdpt = 16;
        b.set_dma_enabled(true);
        b.trigger_blit(0x0042);
        for (offset, byte) in [0x12, 0x34, 0x56, 0x78].iter().copied().enumerate() {
            machine
                .physical_memory
                .write_chip_byte_debug(((0) as u32).wrapping_add(offset as u32), byte);
        }
        for _ in 0..4 {
            machine.step_cck();
        }
        assert_eq!(machine.agnus.blitter.bltapt, 4);
        assert_eq!(machine.agnus.blitter.bltdpt, 16);
        assert!(machine.agnus.blitter.is_busy);
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&roundtrip_synthetic(&machine));
        for _ in 0..8 {
            machine.step_cck();
            restored.step_cck();
            assert_eq!(machine.save_state(), restored.save_state());
        }
        assert_eq!(
            &(16..20)
                .map(|address| restored
                    .physical_memory
                    .read_chip_byte_debug(address as u32))
                .collect::<Vec<_>>(),
            &[0x01, 0x23, 0x45, 0x67]
        );
        assert!(!restored.agnus.blitter.is_busy);
        assert_ne!(restored.paula.interrupts.intreq & 0x0040, 0);
    }
}

#[test]
fn file_roundtrip_preserves_drawn_pixels_and_resumes_active_display() {
    let program = include_bytes!("../../../tests/mixed_scene/program.bin");
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.physical_memory.map_chip_ram_to_low_memory();
    for (offset, byte) in program.iter().copied().enumerate() {
        machine
            .physical_memory
            .write_chip_byte_debug(0x1000 + offset as u32, byte);
    }
    machine.cpu.state.set_sr(0x2700);
    machine.set_pc_and_prime_prefetch(0x1000);
    for _ in 0..16 {
        machine.step_frame();
    }
    for _ in 0..(313 * 228) {
        machine.step_cck();
        if machine.agnus.vpos == 120 && machine.agnus.hpos == 100 {
            break;
        }
    }
    assert_eq!((machine.agnus.vpos, machine.agnus.hpos), (120, 100));
    let row = (120 - 28) * 2;
    let not_yet_drawn = (360..440)
        .map(|x| machine.denise.frame_builder.get_pixel(x, row))
        .collect::<Vec<_>>();
    assert_ne!(
        machine
            .denise
            .frame_builder
            .get_pixel(300 - 196, (100 - 28) * 2),
        0xff000000
    );
    assert!(machine.denise.bpl_armed);
    assert!(machine
        .denise
        .pipeline_pixels_valid
        .iter()
        .all(|valid| *valid));
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&roundtrip_synthetic(&machine));
    assert!(restored.save_state() == machine.save_state());
    assert_eq!(machine.denise.frame_builder, restored.denise.frame_builder);
    for _ in 0..64 {
        machine.step_cck();
        restored.step_cck();
        assert_eq!(machine.denise, restored.denise);
        assert_eq!(machine.agnus, restored.agnus);
    }
    assert_ne!(
        restored.denise.frame_builder.get_pixel(600 - 196, row),
        0xff000000
    );
    assert!(
        (360..440).zip(not_yet_drawn).any(|(x, old)| restored
            .denise
            .frame_builder
            .get_pixel(x, row)
            != old),
        "Restored display did not draw new pixels farther along the current line"
    );
    machine.step_frame();
    restored.step_frame();
    assert!(restored.save_state() == machine.save_state());
}

#[test]
fn file_roundtrip_resumes_uart_transmit_and_partial_receive_shifters() {
    let mut machine = A500Machine::new(A500Config::default());
    let serial = &mut machine.paula.serial_port;
    serial.write_serper(3);
    serial.write_serdat(0x0155);
    // Start bit and three data bits of $A6, with the transmitter running too.
    for high in [false, false, true, true] {
        for _ in 0..4 {
            serial.step_cck(high, false);
        }
    }
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&roundtrip_synthetic(&machine));
    let mut received = 0;
    for high in [false, false, true, false, true, true] {
        for _ in 0..4 {
            let request = machine.paula.serial_port.step_cck(high, false);
            received |= request;
            assert_eq!(restored.paula.serial_port.step_cck(high, false), request);
            assert_eq!(machine.paula.serial_port, restored.paula.serial_port);
        }
    }
    assert_eq!(received & 0x0800, 0x0800);
    assert_eq!(
        restored.paula.serial_port.read_status(true) & 0x03ff,
        0x01a6
    );
    // The first CCK loaded TX; finish the final stop-bit interval as well.
    assert_eq!(machine.paula.serial_port.step_cck(true, true), 0);
    assert_eq!(restored.paula.serial_port.step_cck(true, true), 0);
    assert_eq!(machine.paula.serial_port, restored.paula.serial_port);
    assert_eq!(
        restored.paula.serial_port.read_status(true) & 0x3000,
        0x3000
    );
}

#[test]
fn file_roundtrip_resumes_cia_serial_bits_timer_outputs_and_pc_pulses() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.cia_a.ta_latch = 1;
    machine.cia_a.ta_counter = 1;
    assert!(machine.cia_a.set_cra(0x47));
    for _ in 0..2 {
        machine.cia_a.step_cck();
    }
    assert!(machine.cia_a.set_sdr(0xa6));
    for _ in 0..33 {
        machine.cia_a.step_cck();
    }
    // Three bits received and a Port B access still in the PC pipeline.
    for high in [true, false, true] {
        machine.cia_b.receive_serial_pins(high, false);
        machine.cia_b.step_cck();
        machine.cia_b.receive_serial_pins(high, true);
        machine.cia_b.step_cck();
    }
    machine.cia_b.prb();
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&roundtrip_synthetic(&machine));
    for high in [false, false, true, true, false] {
        for cnt in [false, true] {
            for chip in [&mut machine.cia_b, &mut restored.cia_b] {
                chip.receive_serial_pins(high, cnt);
                chip.step_cck();
            }
            assert_eq!(machine.cia_b, restored.cia_b);
        }
    }
    assert_eq!(restored.cia_b.sdr(), 0xa6);
    assert_eq!(restored.cia_b.icr() & 8, 8);
    for _ in 0..200 {
        machine.cia_a.step_cck();
        restored.cia_a.step_cck();
        assert_eq!(machine.cia_a, restored.cia_a);
    }
    assert_eq!(restored.cia_a.icr() & 8, 8);
    restored.cia_a.reset();
    assert!(restored.cia_a.pins.sp_output_high);
    assert!(restored.cia_a.pins.cnt_output_high);
    assert!(restored.cia_a.pins.pc_high);
}

#[test]
fn file_roundtrip_resumes_disk_write_fifo_and_partial_bit_cell() {
    let mut machine = A500Machine::new(A500Config::default());
    let paula = &mut machine.paula;
    assert!(paula.set_dmacon(0x8210));
    assert!(paula.set_adkcon(0x8100));
    assert!(paula.set_dsklen(0xc003));
    paula.step_cck();
    assert!(paula.set_dsklen(0xc003));
    paula.step_cck();
    for word in [0xa55a, 0x8001, 0xffff] {
        assert!(paula.set_dskdat(word));
    }
    paula.step_cck();
    for _ in 0..19 {
        paula.step_cck();
    }
    assert_eq!(paula.disk.buffered_words(), 2);
    assert!(paula.pins.disk_write_enable);
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&roundtrip_synthetic(&machine));
    let mut transitions = 0;
    for _ in 0..350 {
        machine.paula.step_cck();
        restored.paula.step_cck();
        assert_eq!(machine.paula, restored.paula);
        transitions += usize::from(!restored.paula.pins.disk_write_high);
    }
    assert_eq!(
        transitions, 24,
        "26 set bits minus two emitted before the snapshot"
    );
    assert_eq!(restored.paula.disk.buffered_words(), 0);
    assert!(!restored.paula.pins.disk_write_enable);
    restored.paula.reset();
    assert_eq!(restored.paula, machine_loop::paula::Paula::default());
}

#[test]
fn only_full_memory_restores_a_guest_pulse_in_the_track_gap() {
    const CELL_HZ: u64 = 500_000;
    const TRACK_CELLS: u64 = 100_000;
    // Each of the 11 sectors occupies 1088 raw bytes. This cell is in the gap.
    const GAP_CELL: u64 = 12_200 * 8 + 1;

    let standard = VideoStandard::Pal;
    let clock_hz = u64::from(standard.color_clock_hz());
    let mut machine = A500Machine::new(A500Config::standard_1mb(standard));
    machine.cpu.state.set_stopped(true);
    machine.step_cycles(150);
    let adf = patterned_bytes(ADF_BYTES, 49);
    let sources = MediaSources {
        rom: None,
        disks: [Some(image("gap.adf", adf.clone())), None, None, None],
    };

    let floppy = &mut machine.floppy;
    assert!(floppy.drives[0].insert_disk(&adf));
    assert!(floppy.drives[0].disk_change_flip_flop);
    let clean_tracks = floppy.drives[0].tracks().clone();
    // Select DF0 with the motor on. An outward step clears the latch at track zero.
    for control in [0x7f, 0x76, 0x77] {
        floppy.pins.control_input = control;
        floppy.step_cck();
    }
    assert!(!floppy.drives[0].disk_change_flip_flop);
    assert_eq!(floppy.drives[0].side, 0);
    assert_eq!(floppy.drives[0].cylinder, 0);
    for _ in 0..clock_hz.div_ceil(2) {
        floppy.step_cck();
    }
    assert!(floppy.drives[0].is_ready());
    let mut found_index = false;
    for _ in 0..clock_hz.div_ceil(5) {
        let previous = floppy.pins.index_high;
        floppy.step_cck();
        if previous && !floppy.pins.index_high {
            found_index = true;
            break;
        }
    }
    assert!(found_index, "index pulse must recur after one revolution");
    for _ in 0..(GAP_CELL * clock_hz).div_ceil(CELL_HZ) + 2 {
        floppy.step_cck();
    }

    let expected_reference = json!({
        "sha256": "f0fdbb654fd226e44e9cc4bc9a5fbbb09fcb01feab1c4a26839d4440534e17d2",
        "name": "gap.adf",
    });
    let clean_document =
        serde_json::to_value(SaveState::capture(&machine, &sources).unwrap()).unwrap();
    assert_eq!(
        clean_document["state"]["floppy"]["drives"][0]["track_data"],
        expected_reference
    );

    machine.floppy.pins.write_enable_high = false;
    machine.floppy.pins.write_data_high = false;
    machine.floppy.step_cck();
    assert!(machine.floppy.drives[0].tracks() != &clean_tracks);

    let written = machine.save_state();
    assert_eq!(written.cck, 150);
    let text = SaveState::capture(&machine, &sources)
        .unwrap()
        .to_json()
        .unwrap();
    let document: Value = serde_json::from_str(&text).unwrap();
    let drive = &document["state"]["floppy"]["drives"][0];
    assert_eq!(document["state"]["cck"], 150);
    assert_eq!(drive["rotation_bit"], GAP_CELL);
    assert_eq!(drive["disk_change_flip_flop"], false);
    let phase = drive["cell_phase"].as_u64().unwrap();
    assert!(
        (3..=4).contains(&(phase / CELL_HZ)),
        "pulse must be inside the cell"
    );
    assert_eq!(drive["track_data"], expected_reference);

    let restored = SaveState::from_bytes(text.as_bytes())
        .unwrap()
        .restore(&sources)
        .unwrap();
    let mut expected = written.clone();
    expected.floppy.drives[0] = expected.floppy.drives[0].with_tracks(clean_tracks);
    assert!(
        *restored == expected,
        "file restore must only replace the tracks"
    );

    let mut resumed = A500Machine::new(A500Config::default());
    resumed.load_state(&written);
    assert!(resumed.save_state() == written);

    // Continue the in-flight recording from the pure snapshot and compare the drive outputs.
    let mut live_write = machine.floppy.clone();
    let mut resumed_write = resumed.floppy.clone();
    for tick in 0..128 {
        for target in [&mut live_write, &mut resumed_write] {
            target.pins.write_data_high = tick % 7 != 0;
            target.pins.write_enable_high = tick >= 28;
            target.step_cck();
        }
        assert_eq!(resumed_write.pins, live_write.pins);
    }
    assert!(resumed_write == live_write);

    // Release recording, then read the first sector and the gap on the next revolution.
    let mut file_floppy = restored.floppy;
    let mut sector = [0u8; 1088];
    let revolution_ticks = TRACK_CELLS * clock_hz / CELL_HZ;
    for tick in 1..=revolution_ticks {
        for target in [&mut machine.floppy, &mut resumed.floppy, &mut file_floppy] {
            target.pins.write_enable_high = true;
            target.pins.write_data_high = true;
            target.step_cck();
        }
        assert_eq!(resumed.floppy.pins, machine.floppy.pins);

        if tick == revolution_ticks {
            assert!(!resumed.floppy.pins.read_data_high);
            assert!(file_floppy.pins.read_data_high);
        } else {
            assert_eq!(file_floppy.pins, machine.floppy.pins);
        }

        let angle = phase + tick * CELL_HZ;
        if angle % clock_hz < CELL_HZ {
            let cell = ((GAP_CELL + angle / clock_hz - 1) % TRACK_CELLS) as usize;
            if cell < sector.len() * 8 && !file_floppy.pins.read_data_high {
                sector[cell / 8] |= 0x80 >> (cell % 8);
            }
        }
    }
    let (track, sector_number, payload) =
        machine_loop::floppy::mfm::decode_amiga_sector(&sector).unwrap();
    assert_eq!((track, sector_number), (0, 0));
    assert_eq!(payload.as_slice(), &adf[..512]);
    assert_eq!(sources.disks[0].as_ref().unwrap().bytes, adf);
}

#[test]
fn malformed_admission_state_is_a_load_error_without_defaulting_or_runtime_overflow() {
    let mut machine = A500Machine::new(A500Config::default());
    assert!(machine.paula.set_audper(0, 0x1234));
    let before = machine.save_state();
    let original = serde_json::to_value(capture_synthetic(&machine)).unwrap();
    for missing_cpu_timing in [false, true] {
        let mut malformed = original.clone();
        if missing_cpu_timing {
            malformed["state"]["cpu"]
                .as_object_mut()
                .unwrap()
                .remove("bus_timing");
        } else {
            malformed["state"]["physical_memory"]
                .as_object_mut()
                .unwrap()
                .remove("chip_budget");
        }
        let json = serde_json::to_string(&malformed).unwrap();
        assert!(matches!(
            SaveState::from_json(&json),
            Err(SaveStateError::DeserializationFailed(_))
        ));
    }
    let mut wrong_type = original.clone();
    wrong_type["state"]["paula"]["mutations"] = serde_json::json!("not a mutation sequence");
    assert!(matches!(
        SaveState::from_json(&serde_json::to_string(&wrong_type).unwrap()),
        Err(SaveStateError::DeserializationFailed(_))
    ));
    let mut oversized = original.clone();
    let mutation = oversized["state"]["paula"]["mutations"][0].clone();
    oversized["state"]["paula"]["mutations"] = serde_json::json!(vec![mutation; 33]);
    let json = serde_json::to_string(&oversized).unwrap();
    let loaded = std::panic::catch_unwind(|| SaveState::from_json(&json));
    assert!(matches!(
        loaded.unwrap(),
        Err(SaveStateError::DeserializationFailed(_))
    ));
    assert_eq!(machine.paula.mutations.len(), 1);
    assert_eq!(machine.save_state(), before);
}
