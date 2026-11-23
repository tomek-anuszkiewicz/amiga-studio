#![allow(clippy::unwrap_used, clippy::panic)]
use debugger::{
    launch::{DeviceKind, MachineSetup},
    DebuggerSession,
};
use gui::host_commands::{HostCommand, HostReply, HostRequest, HostValue};
use gui::host_owner::MachineOwner;
use gui::host_video::VideoExchange;
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
        HostCommand::LoadBinary {
            address: 0x10000,
            bytes: program,
            prime: true,
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
    assert!(replacement.machine.floppy.drives[0].insert_disk(&vec![0x22; ADF_BYTES]));
    let mut saved = debugger::SaveState::capture(&replacement.machine);
    saved.header.association.mounted[0] = Some("imported".into());
    let ports = owner.status().ports;
    execute(
        &mut owner,
        HostCommand::RestorePlayer {
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
    let setup = MachineSetup {
        config: config::A500Config::default(),
        ports: [DeviceKind::Mouse, DeviceKind::Joystick],
        drives: [true; 4],
        rom: rom(),
        disks: std::array::from_fn(|i| Some(vec![((i + 1) * 0x11) as u8; 901120])),
    };
    let mut owner = MachineOwner::new(Box::new(DebuggerSession::new()));
    execute(
        &mut owner,
        HostCommand::Launch {
            setup,
            association: debugger::SaveAssociation::default(),
        },
    );
    assert!(owner
        .status()
        .drives
        .iter()
        .all(|drive| drive.connected && drive.inserted));
    // The production owner snapshot hands its exact machine value to the
    // integration fixture; no drive/media objects are substituted after mounting.
    let HostValue::Snapshot(json) = execute(&mut owner, HostCommand::SaveSnapshot) else {
        panic!("Missing snapshot");
    };
    let mut session = DebuggerSession::new();
    session.load_state_from_json(&json).unwrap();
    cia_write(&mut session, 0xbfd100, 0xff);
    cia_write(&mut session, 0xbfd300, 0xff);
    for drive in 0..4 {
        let select = 1 << (drive + 3);
        cia_write(&mut session, 0xbfd100, 0x7f);
        cia_write(&mut session, 0xbfd100, 0x7f & !select);
        session.machine.step_cycles(3); // CIA falling-edge capture and connector delivery.
        assert!(session.machine.floppy.drives[drive].selected);
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
            let saved = session.save_state_to_json().unwrap();
            let mut restored = DebuggerSession::new();
            restored.load_state_from_json(&saved).unwrap();
            assert_eq!(finish(&mut session), finish(&mut restored));
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
        let after = debugger::SaveState::from_json(&after).unwrap();
        assert_eq!(after.state, before.state);
        assert_eq!(after.header.association, before.header.association);
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
    let mut saved = debugger::SaveState::capture(&session.machine);
    saved.header.association.game_id = Some("current".into());
    let mut owner = MachineOwner::new(Box::new(DebuggerSession::new()));
    execute(
        &mut owner,
        HostCommand::RestorePlayer {
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
    saved.state.game_ports.state.port1 = restored.state.game_ports.state.port1;
    saved.state.game_ports.state.port2 = restored.state.game_ports.state.port2;
    assert_eq!(restored.state, saved.state);
    assert_eq!(restored.header.association, saved.header.association);
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
