#![allow(clippy::unwrap_used, clippy::panic)]
use debugger::{
    launch::{DeviceKind, MachineSetup},
    DebuggerSession,
};
use gui::host_commands::{HostCommand, HostRequest, HostValue};
use gui::host_owner::MachineOwner;
use physical_memory::{AddressBus, BusResult};

fn rom() -> Vec<u8> {
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    rom
}
fn execute(owner: &mut MachineOwner, command: HostCommand) -> HostValue {
    owner
        .execute(HostRequest {
            id: 1,
            generation: owner.status().generation,
            command,
        })
        .value
        .unwrap()
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
        // HRM odd/even sector payload begins 56 bytes after the sync words.
        let decoded = ((long(0x2038) & 0x55555555) << 1) | (long(0x2238) & 0x55555555);
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
