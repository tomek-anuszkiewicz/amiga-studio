use debugger::{Debugger, DebuggerSession};
use m68000::Cpu;
use physical_memory::PhysicalMemory;

#[test]
fn test_debugger_stepping_and_breakpoints() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    let mut dbg = Debugger::new();

    // 3 NOPs ($4E71)
    bus.write_word_debug(0x1000, 0x4E71);
    bus.write_word_debug(0x1002, 0x4E71);
    bus.write_word_debug(0x1004, 0x4E71);
    cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);

    // Breakpoint at $1004
    dbg.breakpoints.add_pc_breakpoint(0x1004);

    // Run until breakpoint
    let executed = dbg.run_until_breakpoint(&mut cpu, &mut bus, 10);
    assert_eq!(executed, 2); // Executed $1000 and $1002, stopped at $1004
    assert_eq!(cpu.state.pc.wrapping_sub(4), 0x1004);
}

#[test]
fn test_debugger_session_full_lifecycle() {
    let mut session = DebuggerSession::new();
    assert_eq!(session.instructions_executed, 0);
    assert!(!session.is_running);

    // Load binary code: NOP, NOP, RTS
    let code: [u8; 6] = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    let written = session.load_binary(0x001000, &code, true);
    assert_eq!(written, 6);

    assert!(!session.temporal.is_recording());
    session.temporal.set_recording(true); // Enable temporal for time-travel assertions

    // Memory diff snapshot test
    session.capture_memory_snapshot(0x001000);
    assert_eq!(session.prev_hex_bytes[0], 0x4E);
    assert_eq!(session.prev_hex_bytes[1], 0x71);

    // Step 1 instruction
    session.step_instruction();
    assert_eq!(session.instructions_executed, 1);
    assert!(session.prev_cpu_state.is_some());
    assert_eq!(session.temporal.len(), 1);

    // Step CCK
    let prev_cck = session.debugger.current_cck;
    session.step_cck();
    assert_eq!(session.debugger.current_cck, prev_cck + 1);

    // Time travel
    session.step_backward();
    assert_eq!(session.temporal.scrub_cursor, Some(0));

    session.step_forward();
    assert_eq!(session.temporal.scrub_cursor, None); // Live head

    // Free run slice
    session.toggle_run();
    assert!(session.is_running);
    let ran = session.run_slice(5);
    assert!(ran > 0);

    // Warm reset
    session.reset_warm();
    assert!(!session.is_running);
    assert!(session
        .machine
        .physical_memory
        .is_low_memory_overlay_active());

    // Cold reset
    session.reset_cold();
    assert_eq!(session.instructions_executed, 0);
    assert_eq!(session.temporal.len(), 0);
    assert_eq!(session.debugger.trace.len(), 0);
    assert!(session
        .machine
        .physical_memory
        .is_low_memory_overlay_active());
}

#[test]
fn test_debugger_session_reset_and_overlay_lifecycle() {
    let mut session = DebuggerSession::new();
    // Default session creation begins with cold reset -> overlay active
    assert!(session
        .machine
        .physical_memory
        .is_low_memory_overlay_active());

    // Loading binary via inject_binary() disengages boot overlay for synthetic test execution
    let code: [u8; 4] = [0x4E, 0x71, 0x4E, 0x71]; // NOP, NOP
    session.load_binary(0x001000, &code, true);
    assert!(!session
        .machine
        .physical_memory
        .is_low_memory_overlay_active());

    // Warm reset re-engages overlay per hardware reality
    session.reset_warm();
    assert!(session
        .machine
        .physical_memory
        .is_low_memory_overlay_active());

    // Loading another binary disengages overlay again
    session.load_binary(0x001000, &code, true);
    assert!(!session
        .machine
        .physical_memory
        .is_low_memory_overlay_active());

    // Cold reset re-engages overlay per hardware reality
    session.reset_cold();
    assert!(session
        .machine
        .physical_memory
        .is_low_memory_overlay_active());
}

#[test]
fn test_lea_step_instruction() {
    let mut session = DebuggerSession::new();
    let code: [u8; 8] = [
        0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, // LEA ($002000).L, A0
        0x42, 0x40, // CLR.W D0
    ];
    session.load_binary(0x001000, &code, true);

    println!(
        "Initial: pc={:06X}, instruction_pc={:06X}, ir={:04X}",
        session.machine.cpu.state.pc,
        session.machine.cpu.state.instruction_pc,
        session.machine.cpu.state.ir
    );
    for i in 0..8 {
        session.step_cck();
        println!(
            "After CCK {}: pc={:06X}, instruction_pc={:06X}, micro_step={}, pc-4={:06X}",
            i,
            session.machine.cpu.state.pc,
            session.machine.cpu.state.instruction_pc,
            session.machine.cpu.state.micro.micro_step,
            session.machine.cpu.state.pc.wrapping_sub(4) & 0x00FF_FFFF
        );
    }
}

#[test]
fn test_lea_step_instruction_call() {
    let mut session = DebuggerSession::new();
    let code: [u8; 8] = [
        0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, // LEA ($002000).L, A0
        0x42, 0x40, // CLR.W D0
    ];
    session.load_binary(0x001000, &code, true);
    session.step_instruction();
    println!(
        "After step_instruction: pc={:06X}, instruction_pc={:06X}, a0={:08X}",
        session.machine.cpu.state.pc,
        session.machine.cpu.state.instruction_pc,
        session.machine.cpu.state.a_regs()[0]
    );
}
