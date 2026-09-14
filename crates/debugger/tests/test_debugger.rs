use debugger::{assemble_instruction, disassemble, Debugger};
use m68000::Cpu;
use physical_memory::MemoryBus;

#[test]
fn test_disassembler_primitives() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    // 1. NOP ($4E71)
    bus.write_word_debug(0x001000, 0x4E71);
    let (disasm, bytes) = disassemble(0x001000, |a| bus.read_word_debug(a));
    assert_eq!(disasm.mnemonic, "NOP");
    assert_eq!(bytes, 2);

    // 2. MOVE.W D0, D1 ($3200)
    bus.write_word_debug(0x001002, 0x3200);
    let (disasm, bytes) = disassemble(0x001002, |a| bus.read_word_debug(a));
    assert_eq!(disasm.mnemonic, "MOVE.W");
    assert_eq!(disasm.operands, "D0, D1");
    assert_eq!(bytes, 2);

    // 3. BRA $001000 ($60FA -> displacement -6)
    bus.write_word_debug(0x001004, 0x60FA);
    let (disasm, bytes) = disassemble(0x001004, |a| bus.read_word_debug(a));
    assert_eq!(disasm.mnemonic, "BRA");
    assert_eq!(disasm.operands, "$001000");
    assert_eq!(bytes, 2);
}

#[test]
fn test_debugger_trace_buffer_and_breakpoints() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    let mut dbg = Debugger::new();

    // Load NOPs at $001000..$001006
    bus.write_word_debug(0x001000, 0x4E71);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);

    cpu.set_pc_and_prime_prefetch(0x001000, &mut bus);

    // Set breakpoint at $001004
    dbg.breakpoints.add_pc_breakpoint(0x001004);
    assert!(dbg.breakpoints.check_pc(0x001004));

    // Step first instruction
    dbg.step_instruction(&mut cpu, &mut bus);
    assert_eq!(dbg.trace.len(), 1);

    let entries = dbg.trace.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].opcode, 0x4E71);
}

#[test]
fn test_assembler_primitives() {
    // 1. Raw hex words
    assert_eq!(assemble_instruction("4E71", 0x1000).unwrap(), vec![0x4E71]);
    assert_eq!(
        assemble_instruction("33FC 0042 0007 0000", 0x1000).unwrap(),
        vec![0x33FC, 0x0042, 0x0007, 0x0000]
    );
    assert_eq!(assemble_instruction("$3200", 0x1000).unwrap(), vec![0x3200]);

    // 2. Mnemonics
    assert_eq!(assemble_instruction("NOP", 0x1000).unwrap(), vec![0x4E71]);
    assert_eq!(assemble_instruction("RTS", 0x1000).unwrap(), vec![0x4E75]);
    assert_eq!(
        assemble_instruction("MOVE.W D0, D1", 0x1000).unwrap(),
        vec![0x3200]
    );
    assert_eq!(
        assemble_instruction("MOVEQ #$42, D0", 0x1000).unwrap(),
        vec![0x7042]
    );
    assert_eq!(
        assemble_instruction("CLR.L D0", 0x1000).unwrap(),
        vec![0x4280]
    );
    assert_eq!(
        assemble_instruction("SWAP D2", 0x1000).unwrap(),
        vec![0x4842]
    );

    // 3. Size mismatch detection helper check
    let orig_words = assemble_instruction("MOVE.W D0, D1", 0x1000).unwrap(); // 2 bytes
    let new_words_ok = assemble_instruction("NOP", 0x1000).unwrap(); // 2 bytes
    let new_words_err = assemble_instruction("33FC 0042 0007 0000", 0x1000).unwrap(); // 8 bytes

    assert_eq!(orig_words.len() * 2, 2);
    assert_eq!(new_words_ok.len() * 2, 2);
    assert_eq!(new_words_err.len() * 2, 8);
    assert_ne!(orig_words.len(), new_words_err.len());

    // 4. Invalid mnemonic
    let err = assemble_instruction("INVALID_OPCODE", 0x1000);
    assert!(err.is_err());
}

#[test]
fn test_temporal_history_capacity_and_navigation() {
    use debugger::temporal::{TemporalHistory, PAL_FRAME_CCK};
    use m68000::CpuState;

    let mut history = TemporalHistory::new(250_000);
    assert_eq!(history.capacity(), 250_000);
    assert!(history.is_recording());

    let mut state = CpuState::default();

    // Record 100 frames with CCK advancing by 1000 each
    for i in 0..100 {
        let cck = (i as u64) * 1000;
        let pc = 0x001000 + (i as u32) * 2;
        state.set_d_long(0, i as u32);
        history.record(cck, pc, 0x4E71, state.clone());
    }

    assert_eq!(history.len(), 100);
    assert_eq!(history.total_recorded(), 100);

    // Multi-granularity navigation: at live head, scrub_cursor is None
    assert_eq!(history.scrub_cursor, None);

    // Step back 10 instructions
    let pos10 = history.step_back_n(10).unwrap();
    assert_eq!(pos10, 89);
    assert_eq!(history.scrub_cursor, Some(89));

    // Step forward 5 instructions
    let pos5 = history.step_forward_n(5).unwrap();
    assert_eq!(pos5, 94);

    // Step back 1 PAL frame (~70,824 CCKs) -> from cck=94000 to cck=23176 (frame ~23)
    let frame_back_idx = history.step_frame_back(PAL_FRAME_CCK).unwrap();
    assert_eq!(frame_back_idx, 23);

    // Step forward 1 PAL frame -> from cck=23000 to cck=93824 (frame ~94)
    let frame_fwd_idx = history.step_frame_forward(PAL_FRAME_CCK).unwrap();
    assert_eq!(frame_fwd_idx, 94);

    // Test find_closest_cck binary search
    assert_eq!(history.find_closest_cck(50_000), Some(50));
    assert_eq!(history.find_closest_cck(50_400), Some(50));
    assert_eq!(history.find_closest_cck(50_700), Some(51));

    // Test dynamic capacity resizing
    history.set_capacity(50);
    assert_eq!(history.capacity(), 50);
    assert_eq!(history.len(), 50); // Kept 50 newest frames

    // Test recording toggle
    history.set_recording(false);
    assert!(!history.is_recording());
    history.record(200_000, 0x002000, 0x4E71, state);
    assert_eq!(history.len(), 50); // Not recorded when paused!
}

#[test]
fn test_conditional_breakpoints_and_watchpoints() {
    use debugger::{
        BreakpointCondition, BreakpointManager, ConditionOp, ConditionRegister, WatchAccess,
    };
    use m68000::CpuState;

    let mut bpm = BreakpointManager::new();
    let mut state = CpuState::default();

    // 1. Unconditional breakpoint at $001000
    bpm.add_pc_breakpoint(0x001000);
    assert!(bpm.check_pc_with_state(0x001000, &state));
    assert!(!bpm.check_pc_with_state(0x001002, &state));

    // 2. Conditional breakpoint: at $001004 IF D0 == 42
    bpm.add_conditional_breakpoint(
        0x001004,
        BreakpointCondition {
            register: ConditionRegister::D(0),
            op: ConditionOp::Eq,
            value: 42,
            mask: None,
        },
    );

    // D0 is 0 initially -> condition fails
    state.set_d_long(0, 0);
    assert!(!bpm.check_pc_with_state(0x001004, &state));

    // D0 is 42 -> condition passes
    state.set_d_long(0, 42);
    assert!(bpm.check_pc_with_state(0x001004, &state));

    // 3. Memory Watchpoints
    bpm.add_watchpoint(0x002000, 0x0020FF, WatchAccess::Write);
    assert!(bpm.check_watchpoint(0x002050, true)); // Write inside range -> triggers
    assert!(!bpm.check_watchpoint(0x002050, false)); // Read inside range -> does not trigger
    assert!(!bpm.check_watchpoint(0x003000, true)); // Out of range -> does not trigger
}

#[test]
fn test_debugger_session_controller() {
    use debugger::DebuggerSession;

    let mut session = DebuggerSession::new();
    assert_eq!(session.instructions_executed, 0);
    assert!(!session.is_running);
    assert!(!session.temporal.is_recording());
    session.temporal.set_recording(true); // Enable temporal for time-travel assertions

    // Load NOPs at $001000
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75]; // NOP, NOP, RTS
    session.load_binary(0x001000, &code, true);

    assert_eq!(session.machine.cpu.state.pc, 0x001004); // Prefetch primed to 0x1004
    assert_eq!(session.machine.cpu.state.ir, 0x4E71);

    // Step 1 instruction
    session.step_instruction();
    assert_eq!(session.instructions_executed, 1);
    assert_eq!(session.temporal.len(), 1);

    // Step CCK
    let prev_cck = session.debugger.current_cck;
    session.step_cck();
    assert_eq!(session.debugger.current_cck, prev_cck + 1);

    // Step backward (rewind)
    session.step_backward();
    assert_eq!(session.temporal.scrub_cursor, Some(0));

    // Step forward
    session.step_forward();
    assert_eq!(session.temporal.scrub_cursor, None); // Back to live head

    // Toggle run and execute bounded slice
    session.toggle_run();
    assert!(session.is_running);
    let executed = session.run_slice(10);
    assert!(executed > 0);
    assert_eq!(session.instructions_executed, 1 + executed as u64);

    // Reset cold
    session.reset_cold();
    assert_eq!(session.instructions_executed, 0);
    assert_eq!(session.temporal.len(), 0);
    assert!(!session.is_running);
}
