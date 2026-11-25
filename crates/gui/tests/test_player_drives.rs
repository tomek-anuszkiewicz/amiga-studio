#![allow(clippy::unwrap_used, clippy::panic)]
#[cfg(not(target_arch = "wasm32"))]
mod common;

use debugger::{
    launch::{DeviceKind, MachineSetup, StartupDisk},
    DebuggerSession,
};
use gui::host_commands::{
    GameStartup, HostCommand, HostObservation, HostReply, HostRequest, HostValue,
};
use gui::host_owner::MachineOwner;
use gui::host_video::VideoExchange;
use gui::EditRegister;
use physical_memory::{AddressBus, BusResult};

fn rom() -> Vec<u8> {
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    rom
}
fn execute(owner: &mut MachineOwner, command: HostCommand) -> HostValue {
    request(owner, command).value.unwrap()
}

fn request(owner: &mut MachineOwner, command: HostCommand) -> HostReply {
    owner.execute(HostRequest {
        id: 1,
        generation: owner.status().generation,
        command,
    })
}
fn cia_write(session: &mut DebuggerSession, address: u32, value: u8) {
    for _ in 0..12 {
        if session.machine.memory_bus().write_byte(address, value) == BusResult::Ready(()) {
            session.machine.step_cck();
            return;
        }
        session.machine.step_cck();
    }
    panic!("CIA write not accepted");
}
fn finish(session: &mut DebuggerSession) -> Vec<(u64, u16)> {
    let mut result = Vec::new();
    let mut pointer = session.machine.agnus.dskpt;
    for _ in 0..1_000_000 {
        session.machine.step_cck();
        let next = session.machine.agnus.dskpt;
        if next != pointer {
            assert_eq!(next, pointer + 2);
            result.push((
                session.machine.cck,
                session
                    .machine
                    .physical_memory
                    .read_chip_word_debug(pointer),
            ));
            pointer = next;
        }
        if !session.machine.paula.dma_active {
            return result;
        }
    }
    panic!("Disk DMA timed out");
}

fn prepare_steady_read(session: &mut DebuggerSession, drive: usize) {
    let clock_hz = session.machine.config.video_standard().color_clock_hz();
    let floppy = &mut session.machine.floppy;
    for _ in 0..clock_hz.div_ceil(2) {
        if floppy.drives[drive].is_ready() {
            break;
        }
        floppy.step_cck();
    }
    assert!(floppy.drives[drive].is_ready(), "DF{drive} spin-up");
    for _ in 0..clock_hz.div_ceil(5) {
        let released = floppy.pins.index_high;
        floppy.step_cck();
        if released && !floppy.pins.index_high {
            return;
        }
    }
    panic!("DF{drive} did not reach INDEX");
}

const ADF_BYTES: usize = 901_120;
const WRITTEN_SYNC: u16 = 0x9124;
const WRITE_WORDS: [u16; 8] = [
    0xaaaa,
    WRITTEN_SYNC,
    0x9254,
    0xaa49,
    0x24a9,
    0x2a52,
    0x492a,
    0x9292,
];

fn owner_with_latched_motors() -> MachineOwner {
    let mut session = Box::new(DebuggerSession::new());
    session.load_rom(&rom()).unwrap();
    session.machine.physical_memory.map_chip_ram_to_low_memory();
    session.set_debugger_enabled(false);
    session
        .machine
        .floppy
        .set_connections([true, true, true, false])
        .unwrap();
    for (address, value) in [
        (0xbfd100, 0xff),
        (0xbfd300, 0xff),
        (0xbfd100, 0x7f),
        (0xbfd100, 0x47),
    ] {
        cia_write(&mut session, address, value);
    }
    session.machine.step_cycles(3);
    assert!(session.machine.floppy.drives[..3]
        .iter()
        .all(|drive| drive.motor_on));
    MachineOwner::new(session)
}

fn mount(owner: &mut MachineOwner, drive: usize, id: Option<&str>, source: Option<u8>) {
    execute(
        owner,
        HostCommand::MountDisk {
            name: None,
            drive,
            disk_id: id.map(str::to_owned),
            bytes: source.map(|fill| vec![fill; ADF_BYTES]),
        },
    );
}

fn guest_dma(
    owner: &mut MachineOwner,
    drive: usize,
    address: u16,
    length: u16,
    sync: u16,
    fields: usize,
) {
    use config::custom_reg::{agnus, paula};

    // Select the mounted disk, then poll the active-low READY bit on CIA-A.
    let select = 0x7f & !(1 << (drive + 3));
    let wait_for_ready = vec![
        0x13, 0xfc, 0, select, 0, 0xbf, 0xd1, 0, // MOVE.B #select,$BFD100
        0x08, 0x39, 0, 5, 0, 0xbf, 0xe0, 1, // BTST #5,$BFE001
        0x66, 0xf6, // BNE.S back to BTST
        0x60, 0xfe, // BRA.S here once ready
    ];
    let ready_pc = 0x10000 + wait_for_ready.len() as u32 - 2;
    load_guest_program(owner, wait_for_ready);
    let exchange = VideoExchange::new(vec![0; owner.video_storage_len()]);
    let mut tick = 0;
    for field in 0..=26 {
        let HostValue::Cpu(cpu) = execute(owner, HostCommand::ReadCpu) else {
            panic!("Missing guest CPU state");
        };
        if cpu.instruction_pc() == ready_pc {
            break;
        }
        assert!(field < 26, "DF{drive} did not report READY");
        run_field(owner, &exchange, &mut tick);
    }

    let mut program = Vec::new();
    for (register, value) in [
        (paula::REGISTER_DSKLEN, 0),
        (agnus::REGISTER_DSKPTH, 0),
        (agnus::REGISTER_DSKPTL, address),
        (paula::REGISTER_DSKSYNC, sync),
        (paula::REGISTER_ADKCON, 0x8500),
        (agnus::REGISTER_DMACON, 0x8210),
        (paula::REGISTER_DSKLEN, length),
        (paula::REGISTER_DSKLEN, length),
    ] {
        // MOVE.W #value,$DFFxxx; the guest starts Paula DMA through the bus.
        program.extend([0x33, 0xfc]);
        program.extend(value.to_be_bytes());
        program.extend((0xdff000 + u32::from(register)).to_be_bytes());
    }
    program.extend([0x60, 0xfe]);
    load_guest_program(owner, program);
    for _ in 0..fields {
        run_field(owner, &exchange, &mut tick);
    }
}

fn load_guest_program(owner: &mut MachineOwner, program: Vec<u8>) {
    execute(
        owner,
        HostCommand::WriteMemory {
            address: 0x10000,
            bytes: program,
            restart_instruction: false,
        },
    );
    execute(
        owner,
        HostCommand::SetRegister {
            register: EditRegister::PC,
            value: 0x10000,
        },
    );
}

fn run_field(owner: &mut MachineOwner, exchange: &VideoExchange, tick: &mut u32) {
    execute(owner, HostCommand::StepField);
    loop {
        owner.publish(exchange).unwrap();
        exchange
            .try_consume(owner.status().generation, |_, _| ())
            .unwrap();
        if !owner.status().running {
            return;
        }
        *tick += 1;
        assert!(*tick < 1000, "Guest did not reach the field boundary");
        owner.advance(f64::from(*tick) / 1000.0, || 0.0);
    }
}

fn guest_write(owner: &mut MachineOwner, drive: usize, words: [u16; 8]) {
    execute(
        owner,
        HostCommand::WriteMemory {
            address: 0x2000,
            bytes: words.into_iter().flat_map(u16::to_be_bytes).collect(),
            restart_instruction: false,
        },
    );
    guest_dma(owner, drive, 0x2000, 0xc008, WRITTEN_SYNC, 1);
}

fn guest_read(owner: &mut MachineOwner, drive: usize, sync: u16, word_count: u16) -> Vec<u16> {
    let length = usize::from(word_count) * 2;
    let mut destination = vec![0; length];
    destination.extend(0xdeadu16.to_be_bytes());
    execute(
        owner,
        HostCommand::WriteMemory {
            address: 0x4000,
            bytes: destination,
            restart_instruction: false,
        },
    );
    guest_dma(owner, drive, 0x4000, 0x8000 | word_count, sync, 11);
    let HostValue::Memory { bytes, .. } = execute(
        owner,
        HostCommand::ReadMemory {
            address: 0x4000,
            length: length + 2,
        },
    ) else {
        panic!("Missing guest DMA buffer");
    };
    assert_eq!(bytes[length..], [0xde, 0xad]);
    bytes[..length]
        .chunks_exact(2)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        .collect()
}

fn guest_read_marker(owner: &mut MachineOwner, drive: usize) -> Vec<u16> {
    guest_read(owner, drive, WRITTEN_SYNC, 6)
}

fn guest_read_adf_payload(owner: &mut MachineOwner, drive: usize) -> u32 {
    let words = guest_read(owner, drive, 0x4489, 540);
    // A transfer may start after the first or second sync word of a sector.
    let odd = if words[0] == 0x4489 { 29 } else { 28 };
    let even = odd + 256;
    let long = |index| (u32::from(words[index]) << 16) | u32::from(words[index + 1]);
    ((long(odd) & 0x55555555) << 1) | (long(even) & 0x55555555)
}

struct Captured {
    document: debugger::SaveState,
    state: Box<debugger::A500State>,
    header: debugger::SaveStateHeader,
}

fn captured_state(owner: &mut MachineOwner) -> Box<debugger::A500State> {
    let HostValue::MachineState(state) = execute(
        owner,
        HostCommand::Observe(gui::host_commands::HostObservation::MachineState),
    ) else {
        panic!("Missing full machine state");
    };
    state
}

fn saved_machine(owner: &mut MachineOwner) -> Captured {
    let HostValue::Snapshot(json) = execute(owner, HostCommand::SaveSnapshot) else {
        panic!("Missing machine snapshot");
    };
    let document = debugger::SaveState::from_json(&json).unwrap();
    Captured {
        header: document.header.clone(),
        state: captured_state(owner),
        document,
    }
}

fn raw_tracks(state: &debugger::A500State, drive: usize) -> serde_json::Value {
    serde_json::to_value(&state.floppy.drives[drive]).unwrap()["track_data"].clone()
}

fn game_reset(disks: [Option<(&str, Option<u8>)>; 4]) -> HostCommand {
    HostCommand::ResetPlayer(Some(GameStartup {
        game_id: "game".into(),
        disks: disks.map(|disk| {
            disk.map(|(id, source)| StartupDisk {
                name: None,
                id: id.into(),
                bytes: source.map(|fill| vec![fill; ADF_BYTES]),
            })
        }),
    }))
}

#[test]
fn startup_reset_preserves_written_copies_and_uses_slot_contents_when_moving_disks() {
    let mut owner = owner_with_latched_motors();
    let mut initial = saved_machine(&mut owner);
    initial.document.header.association.game_id = Some("game".into());
    let ports = owner.status().ports;
    execute(
        &mut owner,
        HostCommand::RestorePlayer {
            media: gui::host_commands::RestoreMedia::default(),
            json: initial.document.to_json().unwrap(),
            game_id: Some("game".into()),
            ports,
            resume: false,
        },
    );

    for (drive, id, last_word) in [
        (0, "a", 0x9252),
        (1, "shared", 0x9492),
        (2, "shared", 0x9294),
    ] {
        mount(&mut owner, drive, Some(id), Some(0));
        let mut marker = WRITE_WORDS;
        marker[7] = last_word;
        guest_write(&mut owner, drive, marker);
        assert_eq!(guest_read_marker(&mut owner, drive), marker[2..]);
    }
    execute(&mut owner, HostCommand::SaveQuickSlot(1));
    let written = saved_machine(&mut owner);
    assert_ne!(raw_tracks(&written.state, 1), raw_tracks(&written.state, 2));
    let written_sources = debugger::MediaSources {
        rom: Some(debugger::SourceImage {
            name: None,
            bytes: rom(),
        }),
        disks: std::array::from_fn(|drive| {
            (drive < 3).then(|| debugger::SourceImage {
                name: None,
                bytes: vec![0; ADF_BYTES],
            })
        }),
    };
    let document = serde_json::to_value(&written.document).unwrap();
    assert!(
        document["state"]["floppy"]["drives"][0]["track_data"]["modified_tracks"]
            .as_object()
            .unwrap()
            .values()
            .any(|track| track.as_str().is_some_and(|base64| base64.len() > 20_000)),
        "Guest writing should exercise recorded pulse phases"
    );
    assert_eq!(
        *written.document.clone().restore(&written_sources).unwrap(),
        *written.state
    );
    mount(&mut owner, 1, None, None);
    mount(&mut owner, 2, None, None);

    let before = saved_machine(&mut owner);
    for running in [false, true] {
        if running {
            execute(&mut owner, HostCommand::Run);
        }
        let prior = owner.status();
        assert_eq!(prior.running, running);
        for (disk, destination, startup) in [
            // Two written copies have no preference for DF0.
            (
                "shared",
                "DF0",
                [
                    Some(("shared", Some(0))),
                    Some(("source", Some(0x44))),
                    None,
                    None,
                ],
            ),
            // One written copy in DF0 has no preference between DF1 and DF2.
            (
                "a",
                "DF1",
                [
                    Some(("source", Some(0x44))),
                    Some(("a", Some(0))),
                    Some(("a", Some(0))),
                    None,
                ],
            ),
        ] {
            let rejected = request(&mut owner, game_reset(startup));
            let error = rejected.value.unwrap_err();
            assert!(error.contains("ambiguous"), "{error}");
            assert!(
                error.contains(&format!("Startup disk {disk} for {destination}")),
                "{error}"
            );
            assert_eq!(rejected.status.generation, prior.generation);
            assert_eq!(rejected.status.cck, prior.cck);
            assert_eq!(rejected.status.running, running);
            assert_eq!(
                rejected.status.drive_reset_sequence,
                prior.drive_reset_sequence
            );
            assert_eq!(rejected.status.drives, prior.drives);
            let after = saved_machine(&mut owner);
            assert_eq!(before.state, after.state);
            assert_eq!(before.header.association, after.header.association);
        }
    }

    let reset = request(
        &mut owner,
        game_reset([
            Some(("source", Some(0x44))),
            Some(("shared", None)),
            Some(("shared", None)),
            None,
        ]),
    );
    assert!(reset.status.running);
    assert_eq!(reset.status.cck, 0);
    let HostValue::PlayerReset(ids) = reset.value.unwrap() else {
        panic!("Missing reset disk association");
    };
    assert_eq!(
        ids,
        [
            Some("source".into()),
            Some("shared".into()),
            Some("shared".into()),
            None
        ]
    );
    let reset = saved_machine(&mut owner);
    assert_eq!(reset.header.association.mounted, ids);
    let mut source = DebuggerSession::new();
    assert!(source.machine.floppy.drives[0].insert_disk(&vec![0x44; ADF_BYTES]));
    assert_eq!(
        raw_tracks(&reset.state, 0),
        raw_tracks(&source.machine.save_state(), 0)
    );
    for drive in [1, 2] {
        assert_eq!(
            raw_tracks(&reset.state, drive),
            raw_tracks(&written.state, drive)
        );
    }

    execute(
        &mut owner,
        game_reset([None, Some(("shared", None)), Some(("a", None)), None]),
    );
    let moved = saved_machine(&mut owner);
    assert!(!moved.state.floppy.drives[0].disk_inserted);
    assert_eq!(raw_tracks(&moved.state, 2), raw_tracks(&written.state, 0));

    execute(&mut owner, HostCommand::LoadQuickSlot { slot: 1, ports });
    let loaded = saved_machine(&mut owner);
    assert_eq!(loaded.state.floppy.drives, written.state.floppy.drives);
    assert_eq!(loaded.header.association, written.header.association);
    execute(
        &mut owner,
        game_reset([
            Some(("shared", None)),
            Some(("shared", None)),
            Some(("a", None)),
            None,
        ]),
    );
    let reset_from_slot = saved_machine(&mut owner);
    assert_eq!(
        reset_from_slot.header.association.mounted,
        [
            Some("shared".into()),
            Some("shared".into()),
            Some("a".into()),
            None
        ]
    );
    for (drive, original) in [(0, 2), (1, 1), (2, 0)] {
        assert_eq!(
            raw_tracks(&reset_from_slot.state, drive),
            raw_tracks(&written.state, original)
        );
    }
    assert!(!reset_from_slot.state.floppy.drives[3].connected);
}

#[test]
fn guest_reads_written_disk_after_replacing_it_and_reinserting_it_without_a_snapshot() {
    let mut owner = owner_with_latched_motors();
    mount(&mut owner, 0, Some("disk-a"), Some(0));
    guest_write(&mut owner, 0, WRITE_WORDS);
    let expected = &WRITE_WORDS[2..];
    assert_eq!(
        guest_read_marker(&mut owner, 0),
        expected,
        "write before the swap"
    );
    let observation = execute(
        &mut owner,
        HostCommand::Observe(HostObservation::Drive {
            drive: 0,
            expected_adf: Some(vec![0; ADF_BYTES]),
        }),
    );
    assert!(
        matches!(
            observation,
            HostValue::Drive {
                inserted: true,
                tracks_match: Some(false),
            }
        ),
        "DF0 should differ from its original ADF after the guest write: {observation:?}"
    );

    mount(&mut owner, 0, Some("disk-b"), Some(0x11));
    for (drive, bytes, expected) in [
        (0, None, "Disk is unavailable"),
        (0, Some(vec![0; 10]), "Unsupported ADF length"),
        (3, Some(vec![0; ADF_BYTES]), "Drive is disconnected"),
        (4, None, "Invalid drive number"),
    ] {
        let reply = request(
            &mut owner,
            HostCommand::MountDisk {
                name: None,
                drive,
                bytes,
                disk_id: Some("disk-c".into()),
            },
        );
        assert!(reply.value.unwrap_err().contains(expected));
        assert!(reply.status.drives[0].inserted);
    }
    assert_eq!(guest_read_adf_payload(&mut owner, 0), 0x11111111);
    mount(&mut owner, 0, Some("disk-a"), Some(0));
    assert_eq!(
        guest_read_marker(&mut owner, 0),
        expected,
        "read after the swap"
    );
    mount(&mut owner, 0, None, None);
    mount(&mut owner, 1, Some("disk-a"), None);
    assert!(!owner.status().drives[0].inserted);
    assert_eq!(
        guest_read_marker(&mut owner, 1),
        expected,
        "read after moving to DF1"
    );
}

#[test]
fn independently_written_copies_with_one_id_can_return_to_their_original_drives() {
    let mut owner = owner_with_latched_motors();
    let mut other_words = WRITE_WORDS;
    other_words[7] = 0x9492;
    for (drive, words) in [(0, WRITE_WORDS), (1, other_words)] {
        mount(&mut owner, drive, Some("shared"), Some(0));
        guest_write(&mut owner, drive, words);
    }
    mount(&mut owner, 0, None, None);
    mount(&mut owner, 1, None, None);

    mount(&mut owner, 2, Some("other"), Some(0x11));
    let ambiguous = request(
        &mut owner,
        HostCommand::MountDisk {
            name: None,
            drive: 2,
            disk_id: Some("shared".into()),
            bytes: Some(vec![0; ADF_BYTES]),
        },
    );
    assert!(ambiguous
        .value
        .unwrap_err()
        .contains("Several ejected copies"));
    assert_eq!(guest_read_adf_payload(&mut owner, 2), 0x11111111);

    for (drive, words) in [(0, WRITE_WORDS), (1, other_words)] {
        mount(&mut owner, drive, Some("shared"), None);
        assert_eq!(guest_read_marker(&mut owner, drive), words[2..]);
    }
}

#[test]
fn quick_load_restores_mounted_contents_and_keeps_other_disks_from_the_session() {
    let mut owner = owner_with_latched_motors();
    mount(&mut owner, 0, Some("disk-b"), Some(0x11));
    execute(&mut owner, HostCommand::SaveQuickSlot(1));
    mount(&mut owner, 0, None, None);
    mount(&mut owner, 1, Some("disk-b"), None);
    guest_write(&mut owner, 1, WRITE_WORDS);
    mount(&mut owner, 1, None, None);

    mount(&mut owner, 0, Some("disk-a"), Some(0));
    guest_write(&mut owner, 0, WRITE_WORDS);
    mount(&mut owner, 0, None, None);
    mount(&mut owner, 0, Some("disk-c"), Some(0x22));
    let ports = owner.status().ports;
    assert!(
        request(&mut owner, HostCommand::LoadQuickSlot { slot: 5, ports })
            .value
            .is_err()
    );

    let HostValue::PlayerRestored(association) =
        execute(&mut owner, HostCommand::LoadQuickSlot { slot: 1, ports })
    else {
        panic!("Missing restored disk association");
    };
    assert_eq!(association.mounted[0].as_deref(), Some("disk-b"));
    assert_eq!(guest_read_adf_payload(&mut owner, 0), 0x11111111);
    assert_eq!(guest_read_marker(&mut owner, 0), [0; 6]);
    assert!(
        request(
            &mut owner,
            HostCommand::MountDisk {
                name: None,
                drive: 1,
                disk_id: Some("disk-b".into()),
                bytes: None,
            }
        )
        .value
        .is_err(),
        "later copy of B must not shadow the slot copy"
    );

    mount(&mut owner, 1, Some("disk-a"), None);
    mount(&mut owner, 2, Some("disk-c"), None);
    assert_eq!(guest_read_marker(&mut owner, 1), WRITE_WORDS[2..]);
    assert_eq!(guest_read_adf_payload(&mut owner, 2), 0x22222222);
}

#[test]
fn quick_slot_from_before_a_file_restore_reopens_its_own_disk_session() {
    let mut owner = owner_with_latched_motors();
    mount(&mut owner, 0, Some("older"), Some(0x11));
    execute(&mut owner, HostCommand::SaveQuickSlot(1));
    execute(&mut owner, HostCommand::SaveQuickSlot(2));

    let mut replacement = DebuggerSession::new();
    replacement
        .machine
        .floppy
        .set_connections([true, true, true, false])
        .unwrap();
    debugger::launch::mount(&mut replacement, 0, Some(&vec![0x22; ADF_BYTES])).unwrap();
    let mut saved =
        debugger::SaveState::capture(&replacement.machine, replacement.file_sources()).unwrap();
    saved.header.association.mounted[0] = Some("imported".into());
    let ports = owner.status().ports;
    execute(
        &mut owner,
        HostCommand::RestorePlayer {
            media: gui::host_commands::RestoreMedia {
                disks: vec![(
                    "imported".into(),
                    replacement.file_sources().disks[0].clone().unwrap(),
                )],
                ..Default::default()
            },
            json: saved.to_json().unwrap(),
            game_id: None,
            ports,
            resume: false,
        },
    );
    mount(&mut owner, 0, None, None);
    assert!(owner.status().quick_slots[0]);
    let HostValue::PlayerRestored(association) =
        execute(&mut owner, HostCommand::LoadQuickSlot { slot: 1, ports })
    else {
        panic!("Missing prior session's association");
    };
    assert_eq!(association.mounted[0].as_deref(), Some("older"));
    assert!(owner.status().drives[0].inserted);
    assert!(
        request(
            &mut owner,
            HostCommand::MountDisk {
                name: None,
                drive: 1,
                disk_id: Some("imported".into()),
                bytes: None,
            }
        )
        .value
        .is_err(),
        "unrelated file import's disk must not come along"
    );

    mount(&mut owner, 1, Some("new-in-reopened-session"), Some(0x33));
    mount(&mut owner, 1, None, None);
    execute(&mut owner, HostCommand::LoadQuickSlot { slot: 2, ports });
    mount(&mut owner, 1, Some("new-in-reopened-session"), None);
    assert!(owner.status().drives[1].inserted);
}

#[test]
fn four_owner_mounted_disks_supply_distinct_guest_dma_data_and_resume_in_flight() {
    let sources = debugger::MediaSources {
        rom: Some(debugger::SourceImage {
            name: None,
            bytes: rom(),
        }),
        disks: std::array::from_fn(|i| {
            Some(debugger::SourceImage {
                name: None,
                bytes: vec![((i + 1) * 0x11) as u8; ADF_BYTES],
            })
        }),
    };
    let setup = MachineSetup {
        config: config::A500Config::default(),
        ports: [DeviceKind::Mouse, DeviceKind::Joystick],
        drives: [true; 4],
        rom: sources.rom.as_ref().unwrap().bytes.clone(),
        disks: sources
            .disks
            .each_ref()
            .map(|disk| disk.as_ref().map(|image| image.bytes.clone())),
    };
    let mut owner = MachineOwner::new(Box::new(DebuggerSession::new()));
    execute(
        &mut owner,
        HostCommand::Launch {
            rom_name: None,
            disk_names: [None, None, None, None],
            setup,
            association: debugger::SaveAssociation::default(),
        },
    );
    assert!(owner
        .status()
        .drives
        .iter()
        .all(|drive| drive.connected && drive.inserted));
    let mounted_state = captured_state(&mut owner);
    // The owner exports the exact mounted state through its production codec.
    let HostValue::Snapshot(json) = execute(&mut owner, HostCommand::SaveSnapshot) else {
        panic!("Missing snapshot");
    };
    let mut session = DebuggerSession::new();
    session
        .load_state_from_json_with_sources(&json, sources.clone())
        .unwrap();
    assert_eq!(session.machine.save_state(), *mounted_state);
    assert!(session
        .machine
        .floppy
        .drives
        .iter()
        .all(|drive| drive.disk_change_flip_flop));
    cia_write(&mut session, 0xbfd100, 0xff);
    cia_write(&mut session, 0xbfd300, 0xff);
    for drive in 0..4 {
        let select = 1 << (drive + 3);
        cia_write(&mut session, 0xbfd100, 0x7f);
        cia_write(&mut session, 0xbfd100, 0x7f & !select);
        session.machine.step_cycles(3); // CIA falling-edge capture and connector delivery.
        assert!(session.machine.floppy.drives[drive].selected);
        cia_write(&mut session, 0xbfd100, 0x7f & !select & !1);
        cia_write(&mut session, 0xbfd100, 0x7f & !select);
        session.machine.step_cycles(3);
        assert!(!session.machine.floppy.drives[drive].disk_change_flip_flop);
        prepare_steady_read(&mut session, drive);
        assert!(session.machine.memory_bus().write_custom_word(
            config::custom_reg::agnus::REGISTER_DSKPTH,
            0,
            "DMA"
        ));
        assert!(session.machine.memory_bus().write_custom_word(
            config::custom_reg::agnus::REGISTER_DSKPTL,
            0x2000,
            "DMA"
        ));
        assert!(session.machine.paula.set_dsksync(0x4489));
        assert!(session.machine.paula.set_adkcon(0x8500));
        assert!(session.machine.memory_bus().write_custom_word(
            config::custom_reg::agnus::REGISTER_DMACON,
            0x8210,
            "DMA"
        ));
        assert!(session.machine.paula.set_dsklen(0x8000 | 540));
        session.machine.step_cycles(3);
        assert!(session.machine.paula.set_dsklen(0x8000 | 540));
        session.machine.step_cycles(3);
        if drive == 0 {
            while session.machine.agnus.dskpt < 0x2040 {
                session.machine.step_cck();
            }
            // Advance into a partially received word before the file capture.
            while serde_json::to_value(&session.machine.paula.disk).unwrap()["word_bits"] == 0 {
                session.machine.step_cck();
            }
            let saved = session.save_state_to_json().unwrap();
            let document: serde_json::Value = serde_json::from_str(&saved).unwrap();
            assert_ne!(
                document["state"]["floppy"]["drives"][drive]["cell_phase"],
                0
            );
            assert!(!session.machine.floppy.drives[drive].disk_change_flip_flop);
            let mut restored = DebuggerSession::new();
            restored
                .load_state_from_json_with_sources(&saved, sources.clone())
                .unwrap();
            assert_eq!(session.machine.save_state(), restored.machine.save_state());
            session.machine.step_cck();
            restored.machine.step_cck();
            assert_eq!(
                session.machine.floppy.pins.read_data_high,
                restored.machine.floppy.pins.read_data_high
            );
            assert_eq!(finish(&mut session), finish(&mut restored));
            assert_eq!(session.machine.agnus.dskpt, 0x2000 + 540 * 2);
            assert_eq!(session.machine.paula.dsklen & 0x3fff, 0);
            assert_ne!(session.machine.paula.intreqr_debug() & 0x02, 0);
            assert_eq!(session.machine.save_state(), restored.machine.save_state());
        } else {
            finish(&mut session);
        }
        let long = |address| {
            (u32::from(
                session
                    .machine
                    .physical_memory
                    .read_chip_word_debug(address),
            ) << 16)
                | u32::from(
                    session
                        .machine
                        .physical_memory
                        .read_chip_word_debug(address + 2),
                )
        };
        // WORDSYNC starts after the first sync word, so the second one is transferred.
        assert_eq!(
            session.machine.physical_memory.read_chip_word_debug(0x2000),
            0x4489
        );
        let decoded = ((long(0x203a) & 0x55555555) << 1) | (long(0x223a) & 0x55555555);
        assert_eq!(
            decoded,
            u32::from_be_bytes([((drive + 1) * 0x11) as u8; 4]),
            "DF{drive} data selection"
        );
        cia_write(&mut session, 0xbfd100, 0xff);
    }
}

#[test]
fn rejected_player_restore_preserves_machine_generation_and_game_association() {
    let mut owner = MachineOwner::new(Box::new(DebuggerSession::new()));
    execute(
        &mut owner,
        HostCommand::Launch {
            rom_name: None,
            disk_names: [None, None, None, None],
            setup: MachineSetup {
                config: config::A500Config::default(),
                ports: [DeviceKind::Mouse, DeviceKind::Joystick],
                drives: [true, false, false, false],
                rom: rom(),
                disks: std::array::from_fn(|_| None),
            },
            association: debugger::SaveAssociation {
                game_id: Some("current".into()),
                mounted: [None, None, None, None],
            },
        },
    );
    execute(&mut owner, HostCommand::Reset);
    execute(
        &mut owner,
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value: 0x12345678,
        },
    );
    let HostValue::Snapshot(before) = execute(&mut owner, HostCommand::SaveSnapshot) else {
        panic!("snapshot");
    };
    let generation = owner.status().generation;
    for (game_id, ports, expected) in [(
        Some("foreign".into()),
        [DeviceKind::Mouse, DeviceKind::Joystick],
        "different game",
    )] {
        let reply = owner.execute(HostRequest {
            id: 2,
            generation,
            command: HostCommand::RestorePlayer {
                media: gui::host_commands::RestoreMedia::default(),
                json: before.clone(),
                game_id,
                ports,
                resume: true,
            },
        });
        assert!(reply.value.unwrap_err().contains(expected));
        assert_eq!(owner.status().generation, generation);
        let HostValue::Snapshot(after) = execute(&mut owner, HostCommand::SaveSnapshot) else {
            panic!("snapshot");
        };
        let before = debugger::SaveState::from_json(&before).unwrap();
        let mut after = debugger::SaveState::from_json(&after).unwrap();
        after.header.timestamp = before.header.timestamp;
        assert_eq!(after, before);
        assert!(!owner.status().running);
    }
}

#[test]
fn player_restore_accepts_changed_endpoints_and_preserves_saved_progress() {
    let mut session = DebuggerSession::new();
    session.machine.cpu.state.set_d_long(7, 0x12345678);
    session.machine.game_ports.apply_mouse_delta(0, 5, -3);
    session
        .machine
        .game_ports
        .set_mouse_buttons(0, true, true, false);
    session.machine.game_ports.set_joystick(1, [true; 6]);
    let mut saved = debugger::SaveState::capture(&session.machine, session.file_sources()).unwrap();
    saved.header.association.game_id = Some("current".into());
    let mut owner = MachineOwner::new(Box::new(DebuggerSession::new()));
    execute(
        &mut owner,
        HostCommand::RestorePlayer {
            media: gui::host_commands::RestoreMedia::default(),
            json: saved.to_json().unwrap(),
            game_id: Some("current".into()),
            ports: [DeviceKind::Joystick, DeviceKind::Disconnected],
            resume: false,
        },
    );
    assert_eq!(
        owner.status().ports,
        [DeviceKind::Joystick, DeviceKind::Disconnected]
    );
    let HostValue::Snapshot(json) = execute(&mut owner, HostCommand::SaveSnapshot) else {
        panic!("snapshot");
    };
    let restored = debugger::SaveState::from_json(&json).unwrap();
    assert_eq!(restored.header.association, saved.header.association);
    let mut expected = session.machine.save_state();
    let restored = restored
        .restore(&debugger::MediaSources::default())
        .unwrap();
    expected.game_ports.state.port1 = restored.game_ports.state.port1;
    expected.game_ports.state.port2 = restored.game_ports.state.port2;
    assert_eq!(*restored, expected);
}

#[test]
fn mounting_and_ejecting_while_selected_refreshes_only_transfer_eligibility() {
    let mut session = DebuggerSession::new();
    for (address, value) in [
        (0xbfd100, 0xff),
        (0xbfd300, 0xff),
        (0xbfd100, 0x7f),
        (0xbfd100, 0x77),
    ] {
        cia_write(&mut session, address, value);
    }
    session.machine.step_cycles(2);
    assert!(session.machine.floppy.drives[0].motor_on);
    let baseline = session.machine.paula.disk_activity.sequences;
    session.machine.paula.dskbytr = 0x8001;
    session.machine.paula.read_dskbytr();
    assert_eq!(session.machine.paula.disk_activity.sequences, baseline);
    debugger::launch::mount(&mut session, 0, Some(&vec![0; 901120])).unwrap();
    assert_eq!(session.machine.paula.disk_activity.sequences, baseline);
    session.machine.paula.dskbytr = 0x8001;
    session.machine.paula.read_dskbytr();
    assert_eq!(
        session.machine.paula.disk_activity.sequences[0],
        baseline[0] + 1
    );
    debugger::launch::mount(&mut session, 0, None).unwrap();
    session.machine.paula.dskbytr = 0x8001;
    session.machine.paula.read_dskbytr();
    assert_eq!(
        session.machine.paula.disk_activity.sequences[0],
        baseline[0] + 1
    );
    assert!(session.machine.floppy.drives[0].selected);
    assert!(session.machine.floppy.drives[0].motor_on);
}

#[cfg(not(target_arch = "wasm32"))]
mod named_start_state {
    use super::*;
    use egui::accesskit::Role;
    use egui_kittest::{kittest::Queryable, Harness};
    use gui::game_library::{persist_save, GameLibrary};
    use gui::player::PlayerScreen;
    use gui::{EmulatorApp, ViewMode};
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};

    #[test]
    fn promoted_quicksave_loads_guest_written_media_from_the_sessions_adf_buffer() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        let directory = root.join("game");
        let disks = directory.join("disks");
        fs::create_dir_all(&disks).unwrap();
        fs::write(root.join("kick.rom"), rom()).unwrap();
        let original = disks.join("written.adf");
        let original_adf = vec![0x11; ADF_BYTES];
        fs::write(&original, &original_adf).unwrap();
        fs::write(disks.join("startup.adf"), vec![0x22; ADF_BYTES]).unwrap();
        fs::write(
            directory.join("game.json"),
            serde_json::to_vec(&serde_json::json!({
                "id": "game", "title": "Disk game", "configuration": "standard",
                "disks": [
                    {"id": "startup", "filename": "startup.adf"},
                    {"id": "written", "filename": "written.adf"}
                ],
                "mounted": ["startup", null, null, null]
            }))
            .unwrap(),
        )
        .unwrap();
        let game = GameLibrary::discover(root).unwrap().games.remove(0);

        let mut owner = owner_with_latched_motors();
        mount(&mut owner, 0, Some("written"), Some(0x11));
        guest_write(&mut owner, 0, WRITE_WORDS);
        let mut saved = saved_machine(&mut owner);
        saved.document.header.association.game_id = Some("game".into());
        let json = saved.document.to_json().unwrap();
        persist_save(&game, &json, Some(1)).unwrap();

        let mut app = player_app(root);
        app.start_library_game(game, None, false);
        crate::common::command(&mut app, HostCommand::Pause);
        assert_eq!(app.player.mounted[0].as_deref(), Some("startup"));
        let cached = app
            .player
            .disks
            .iter()
            .find(|disk| disk.id == "written")
            .unwrap();
        assert_eq!(cached.bytes.as_deref(), Some(original_adf.as_slice()));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 1000.0))
            .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
        crate::common::settle_harness(&mut h);
        click(&mut h, "Library");
        click(&mut h, "Disk game");
        click(&mut h, "Add to start states");
        h.get_by_role_and_label(Role::TextInput, "Start state name")
            .set_text_value_accesskit("Written media");
        crate::common::settle_harness(&mut h);
        click(&mut h, "Add start state");
        assert_eq!(fs::read(&original).unwrap(), original_adf);
        fs::remove_file(original).unwrap();

        let generation = h.state().host.status.generation;
        click(&mut h, "Load Written media");
        click(&mut h, "Replace session");
        crate::common::wait_harness(&mut h, |app| {
            app.host.status.generation > generation && app.host.status.cck > saved.state.cck
        });
        assert!(h.state().host.status.running);
        assert_eq!(h.state().player.mounted[0].as_deref(), Some("written"));
        click(&mut h, "Pause");
        crate::common::wait_harness(&mut h, |app| !app.host.status.running);
        let restored = crate::common::snapshot(h.state_mut());
        assert_eq!(raw_tracks(&restored, 0), raw_tracks(&saved.state, 0));
        let mut probe = Box::new(DebuggerSession::new());
        probe.load_state(&restored);
        let mut probe = MachineOwner::new(probe);
        assert_eq!(guest_read_marker(&mut probe, 0), WRITE_WORDS[2..]);

        let generation = h.state().host.status.generation;
        click(&mut h, "Reset");
        crate::common::wait_harness(&mut h, |app| app.host.status.generation > generation);
        assert_eq!(h.state().player.mounted[0].as_deref(), Some("startup"));
        assert!(h.state().host.status.running);
    }

    fn player_app(root: &Path) -> EmulatorApp {
        let mut app = EmulatorApp::default();
        app.set_view_mode(ViewMode::ScreenOnly);
        app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
        app.player.catalog.configurations[0].launch.drives = [true, true, true, false];
        let mut contents = serde_json::to_value(&app.player.catalog).unwrap();
        contents.as_object_mut().unwrap().remove("selected");
        fs::write(root.join("configurations.json"), contents.to_string()).unwrap();
        app.player.library_root = Some(root.to_path_buf());
        app.player.library = GameLibrary::discover(root).unwrap();
        app.player.screen = PlayerScreen::Library;
        app
    }

    fn click(h: &mut Harness<'_, EmulatorApp>, label: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while h.query_by_label(label).is_none() {
            assert!(Instant::now() < deadline, "Missing control {label}");
            h.step();
            std::thread::sleep(Duration::from_millis(1));
        }
        h.get_by_label(label).click_accesskit();
        crate::common::settle_harness(h);
    }
}
