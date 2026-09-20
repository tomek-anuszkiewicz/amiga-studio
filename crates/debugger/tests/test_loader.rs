#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cpu::Cpu;
use debugger::loader::{inject_binary, DEFAULT_TARGET_ADDRESS};
use physical_memory::PhysicalMemory;

#[test]
fn test_inject_binary_with_auto_prime() {
    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();

    // SP is 0 initially in raw Cpu::new()
    assert_eq!(cpu.state.a_long(7), 0);

    // Code: NOP ($4E71), RTS ($4E75)
    let code: [u8; 4] = [0x4E, 0x71, 0x4E, 0x75];
    let written = inject_binary(&mut cpu, &mut bus, DEFAULT_TARGET_ADDRESS, &code, true);
    assert_eq!(written, 4);

    // Verify written memory
    assert_eq!(bus.read_word_debug(0x001000), 0x4E71);
    assert_eq!(bus.read_word_debug(0x001002), 0x4E75);

    // Verify PC primed: after prefetch of 2 words, PC is at $001004, IR is $4E71, IRC is $4E75
    assert_eq!(cpu.state.pc, 0x001004);
    assert_eq!(cpu.state.ir, 0x4E71);
    assert_eq!(cpu.state.prefetch, 0x4E75);

    // Verify SP auto-initialized to 512KB Chip RAM top ($080000)
    assert_eq!(cpu.state.a_long(7), 0x080000);
    assert_eq!(cpu.state.ssp, 0x080000);
}

#[test]
fn test_inject_binary_without_auto_prime_preserves_cpu() {
    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();

    cpu.state.pc = 0x005000;
    cpu.state.set_a_long(7, 0x004000);

    let code: [u8; 2] = [0x4E, 0x71];
    let written = inject_binary(&mut cpu, &mut bus, 0x002000, &code, false);
    assert_eq!(written, 2);

    // Memory was updated
    assert_eq!(bus.read_word_debug(0x002000), 0x4E71);

    // CPU state was untouched
    assert_eq!(cpu.state.pc, 0x005000);
    assert_eq!(cpu.state.a_long(7), 0x004000);
}
