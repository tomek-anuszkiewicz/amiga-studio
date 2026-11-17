#![allow(clippy::unwrap_used, clippy::panic)]

use debugger::launch::DeviceKind;
use debugger::{DebuggerSession, SaveState};
use gui::host_commands::{HostCommand, HostRequest, HostValue};
use gui::host_owner::MachineOwner;
use gui::host_video::VideoExchange;

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
fn capture(owner: &mut MachineOwner) -> SaveState {
    let HostValue::Snapshot(json) = execute(owner, HostCommand::SaveSnapshot) else {
        panic!("snapshot");
    };
    SaveState::from_json(&json).unwrap()
}

fn acknowledge(session: &mut DebuggerSession) {
    assert!(session.machine.cia_a.set_cra(0x40));
    session.machine.step_cycles(330);
    assert!(session.machine.cia_a.set_cra(0));
    session.machine.step_cycles(10);
}
fn receive(session: &mut DebuggerSession) -> u8 {
    for _ in 0..4000 {
        session.machine.step_cck();
        if session.machine.cia_a.icr_debug() & 8 != 0 {
            session.machine.cia_a.icr();
            let byte = session.machine.cia_a.sdr();
            session.machine.step_cycles(100);
            return byte;
        }
    }
    panic!("keyboard byte timed out");
}
fn keyboard_guest() -> Box<DebuggerSession> {
    let mut session = Box::new(DebuggerSession::new());
    session.machine.cpu.state.set_stopped(true);
    assert!(session.machine.cia_a.set_icr(0x88));
    session.machine.step_cycles(400);
    acknowledge(&mut session);
    for expected in [0x04, 0x02] {
        assert_eq!(receive(&mut session), expected);
        acknowledge(&mut session);
    }
    // Poll ICR, store SDR bytes through (A0)+, acknowledge via CRA, and repeat.
    // All reads and acknowledgments below are executed by the emulated CPU.
    let mut code = vec![
        0x20, 0x7c, 0x00, 0xc0, 0x10, 0x00, // MOVEA.L #$C01000,A0
        0x08, 0x39, 0x00, 0x03, 0x00, 0xbf, 0xed, 0x01, // BTST #3,$BFED01
        0x67, 0xf6, // BEQ poll
        0x10, 0xf9, 0x00, 0xbf, 0xec, 0x01, // MOVE.B $BFEC01,(A0)+
        0x13, 0xfc, 0x00, 0x40, 0x00, 0xbf, 0xee, 0x01, // MOVE.B #$40,$BFEE01
        0x30, 0x3c, 0x00, 0x64, // MOVE.W #100,D0
        0x51, 0xc8, 0xff, 0xfe, // DBRA D0,delay (more than 85 us)
        0x13, 0xfc, 0x00, 0x00, 0x00, 0xbf, 0xee, 0x01, // MOVE.B #0,$BFEE01
    ];
    let displacement = 6i16 - (code.len() as i16 + 2);
    code.extend([0x60, 0x00]);
    code.extend(displacement.to_be_bytes());
    session.load_binary(0xc00000, &code, true).unwrap();
    session.machine.cpu.state.set_stopped(false);
    session.set_debugger_enabled(false);
    session
}

#[test]
fn held_snapshot_load_delivers_keyup_to_a_guest_during_transfer_and_with_full_queue() {
    for full in [false, true] {
        let mut session = keyboard_guest();
        session.machine.keyboard.key_down(0x35);
        session.machine.step_cycles(700); // Partially shifted press, no complete byte yet.
        assert_eq!(session.machine.cia_a.icr_debug() & 8, 0);
        if full {
            for _ in 0..keyboard::KEYBOARD_BUFFER_CAPACITY {
                session.machine.keyboard.enqueue_scancode(0x20);
            }
        }
        session
            .machine
            .game_ports
            .set_mouse_buttons(0, true, true, false);
        session.machine.game_ports.set_joystick(1, [true; 6]);
        let exact = session.machine.save_state();
        let mut owner = MachineOwner::new(session);
        let generation = owner.status().generation;
        let held = capture(&mut owner);
        assert_eq!(
            *held.state, exact,
            "capture released controls or advanced state"
        );
        assert_eq!(owner.status().generation, generation);
        execute(
            &mut owner,
            HostCommand::RestorePlayer {
                json: held.to_json().unwrap(),
                game_id: None,
                ports: [
                    debugger::launch::DeviceKind::Mouse,
                    debugger::launch::DeviceKind::Joystick,
                ],
                resume: false,
            },
        );
        let loaded = capture(&mut owner);
        assert!(!loaded.state.game_ports.fire1_port1());
        assert!(!loaded.state.game_ports.fire1_port2());
        assert!(!loaded.state.game_ports.state.port2.is_fire2_pressed());
        assert_eq!(loaded.state.cck, held.state.cck);
        assert_eq!(loaded.state.cpu, held.state.cpu);
        let mut guest = DebuggerSession::new();
        guest.load_state(&loaded.state);
        guest.machine.step_cycles(80_000);
        let received = |offset: u32| {
            guest
                .machine
                .physical_memory
                .read_byte_debug(0xc01000 + offset)
        };
        assert_eq!(
            received(0),
            0x95,
            "guest did not receive original key press"
        );
        if full {
            for offset in 1..=10 {
                assert_eq!(received(offset), 0xbf);
            }
        }
        assert_eq!(
            received(if full { 11 } else { 1 }),
            0x94,
            "guest did not receive release"
        );
        guest.machine.keyboard.key_down(0x35);
        guest.machine.step_cycles(15_000);
        assert_eq!(
            guest
                .machine
                .physical_memory
                .read_byte_debug(if full { 0xc0100c } else { 0xc01002 }),
            0x95
        );
    }
}

#[test]
fn stopped_reattachment_preserves_phases_registers_media_and_rejects_old_events() {
    let mut session = keyboard_guest();
    session.machine.game_ports.apply_mouse_delta(0, 7, -4);
    session.machine.step_cycles(3);
    session
        .machine
        .game_ports
        .set_mouse_buttons(0, true, true, false);
    session.machine.game_ports.set_joystick(1, [true; 6]);
    assert!(session.machine.floppy.drives[0].insert_disk(&vec![0x33; 901120]));
    let mut expected = session.machine.save_state();
    expected.keyboard.release_keys();
    expected.game_ports.release_controls();
    let mut owner = MachineOwner::new(session);
    let generation = owner.status().generation;
    execute(&mut owner, HostCommand::SaveQuickSlot(1));
    execute(
        &mut owner,
        HostCommand::ReconfigurePorts([DeviceKind::Mouse, DeviceKind::Joystick]),
    );
    assert_eq!(*capture(&mut owner).state, expected);
    execute(
        &mut owner,
        HostCommand::ReconfigurePorts([DeviceKind::Mouse, DeviceKind::Disconnected]),
    );
    let changed = capture(&mut owner);
    expected.game_ports.state.port2 = changed.state.game_ports.state.port2;
    assert_eq!(*changed.state, expected);
    let stale = owner.execute(HostRequest {
        id: 3,
        generation,
        command: HostCommand::Run,
    });
    assert!(stale.value.unwrap_err().contains("obsolete"));
    execute(
        &mut owner,
        HostCommand::LoadQuickSlot {
            slot: 1,
            ports: [DeviceKind::Mouse, DeviceKind::Disconnected],
        },
    );
    assert_eq!(
        owner.status().ports,
        [DeviceKind::Mouse, DeviceKind::Disconnected]
    );
    assert_eq!(*capture(&mut owner).state, expected);
    let video = VideoExchange::new(vec![0; owner.video_storage_len()]);
    execute(&mut owner, HostCommand::Run);
    owner.publish(&video).unwrap();
    let before = capture(&mut owner);
    assert!(owner
        .execute(HostRequest {
            id: 4,
            generation: owner.status().generation,
            command: HostCommand::ReconfigurePorts([DeviceKind::Joystick; 2])
        })
        .value
        .is_err());
    assert_eq!(*capture(&mut owner).state, *before.state);
}

#[test]
fn a_foreign_game_quick_slot_cannot_replace_current_progress_or_association() {
    let session = DebuggerSession::new();
    let mut saved_a = SaveState::capture(&session.machine);
    saved_a.header.association.game_id = Some("game-a".into());
    let mut saved_b = saved_a.clone();
    saved_b.header.association.game_id = Some("game-b".into());
    saved_b.state.cpu.set_d_long(7, 0x12345678);
    let ports = [DeviceKind::Mouse, DeviceKind::Joystick];
    let mut owner = MachineOwner::new(Box::new(session));
    for (saved, game) in [(&saved_a, "game-a"), (&saved_b, "game-b")] {
        execute(
            &mut owner,
            HostCommand::RestorePlayer {
                json: saved.to_json().unwrap(),
                game_id: Some(game.into()),
                ports,
                resume: false,
            },
        );
        if game == "game-a" {
            execute(&mut owner, HostCommand::SaveQuickSlot(1));
        }
    }
    assert!(owner.status().quick_slots[0]);
    let before = capture(&mut owner);
    let generation = owner.status().generation;
    let rejected = owner.execute(HostRequest {
        id: 3,
        generation,
        command: HostCommand::LoadQuickSlot { slot: 1, ports },
    });
    assert!(rejected.value.unwrap_err().contains("different game"));
    assert_eq!(owner.status().generation, generation);
    assert!(!owner.status().running);
    let after = capture(&mut owner);
    assert_eq!(after.state, before.state);
    assert_eq!(after.header.association, before.header.association);
    assert_eq!(after.header.association.game_id.as_deref(), Some("game-b"));
}
