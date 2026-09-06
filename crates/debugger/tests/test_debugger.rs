use debugger::{disassemble, Debugger};
use m68000::Cpu;
use memory_bus::MemoryBus;

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

    cpu.reload_pc_and_prefetch(0x001000, &mut bus);

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
