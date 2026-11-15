#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

use gui::host_commands::{HistoryCommand, HostCommand, HostReply, HostValue};
use gui::host_owner::MachineOwner;
use gui::host_runtime::HostRuntime;
use gui::host_video::VideoRead;
use gui::{DebuggerSession, EditRegister};
use std::time::{Duration, Instant};

fn wait_reply(host: &mut HostRuntime, command: HostCommand) -> HostReply {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut pending = command;
    let request = loop {
        match host.submit(pending) {
            Ok(request) => break request,
            Err(busy) => pending = busy.0,
        }
        assert!(Instant::now() < deadline, "Command could not be submitted");
        host.poll().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    };
    loop {
        assert!(Instant::now() < deadline, "Machine owner did not reply");
        for reply in host.poll().unwrap() {
            if reply.id == request {
                return reply;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn session() -> Box<DebuggerSession> {
    let mut session = Box::new(DebuggerSession::new());
    session
        .load_binary(0x1000, &[0x4e, 0x71, 0x60, 0xfc], true)
        .unwrap();
    session.set_debugger_enabled(false);
    session
}

fn start_host() -> HostRuntime {
    HostRuntime::start(MachineOwner::new(session()), egui::Context::default()).unwrap()
}

#[test]
fn owner_history_navigation_restores_cpu_and_clears_trace() {
    let mut session = session();
    session.set_debugger_enabled(true);
    session.step_instruction();
    assert!(!session.debugger.trace.entries().is_empty());
    session.temporal.clear();
    session.temporal.set_recording(true);
    for index in 0..5 {
        let mut cpu = session.machine.cpu.state.clone();
        cpu.set_d_long(0, index);
        let pc = 0x1000 + 2 * index;
        cpu.set_pc(pc);
        session.temporal.record(
            index as u64 * debugger::temporal::PAL_FRAME_CCK,
            pc,
            0x4e71,
            cpu,
        );
    }
    session.machine.cpu.state.set_d_long(0, 99);
    let mut host =
        HostRuntime::start(MachineOwner::new(session), egui::Context::default()).unwrap();
    for (command, cursor, value) in [
        (HistoryCommand::Index(1), Some(1), 1),
        (HistoryCommand::Forward, Some(2), 2),
        (HistoryCommand::BackwardField, Some(1), 1),
        (HistoryCommand::ForwardField, Some(2), 2),
        (
            HistoryCommand::Cck(3 * debugger::temporal::PAL_FRAME_CCK),
            Some(3),
            3,
        ),
        (HistoryCommand::Live, None, 99),
    ] {
        assert!(wait_reply(&mut host, HostCommand::History(command))
            .value
            .is_ok());
        let Ok(HostValue::Inspection(state)) = wait_reply(&mut host, HostCommand::Inspect).value
        else {
            panic!("Missing history inspection");
        };
        assert_eq!(state.temporal.scrub_cursor, cursor);
        assert_eq!(state.cpu.d_long(0), value);
    }
    assert!(wait_reply(&mut host, HostCommand::ClearTrace).value.is_ok());
    let Ok(HostValue::Inspection(state)) = wait_reply(&mut host, HostCommand::Inspect).value else {
        panic!("Missing trace inspection");
    };
    assert!(state.debugger.trace.entries().is_empty());
}

#[test]
fn stack_register_commands_and_warm_reset_preserve_native_owner_contracts() {
    let mut host = start_host();
    for (register, value) in [
        (EditRegister::USP, 0x11223344),
        (EditRegister::SSP, 0x55667788),
    ] {
        assert!(
            wait_reply(&mut host, HostCommand::SetRegister { register, value })
                .value
                .is_ok()
        );
    }
    let Ok(HostValue::Cpu(cpu)) = wait_reply(&mut host, HostCommand::ReadCpu).value else {
        panic!("Missing stack registers");
    };
    assert_eq!(cpu.usp(), 0x11223344);
    assert_eq!(cpu.ssp(), 0x55667788);
    assert!(wait_reply(
        &mut host,
        HostCommand::WriteMemory {
            address: 0x2000,
            bytes: vec![0xab],
            restart_instruction: false,
        }
    )
    .value
    .is_ok());
    let generation = host.status().generation;
    let reset = wait_reply(&mut host, HostCommand::ResetWarm);
    assert!(reset.value.is_ok());
    assert_eq!(reset.status.generation, generation + 1);
    assert!(!reset.status.running);
    let Ok(HostValue::Inspection(state)) = wait_reply(&mut host, HostCommand::Inspect).value else {
        panic!("Missing retained RAM");
    };
    // Warm reset re-enables the ROM overlay; inspect retained physical Chip RAM.
    assert_eq!(state.memory.chip_ram[0x2000], 0xab);
}

#[test]
fn unfinished_fields_continue_on_the_worker_and_still_service_controls() {
    let mut session = session();
    session
        .load_binary(
            0x1000,
            &[0x33, 0xfc, 0, 0, 0, 0xdf, 0xf0, 0x2c, 0x60, 0xf6],
            true,
        )
        .unwrap();
    let mut host =
        HostRuntime::start(MachineOwner::new(session), egui::Context::default()).unwrap();
    assert!(wait_reply(&mut host, HostCommand::Run).value.is_ok());
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.status().cck < 2 * debugger::session::MAX_FRAME_SLICE_CCK as u64 {
        assert!(
            Instant::now() < deadline,
            "Unfinished field stopped worker progress"
        );
        host.poll().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(host.status().field_number, 0);
    let paused = wait_reply(&mut host, HostCommand::Pause);
    assert!(paused.value.is_ok());
    assert!(paused.status.pause_pending);
    assert!(wait_reply(&mut host, HostCommand::Reset).value.is_ok());
    assert!(!host.status().running);
    assert!(wait_reply(&mut host, HostCommand::Shutdown).value.is_ok());
}

#[test]
fn debugger_writes_patch_rom_bytes_and_wrap_the_physical_address_space() {
    let mut host = start_host();
    for (address, bytes, restart_instruction) in [
        (0xfc0000, vec![0x70], false),
        (0xfc0000, vec![0x4e, 0x71], true),
        (0xfbffff, vec![0xaa, 0xbb], false),
        (0xffffff, vec![0xcc, 0xdd], false),
    ] {
        let write = wait_reply(
            &mut host,
            HostCommand::WriteMemory {
                address,
                bytes: bytes.clone(),
                restart_instruction,
            },
        );
        assert!(
            write.value.is_ok(),
            "Write at {address:06x}: {:?}",
            write.value
        );
        let read = wait_reply(
            &mut host,
            HostCommand::ReadMemory {
                address,
                length: bytes.len(),
            },
        );
        let Ok(HostValue::Memory { bytes: actual, .. }) = read.value else {
            panic!("Missing memory reply");
        };
        if address == 0xfbffff {
            assert_eq!(actual[1], bytes[1]);
        } else {
            assert_eq!(actual, bytes);
        }
    }
}

#[test]
fn full_video_output_still_services_speed_pause_inspection_and_shutdown() {
    let mut host = start_host();
    assert!(wait_reply(&mut host, HostCommand::Run).value.is_ok());
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.status().field_number == 0 {
        assert!(
            Instant::now() < deadline,
            "Machine did not complete a field"
        );
        host.poll().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    // Intentionally leave the initial published image unread.
    let paused = wait_reply(&mut host, HostCommand::Pause);
    assert!(!paused.status.running);
    let stopped_cck = paused.status.cck;
    assert!(stopped_cck > 0);
    let faster = wait_reply(&mut host, HostCommand::Faster);
    assert_eq!(faster.status.speed, 2.0);
    assert_eq!(faster.status.cck, stopped_cck);
    assert!(matches!(
        wait_reply(&mut host, HostCommand::ReadCpu).value,
        Ok(HostValue::Cpu(_))
    ));
    assert_eq!(host.status().cck, stopped_cck);
    let shutdown = wait_reply(&mut host, HostCommand::Shutdown);
    assert!(shutdown.status.shutdown);
    assert_eq!(shutdown.status.cck, stopped_cck);
    assert!(
        host.submit(HostCommand::Run).is_err(),
        "Shutdown owner accepted more work"
    );
    while host.finish_shutdown().unwrap().is_none() {
        assert!(Instant::now() < deadline, "Worker did not shut down");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn register_and_memory_commands_report_coherent_owned_results() {
    let mut host = start_host();
    let reply = wait_reply(
        &mut host,
        HostCommand::SetRegister {
            register: EditRegister::D(3),
            value: 0x1234_5678,
        },
    );
    assert!(reply.value.is_ok());
    let cpu = wait_reply(&mut host, HostCommand::ReadCpu);
    let Ok(HostValue::Cpu(cpu)) = cpu.value else {
        panic!("Missing CPU reply");
    };
    assert_eq!(cpu.d_long(3), 0x1234_5678);
    assert!(wait_reply(
        &mut host,
        HostCommand::WriteMemory {
            address: 0x2000,
            bytes: vec![0x12, 0x34, 0xab, 0xcd],
            restart_instruction: false,
        }
    )
    .value
    .is_ok());
    let memory = wait_reply(
        &mut host,
        HostCommand::ReadMemory {
            address: 0x2000,
            length: 4,
        },
    );
    let Ok(HostValue::Memory { address, bytes }) = memory.value else {
        panic!("Missing memory reply");
    };
    assert_eq!(address, 0x2000);
    assert_eq!(bytes, [0x12, 0x34, 0xab, 0xcd]);
    assert!(wait_reply(
        &mut host,
        HostCommand::SetRegister {
            register: EditRegister::D(8),
            value: 7
        }
    )
    .value
    .is_err());
    assert!(wait_reply(
        &mut host,
        HostCommand::ReadMemory {
            address: 0,
            length: 4097
        }
    )
    .value
    .is_err());
}

#[test]
fn reset_changes_generation_and_releases_old_output_without_displaying_it() {
    let mut host = start_host();
    let before = host.status().generation;
    // Wait for the initial image to be pending before invalidating its timeline.
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.status().publication == 0 {
        assert!(Instant::now() < deadline, "Initial image was not published");
        host.poll().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    let reset = wait_reply(&mut host, HostCommand::Reset);
    assert!(reset.value.is_ok());
    assert_eq!(reset.status.generation, before + 1);
    loop {
        let result = host
            .try_consume(|_, _, _| panic!("Old image reached the GUI"))
            .unwrap();
        if result != VideoRead::Pending {
            assert_eq!(result, VideoRead::Discarded);
            break;
        }
        assert!(Instant::now() < deadline, "Old image was not released");
        std::thread::yield_now();
    }
    loop {
        host.poll().unwrap();
        if let VideoRead::Frame(info) = host.try_consume(|info, _, _| info).unwrap() {
            assert_eq!(info.generation, reset.status.generation);
            assert_eq!(info.cck, 0);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "New generation image was not published"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn paused_step_and_invalid_load_preserve_owner_state() {
    let mut reference = session();
    reference.step_instruction();
    let mut host = start_host();
    let stepped = wait_reply(&mut host, HostCommand::StepInstruction);
    assert!(stepped.value.is_ok());
    assert!(!stepped.status.running);
    assert_eq!(stepped.status.cck, reference.machine.cck);
    let cpu = wait_reply(&mut host, HostCommand::ReadCpu);
    let Ok(HostValue::Cpu(cpu)) = cpu.value else {
        panic!("Missing CPU reply");
    };
    assert_eq!(cpu.instruction_pc(), 0x1002);
    assert_eq!(*cpu, reference.machine.cpu.state);
    let before = host.status();
    assert!(wait_reply(&mut host, HostCommand::LoadRom(vec![0; 1]))
        .value
        .is_err());
    assert_eq!(host.status().generation, before.generation);
    assert_eq!(host.status().cck, before.cck);
}

#[test]
fn stopped_inspection_preserves_history_navigation_without_copying_cpu_snapshots() {
    let mut reference = session();
    reference.set_debugger_enabled(true);
    reference.temporal.set_recording(true);
    let mut host = HostRuntime::start(
        MachineOwner::new(reference.clone()),
        egui::Context::default(),
    )
    .unwrap();
    for _ in 0..3 {
        reference.step_instruction();
        assert!(wait_reply(&mut host, HostCommand::StepInstruction)
            .value
            .is_ok());
    }
    let snapshot = wait_reply(&mut host, HostCommand::Inspect);
    let Ok(HostValue::Inspection(snapshot)) = snapshot.value else {
        panic!("Missing stopped inspector snapshot");
    };
    assert_eq!(snapshot.cpu, reference.machine.cpu.state);
    assert_eq!(snapshot.cck, reference.machine.cck);
    assert_eq!(snapshot.memory, reference.machine.physical_memory);
    assert_eq!(snapshot.temporal.len(), reference.temporal.len());
    assert_eq!(snapshot.temporal.capacity(), reference.temporal.capacity());
    assert!(snapshot.temporal.is_recording());
    for index in 0..reference.temporal.len() {
        let expected = reference.temporal.get_chronological(index).unwrap();
        let actual = snapshot.temporal.get_chronological(index).unwrap();
        assert_eq!((actual.pc, actual.cck), (expected.pc, expected.cck));
        assert_eq!(
            snapshot.temporal.find_matches_by_pc(expected.pc, 2),
            reference.temporal.find_matches_by_pc(expected.pc, 2)
        );
    }
    reference.scrub_to_frame(0);
    assert!(
        wait_reply(&mut host, HostCommand::History(HistoryCommand::Index(0)))
            .value
            .is_ok()
    );
    let restored = wait_reply(&mut host, HostCommand::ReadCpu);
    let Ok(HostValue::Cpu(restored)) = restored.value else {
        panic!("Missing historical CPU state");
    };
    assert_eq!(*restored, reference.machine.cpu.state);
    assert!(wait_reply(
        &mut host,
        HostCommand::WriteMemory {
            address: 0x2000,
            bytes: vec![0xaa],
            restart_instruction: false,
        }
    )
    .value
    .is_ok());
    assert_eq!(snapshot.memory.read_byte_debug(0x2000), 0);
    assert_eq!(snapshot.temporal.scrub_cursor, None);
    wait_reply(&mut host, HostCommand::Run);
    assert!(wait_reply(&mut host, HostCommand::Inspect).value.is_err());
}

#[test]
fn ordered_command_backpressure_releases_all_replies_without_losing_register_edits() {
    let mut host = start_host();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut submitted = 0;
    let mut replies = Vec::new();
    const COMMANDS: u32 = 192;
    while submitted < COMMANDS || replies.len() < COMMANDS as usize {
        if submitted < COMMANDS
            && host
                .submit(HostCommand::SetRegister {
                    register: EditRegister::D(3),
                    value: submitted,
                })
                .is_ok()
        {
            submitted += 1;
        }
        replies.extend(host.poll().unwrap());
        assert!(
            Instant::now() < deadline,
            "Bounded command/reply exchange stalled"
        );
        std::thread::yield_now();
    }
    for (index, reply) in replies.iter().enumerate() {
        assert_eq!(reply.id, index as u64 + 1);
        assert!(reply.value.is_ok());
    }
    let Ok(HostValue::Cpu(cpu)) = wait_reply(&mut host, HostCommand::ReadCpu).value else {
        panic!("Missing final register state");
    };
    assert_eq!(cpu.d_long(3), COMMANDS - 1);
}

#[test]
fn invalid_rom_does_not_mutate_any_running_machine_state_at_the_command_boundary() {
    use gui::host_commands::HostRequest;
    let mut owner = MachineOwner::new(session());
    let exchange = gui::host_video::VideoExchange::new(vec![0; owner.video_storage_len()]);
    owner.publish(&exchange).unwrap();
    owner.execute(HostRequest {
        id: 1,
        generation: 1,
        command: HostCommand::Run,
    });
    owner.advance(0.0, || 0.0);
    let before = owner.execute(HostRequest {
        id: 2,
        generation: 1,
        command: HostCommand::SaveSnapshot,
    });
    let invalid = owner.execute(HostRequest {
        id: 3,
        generation: 1,
        command: HostCommand::LoadRom(vec![0; 1024]),
    });
    assert!(invalid.value.is_err());
    assert!(invalid.status.running);
    assert_eq!(invalid.status.cck, before.status.cck);
    assert_eq!(invalid.status.generation, before.status.generation);
    let after = owner.execute(HostRequest {
        id: 4,
        generation: 1,
        command: HostCommand::SaveSnapshot,
    });
    let (Ok(HostValue::Snapshot(before)), Ok(HostValue::Snapshot(after))) =
        (before.value, after.value)
    else {
        panic!("Missing complete machine snapshots");
    };
    assert_eq!(
        before, after,
        "Rejected ROM load modified the running machine"
    );
}

#[test]
fn reset_reply_and_video_handoff_cannot_publish_an_obsolete_or_lose_a_new_image() {
    let mut host = start_host();
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.status().publication == 0 {
        assert!(Instant::now() < deadline);
        host.poll().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    let request = loop {
        if let Ok(id) = host.submit(HostCommand::Reset) {
            break id;
        }
        host.poll().unwrap();
        assert!(Instant::now() < deadline);
    };
    assert_eq!(
        host.try_consume(|_, _, _| ()).unwrap(),
        VideoRead::Pending,
        "An unacknowledged reset exposed the old image"
    );
    let mut acknowledged = false;
    loop {
        for reply in host.poll().unwrap() {
            if reply.id == request {
                acknowledged = true;
                assert!(reply.value.is_ok());
            }
        }
        if let VideoRead::Frame(info) = host.try_consume(|info, _, _| info).unwrap() {
            assert!(acknowledged);
            assert_eq!(info.generation, 2);
            assert_eq!(info.cck, 0);
            break;
        }
        assert!(Instant::now() < deadline, "Reset image was lost");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn binary_load_during_an_unfinished_running_field_keeps_the_previous_image() {
    use gui::host_commands::HostRequest;
    let mut session = session();
    // MOVE.W #0,$DFF02C; BRA.S $1000 continuously restarts the beam position.
    session
        .load_binary(
            0x1000,
            &[0x33, 0xfc, 0, 0, 0, 0xdf, 0xf0, 0x2c, 0x60, 0xf6],
            true,
        )
        .unwrap();
    let mut owner = MachineOwner::new(session);
    let exchange = gui::host_video::VideoExchange::new(vec![0; owner.video_storage_len()]);
    assert!(owner.publish(&exchange).unwrap());
    assert!(matches!(
        exchange.try_consume(1, |_, _| ()).unwrap(),
        VideoRead::Frame(())
    ));
    owner.execute(HostRequest {
        id: 1,
        generation: 1,
        command: HostCommand::Run,
    });
    owner.advance(0.0, || 0.0);
    assert!(owner.status().cck > 0);
    assert_eq!(owner.status().field_number, 0);
    let loaded = owner.execute(HostRequest {
        id: 2,
        generation: 1,
        command: HostCommand::LoadBinary {
            address: 0x2000,
            bytes: vec![0x4e, 0x71],
            prime: false,
        },
    });
    assert!(loaded.value.is_ok());
    assert!(loaded.status.running);
    assert_eq!(loaded.status.generation, 2);
    assert!(
        !owner.publish(&exchange).unwrap(),
        "Running load published an unfinished field"
    );
    assert_eq!(
        exchange.try_consume(2, |_, _| ()).unwrap(),
        VideoRead::Pending
    );
}
