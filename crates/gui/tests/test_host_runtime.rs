#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use common::TestContext as _;

use gui::host_commands::{HistoryCommand, HostCommand, HostObservation, HostReply, HostValue};
use gui::host_owner::MachineOwner;
use gui::host_runtime::HostRuntime;
use gui::host_video::VideoRead;
use gui::{DebuggerSession, EditRegister};
use std::cell::Cell;
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

thread_local! {
    static INVARIANT_READ_STARTED: Cell<bool> = const { Cell::new(false) };
}

// Inject through the existing bank dispatch rather than adding a production
// crash command or corrupting machine state. This is a forced host-boundary
// failure, not evidence of a naturally occurring memory invariant violation.
fn invariant_read(
    memory: &mut physical_memory::PhysicalMemory,
    address: u32,
) -> physical_memory::BusResult<u16> {
    use config::chip_ram::ChipRamAccess;
    INVARIANT_READ_STARTED.set(true);
    memory.read_chip_word(address);
    physical_memory::BusResult::Ready(memory.read_chip_word(address))
}

#[test]
fn fatal_owner_failure_wakes_gui_rejects_commands_and_requires_no_shutdown_reply() {
    use gui::host_runtime::HostRuntimeError;
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTUP_REPAINT: AtomicBool = AtomicBool::new(false);
    static FATAL_REPAINT: AtomicBool = AtomicBool::new(false);

    for command in [
        HostCommand::Run,
        HostCommand::StepInstruction,
        HostCommand::StepCck,
        HostCommand::StepField,
    ] {
        let single_cck = matches!(command, HostCommand::StepCck);
        let mut session = session();
        session.machine.physical_memory.bank_map[0].read_word = invariant_read;
        let context = egui::Context::default();
        let gui_thread = std::thread::current().id();
        STARTUP_REPAINT.store(false, Ordering::Release);
        FATAL_REPAINT.store(false, Ordering::Release);
        context.set_request_repaint_callback(move |info| {
            if info.delay.is_zero() && std::thread::current().id() != gui_thread {
                // The callback runs on the requesting thread. The failing
                // owner signals after reaching the injected memory access.
                if INVARIANT_READ_STARTED.get() {
                    FATAL_REPAINT.store(true, Ordering::Release);
                } else {
                    STARTUP_REPAINT.store(true, Ordering::Release);
                }
            }
        });
        let mut host = HostRuntime::start(MachineOwner::new(session), context.clone()).unwrap();
        let startup_deadline = Instant::now() + Duration::from_secs(10);
        while !STARTUP_REPAINT.load(Ordering::Acquire) {
            assert!(
                Instant::now() < startup_deadline,
                "Startup did not wake the GUI"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            host.try_consume(|_, _, _| ()).unwrap(),
            VideoRead::Frame(())
        );
        // Clear pending repaints without drawing, since the first draw can
        // request another repaint to synchronize the native window theme.
        for _ in 0..2 {
            let _ = context.run_logic(&egui::RawInput::default(), |_| {});
        }
        assert!(!context.has_requested_repaint());
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut pending = command;
        while let Err(busy) = host.submit(pending) {
            pending = busy.0;
            assert!(Instant::now() < deadline, "Command could not be submitted");
            std::thread::sleep(Duration::from_millis(1));
        }
        // Fill the command path before the worker runs. Even a single-CCK
        // command must eventually reach the injected normal CPU read.
        if single_cck {
            for _ in 0..16 {
                let _ = host.submit(HostCommand::StepCck);
            }
        }
        while !FATAL_REPAINT.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline, "Failure did not wake the GUI");
            std::thread::sleep(Duration::from_millis(1));
        }
        let error = host.poll().unwrap_err();
        let HostRuntimeError::Fatal(message) = error else {
            panic!("Lost fatal diagnostic: {error:?}");
        };
        assert!(
            message.starts_with("Chip RAM access invariant:"),
            "{message}"
        );
        let failure = HostRuntimeError::Fatal(message);
        assert!(!host.status().running);
        let cck = host.status().cck;
        for command in [HostCommand::Run, HostCommand::Reset, HostCommand::Shutdown] {
            assert!(host.submit(command).is_err());
            assert_eq!(host.poll().unwrap_err(), failure);
            assert_eq!(host.status().cck, cck);
            assert_eq!(host.status().field_number, 0);
        }
        assert_eq!(host.try_consume(|_, _, _| ()).unwrap(), VideoRead::Pending);
        // The failed worker has no Shutdown acknowledgment. Completion must
        // return the original failure immediately, and Drop must join it.
        assert_eq!(host.finish_shutdown().unwrap_err(), failure);
        drop(host);
        assert!(
            Instant::now() < deadline,
            "Shutdown waited for a dead owner"
        );
    }
}

#[test]
fn shutdown_completion_publishes_final_status_without_a_final_poll() {
    let mut host = start_host();
    host.submit(HostCommand::Shutdown).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if host.finish_shutdown().unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(host.status().shutdown);
    assert!(!host.status().running);
}

#[test]
fn actual_queue_overflow_reaches_the_owner_boundary_for_run_and_manual_steps() {
    use gui::host_runtime::HostRuntimeError;
    for command in [
        HostCommand::Run,
        HostCommand::StepInstruction,
        HostCommand::StepCck,
        HostCommand::StepField,
    ] {
        let mut session = session();
        for index in 0..32 {
            session.machine.paula.mutations.stage(
                config::custom_reg::paula::REGISTER_AUD0PER,
                index,
                200,
                config::MutationMode::Pipeline,
            );
        }
        session.machine.agnus.pending_dma_write =
            Some((config::custom_reg::paula::REGISTER_AUD1DAT, 0x4567));
        let mut host =
            HostRuntime::start(MachineOwner::new(session), egui::Context::default()).unwrap();
        wait_reply(&mut host, HostCommand::ReadCpu);
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut pending = command;
        loop {
            match host.submit(pending) {
                Ok(_) => break,
                Err(busy) => pending = busy.0,
            }
            host.poll().unwrap();
            assert!(Instant::now() < deadline, "Fatal trigger was not submitted");
            std::thread::yield_now();
        }
        let failure = loop {
            match host.poll() {
                Err(HostRuntimeError::Fatal(message)) => break message,
                Err(error) => panic!("Lost queue failure: {error:?}"),
                Ok(_) => {}
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert!(
            failure.contains("queue=Paula, CCK=Some(1), producer=DMA"),
            "{failure}"
        );
        assert_eq!(host.status().field_number, 0);
        assert!(host.submit(HostCommand::Run).is_err());
        assert!(matches!(
            host.finish_shutdown(),
            Err(HostRuntimeError::Fatal(_))
        ));
    }
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
    assert_eq!(state.memory.read_chip_byte_debug((0x2000) as u32), 0xab);
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
    let reset = wait_reply(&mut host, HostCommand::Reset);
    assert!(reset.value.is_ok());
    assert!(!reset.status.running);
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
fn obsolete_generation_rejects_memory_writes_without_advancing_the_machine() {
    use gui::host_commands::HostRequest;

    let mut owner = MachineOwner::new(session());
    let original_generation = owner.status().generation;
    let reset = owner.execute(HostRequest {
        id: 1,
        generation: original_generation,
        command: HostCommand::ResetWarm,
    });
    assert!(reset.value.is_ok());
    assert_eq!(reset.status.generation, original_generation + 1);
    assert!(owner
        .execute(HostRequest {
            id: 2,
            generation: reset.status.generation,
            command: HostCommand::WriteMemory {
                address: 0xfc2000,
                bytes: vec![0xab],
                restart_instruction: false,
            },
        })
        .value
        .is_ok());
    let before = owner.status();
    let rejected = owner.execute(HostRequest {
        id: 3,
        generation: original_generation,
        command: HostCommand::WriteMemory {
            address: 0xfc2000,
            bytes: vec![0xcd],
            restart_instruction: false,
        },
    });
    assert_eq!(rejected.id, 3);
    assert_eq!(
        rejected.value.unwrap_err(),
        "Command belongs to an obsolete runtime generation"
    );
    assert_eq!(rejected.status.generation, before.generation);
    assert_eq!(rejected.status.cck, before.cck);
    let read = owner.execute(HostRequest {
        id: 4,
        generation: reset.status.generation,
        command: HostCommand::ReadMemory {
            address: 0xfc2000,
            length: 1,
        },
    });
    let Ok(HostValue::Memory { address, bytes }) = read.value else {
        panic!("Missing retained memory");
    };
    assert_eq!(address, 0xfc2000);
    assert_eq!(bytes, [0xab]);
}

#[test]
fn targeted_owner_reads_preserve_machine_state_and_reject_obsolete_generations() {
    use gui::host_commands::HostRequest;

    let address = 0xc00100;
    let mut source = session();
    source
        .machine
        .physical_memory
        .write_word_debug(address, 0xcafe);
    source.machine.cpu.state.set_d_long(0, 0x1234_5678);
    source.machine.keyboard.key_down(0x20);
    source.machine.step_cycles(3);
    let exact = source.machine.save_state();
    let mut owner = MachineOwner::new(source);
    let generation = owner.status().generation;
    let request = |id, generation, command| HostRequest {
        id,
        generation,
        command,
    };
    let execution = || {
        HostCommand::Observe(HostObservation::Execution {
            data_register: 0,
            word_address: Some(address),
        })
    };
    let input = || HostCommand::Observe(HostObservation::Input { key: Some(0x20) });
    let run = owner.execute(request(1, generation, HostCommand::Run));
    assert!(run.value.is_ok() && run.status.running);
    let live = owner.execute(request(2, generation, execution()));
    assert_eq!(live.id, 2);
    assert_eq!(live.status, run.status);
    assert!(matches!(
        live.value,
        Ok(HostValue::Execution {
            data_register: 0x1234_5678,
            memory_word: Some(0xcafe),
        })
    ));
    let keys = owner.execute(request(3, generation, input()));
    assert_eq!(keys.id, 3);
    assert_eq!(keys.status, run.status);
    let Ok(HostValue::Input(keys)) = keys.value else {
        panic!("Missing held-key observation");
    };
    assert_eq!(keys.key_held, Some(true));
    assert!(owner
        .execute(request(4, generation, HostCommand::Inspect))
        .value
        .is_err());
    let saved = owner.execute(request(5, generation, HostCommand::SaveSnapshot));
    assert_eq!(saved.status, run.status);
    let Ok(HostValue::Snapshot(json)) = saved.value else {
        panic!("Missing unchanged machine state");
    };
    assert_eq!(*debugger::SaveState::from_json(&json).unwrap().state, exact);

    let reset = owner.execute(request(6, generation, HostCommand::Reset));
    assert!(reset.value.is_ok());
    assert_eq!(reset.status.generation, generation + 1);
    assert_eq!(reset.status.cck, 0);
    let obsolete = owner.execute(request(7, generation, execution()));
    assert_eq!(obsolete.id, 7);
    assert_eq!(obsolete.status, reset.status);
    assert_eq!(
        obsolete.value.unwrap_err(),
        "Command belongs to an obsolete runtime generation"
    );
    let fresh = owner.execute(request(8, reset.status.generation, execution()));
    assert_eq!(fresh.id, 8);
    assert_eq!(fresh.status, reset.status);
    assert!(matches!(
        fresh.value,
        Ok(HostValue::Execution {
            data_register: 0,
            memory_word: Some(0),
        })
    ));
    let keys = owner.execute(request(9, reset.status.generation, input()));
    assert_eq!(keys.status, reset.status);
    let Ok(HostValue::Input(keys)) = keys.value else {
        panic!("Missing reset key state");
    };
    assert_eq!(keys.key_held, Some(false));

    let mut checkpoint = session();
    checkpoint.machine.step_cycles(11);
    let checkpoint_cck = checkpoint.machine.cck;
    checkpoint.machine.cpu.state.set_d_long(0, 0xa1b2_c3d4);
    checkpoint
        .machine
        .physical_memory
        .write_word_debug(address, 0xbeef);
    let restored = owner.execute(request(
        10,
        reset.status.generation,
        HostCommand::RestorePlayer {
            json: debugger::SaveState::capture(&checkpoint.machine)
                .to_json()
                .unwrap(),
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
            resume: true,
        },
    ));
    assert_eq!(restored.id, 10);
    assert!(matches!(restored.value, Ok(HostValue::PlayerRestored(_))));
    assert_eq!(restored.status.generation, reset.status.generation + 1);
    assert_eq!(restored.status.cck, checkpoint_cck);
    assert!(restored.status.running);

    let obsolete = owner.execute(request(11, reset.status.generation, execution()));
    assert_eq!(obsolete.id, 11);
    assert_eq!(obsolete.status, restored.status);
    assert_eq!(
        obsolete.value.unwrap_err(),
        "Command belongs to an obsolete runtime generation"
    );
    let fresh = owner.execute(request(12, restored.status.generation, execution()));
    assert_eq!(fresh.id, 12);
    assert_eq!(fresh.status, restored.status);
    assert!(matches!(
        fresh.value,
        Ok(HostValue::Execution {
            data_register: 0xa1b2_c3d4,
            memory_word: Some(0xbeef),
        })
    ));
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
fn queued_memory_write_is_visible_to_a_running_observation() {
    let address = 0xc00100;
    let mut host = start_host();
    let initial = host.status();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut ids = Vec::new();
    for mut command in [
        HostCommand::WriteMemory {
            address,
            bytes: vec![0x89, 0xab],
            restart_instruction: false,
        },
        HostCommand::Run,
        HostCommand::Observe(HostObservation::Execution {
            data_register: 0,
            word_address: Some(address),
        }),
    ] {
        loop {
            match host.submit(command) {
                Ok(id) => {
                    ids.push(id);
                    break;
                }
                Err(busy) => command = busy.0,
            }
            assert!(Instant::now() < deadline, "Queued command was not admitted");
            std::thread::yield_now();
        }
    }

    let mut replies = Vec::new();
    while replies.len() < ids.len() {
        replies.extend(host.poll().unwrap());
        assert!(
            Instant::now() < deadline,
            "Queued commands did not all reply"
        );
        std::thread::yield_now();
    }
    let [edit, run, observed] = replies.as_slice() else {
        panic!("Expected one reply per queued command: {replies:?}");
    };
    assert_eq!([edit.id, run.id, observed.id].as_slice(), ids.as_slice());
    assert!(matches!(edit.value, Ok(HostValue::Done)));
    assert!(matches!(run.value, Ok(HostValue::Done)));
    assert!(!edit.status.running);
    assert!(run.status.running && observed.status.running);
    assert_eq!(edit.status.generation, initial.generation);
    assert_eq!(run.status.generation, initial.generation);
    assert_eq!(observed.status.generation, initial.generation);
    assert!(observed.status.cck >= run.status.cck);
    assert!(matches!(
        observed.value,
        Ok(HostValue::Execution {
            memory_word: Some(0x89ab),
            ..
        })
    ));
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
    let before = debugger::SaveState::from_json(&before).unwrap();
    let mut after = debugger::SaveState::from_json(&after).unwrap();
    // Separate saves can cross a UTC second without advancing the machine.
    after.header.timestamp = before.header.timestamp;
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

fn wait_status(host: &mut HostRuntime, ready: impl Fn(gui::host_commands::HostStatus) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready(host.status()) {
        host.poll().unwrap();
        host.try_consume(|_, _, _| ()).unwrap();
        assert!(Instant::now() < deadline, "Input delivery did not progress");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn snapshot(host: &mut HostRuntime) -> debugger::SaveState {
    let Ok(HostValue::Snapshot(json)) = wait_reply(host, HostCommand::SaveSnapshot).value else {
        panic!("Missing machine snapshot");
    };
    debugger::SaveState::from_json(&json).unwrap()
}

fn input_contacts(host: &mut HostRuntime) -> gui::host_commands::InputObservation {
    let reply = wait_reply(
        host,
        HostCommand::Observe(HostObservation::Input { key: None }),
    );
    let Ok(HostValue::Input(input)) = reply.value else {
        panic!("Missing owner input observation: {:?}", reply.value);
    };
    input
}

#[test]
fn developer_image_click_admits_input_and_window_focus_loss_releases_it() {
    use gui::host_input::HostInputEvent;

    let mut app = gui::EmulatorApp::with_session(session());
    let context = egui::Context::default();
    common::settle(&mut app, &context);
    assert_eq!(app.view_mode, gui::ViewMode::Developer);
    assert!(common::command(&mut app, HostCommand::Run).value.is_ok());
    let draw = |app: &mut gui::EmulatorApp, focused, events| {
        let _ = context.run_test_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 1080.0),
                )),
                focused,
                events,
                ..Default::default()
            },
            |ui| app.update_ui(ui),
        );
    };
    draw(&mut app, true, vec![]);
    common::settle(&mut app, &context);
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let generation = app.host.status.generation;
    let press = || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !sender.try_host_event(
            HostInputEvent::MouseButtons {
                port: 0,
                left: true,
                right: true,
                middle: true,
            },
            generation,
        ) {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    };
    let sample = |app: &mut gui::EmulatorApp, focused| {
        let before = app.host.runtime.as_ref().unwrap().status().input_deliveries;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            // Honor repaint retries while preserving the simulated window focus.
            draw(app, focused, vec![]);
            // Direct owner polling must not consume GUI-owned command replies.
            if app.host.status.input_deliveries >= before + 2 && app.host.is_idle() {
                break;
            }
            assert!(Instant::now() < deadline, "Input delivery did not progress");
            std::thread::sleep(Duration::from_millis(1));
        }
        let input = common::observe_input(app, None);
        (input.ciaa_port_a_inputs & 0x40, input.potgor & 0x0500)
    };

    // Developer view keeps the mouse free until the image is clicked.
    press();
    assert_eq!(sample(&mut app, true), (0x40, 0x0500));
    let image_id = egui::Id::unique("amiga-screen-input");
    let center = context.read_response(image_id).unwrap().rect.center();
    for pressed in [true, false] {
        draw(
            &mut app,
            true,
            vec![
                egui::Event::PointerMoved(center),
                egui::Event::PointerButton {
                    pos: center,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    common::settle(&mut app, &context);
    assert!(app.player.captured);
    press();
    assert_eq!(sample(&mut app, true), (0, 0));

    // Losing application focus releases held controls and rejects fresh presses.
    draw(&mut app, false, vec![egui::Event::WindowFocused(false)]);
    assert_eq!(sample(&mut app, false), (0x40, 0x0500));
    press();
    assert_eq!(sample(&mut app, false), (0x40, 0x0500));
}

#[test]
fn runtime_input_capture_pause_reset_and_load_release_obsolete_controls() {
    use gui::host_input::HostInputEvent;
    let mut host = start_host();
    let sender = host.input_sender();
    let buttons = HostInputEvent::MouseButtons {
        port: 0,
        left: true,
        right: true,
        middle: true,
    };
    let send = |event, generation| {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !sender.try_host_event(event, generation) {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    };
    let capture = |enabled| {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !sender.set_enabled(true, enabled, false) {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    };
    assert!(wait_reply(&mut host, HostCommand::Run).value.is_ok());
    // Disabled capture must discard presses at runtime ingress.
    send(buttons, host.status().generation);
    let before = host.status().input_deliveries;
    wait_status(&mut host, |status| status.input_deliveries >= before + 2);
    assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x40, 0x40);
    capture(true);
    send(buttons, host.status().generation);
    let before = host.status().input_deliveries;
    wait_status(&mut host, |status| status.input_deliveries >= before + 2);
    assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x40, 0);
    let held = snapshot(&mut host);
    // Capturing while held is observational; loading must release this actual
    // saved contact state rather than rely on a neutral fixture.
    assert_eq!(held.state.game_ports.sample_ciaa_port_a_inputs() & 0x40, 0);
    assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x40, 0);
    capture(false);
    let before = host.status().input_deliveries;
    wait_status(&mut host, |status| status.input_deliveries >= before + 2);
    assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x40, 0x40);
    capture(true);
    send(buttons, host.status().generation);
    assert!(wait_reply(&mut host, HostCommand::Pause).value.is_ok());
    wait_status(&mut host, |status| !status.running);
    send(buttons, host.status().generation); // Paused input must not survive resume.
    assert!(wait_reply(&mut host, HostCommand::Run).value.is_ok());
    let before = host.status().input_deliveries;
    wait_status(&mut host, |status| status.input_deliveries >= before + 2);
    assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x40, 0x40);
    for command in [
        HostCommand::ResetWarm,
        HostCommand::RestorePlayer {
            json: held.to_json().unwrap(),
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
            resume: false,
        },
    ] {
        send(buttons, host.status().generation);
        let generation = host.status().generation;
        let reply = wait_reply(&mut host, command);
        assert!(reply.value.is_ok());
        assert_eq!(reply.status.generation, generation + 1);
        assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x40, 0x40);
        assert!(wait_reply(
            &mut host,
            HostCommand::WriteMemory {
                address: 0xc01000,
                bytes: vec![0x60, 0xfe],
                restart_instruction: false,
            }
        )
        .value
        .is_ok());
        assert!(wait_reply(
            &mut host,
            HostCommand::SetRegister {
                register: EditRegister::PC,
                value: 0xc01000,
            }
        )
        .value
        .is_ok());
        assert_eq!(host.status().generation, generation + 1);
        assert!(wait_reply(&mut host, HostCommand::Run).value.is_ok());
        let before = host.status().input_deliveries;
        wait_status(&mut host, |status| status.input_deliveries >= before + 2);
        send(buttons, generation); // Capture is active: only the generation check rejects this.
        let before = host.status().input_deliveries;
        wait_status(&mut host, |status| status.input_deliveries >= before + 2);
        assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x40, 0x40);
    }
}

#[test]
fn running_restore_and_keyboard_reset_invalidate_cached_controller_contacts() {
    use gui::host_input::HostInputEvent;
    use gui::player_config::{ConfigurationCatalog, InputSource, NamedConfiguration};
    for keyboard_reset in [true, false] {
        let mut session = session();
        let mut rom = vec![0; 256 * 1024];
        rom[..8].copy_from_slice(&[0, 8, 0, 0, 0, 0xf8, 0, 8]);
        rom[8..10].copy_from_slice(&[0x60, 0xfe]);
        session.load_rom(&rom).unwrap();
        let mut host =
            HostRuntime::start(MachineOwner::new(session), egui::Context::default()).unwrap();
        let neutral = (!keyboard_reset).then(|| snapshot(&mut host));
        let sender = host.input_sender();
        let mut configuration = NamedConfiguration::default();
        configuration.controls.ports[1].source = InputSource::Gamepad {
            identity: "pad".into(),
            mapping: "arrows".into(),
        };
        let retry = |operation: &mut dyn FnMut() -> bool| {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !operation() {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        };
        retry(&mut || {
            sender.configure(
                &configuration.controls,
                &ConfigurationCatalog::default().mappings,
            )
        });
        retry(&mut || sender.set_enabled(true, true, true));
        assert!(wait_reply(&mut host, HostCommand::Run).value.is_ok());
        let generation = host.status().generation;
        let sample = |generation, pressed| {
            retry(&mut || {
                sender.try_gamepads(
                    vec![gui::input_mapping::GamepadSample {
                        identity: "pad".into(),
                        name: "Test controller".into(),
                        axes: vec![0.0, 0.0],
                        buttons: vec![pressed; 16],
                    }],
                    generation,
                    true,
                )
            });
        };
        let delivered = |host: &mut HostRuntime| {
            let before = host.status().input_deliveries;
            wait_status(host, |status| status.input_deliveries >= before + 2);
        };
        delivered(&mut host);
        sample(generation, false);
        sample(generation, true);
        delivered(&mut host);
        assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x80, 0);
        if keyboard_reset {
            for code in [0x63, 0x66, 0x67] {
                retry(&mut || {
                    sender.try_host_event(
                        HostInputEvent::Key {
                            code,
                            pressed: true,
                        },
                        generation,
                    )
                });
            }
            wait_status(&mut host, |status| status.generation != generation);
        } else {
            let neutral = neutral.unwrap();
            let ports = debugger::launch::state_port_kinds(&neutral.state.game_ports);
            assert!(wait_reply(
                &mut host,
                HostCommand::RestorePlayer {
                    json: neutral.to_json().unwrap(),
                    game_id: None,
                    ports,
                    resume: true,
                }
            )
            .value
            .is_ok());
        }
        let generation = host.status().generation;
        assert!(host.status().running);
        for captured in [false, true, false] {
            retry(&mut || sender.set_enabled(true, captured, true));
            delivered(&mut host);
            assert_eq!(
                input_contacts(&mut host).ciaa_port_a_inputs & 0x80,
                0x80,
                "A capture change replayed contacts from the previous generation"
            );
        }
        sample(generation, true);
        delivered(&mut host);
        assert_eq!(
            input_contacts(&mut host).ciaa_port_a_inputs & 0x80,
            0x80,
            "A held control must be released before it can play after reset or restore"
        );
        sample(generation, false);
        sample(generation, true);
        delivered(&mut host);
        assert_eq!(input_contacts(&mut host).ciaa_port_a_inputs & 0x80, 0);
    }
}
