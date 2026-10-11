//! Bounded execution of small ROM programs through the production machine bus.
#![allow(dead_code)]

use config::A500Config;
use machine_loop::A500Machine;

pub const BOOT_PC: u32 = 0xF81000;
pub const BOOT_SSP: u32 = 0xC70000;
pub const RAM_MARKER: usize = 0x40000;
pub const COLOR_MARKER: u16 = 0x00A5;
pub const BOOT_CODE: &[u16] = &[
    0x702A, // MOVEQ #42,D0
    0x33C0,
    0x00C4,
    0x0000, // MOVE.W D0,$C40000.L (Slow RAM, outside the ROM overlay)
    0x33FC,
    COLOR_MARKER,
    0x00DF,
    0xF180, // MOVE.W #$A5,COLOR00
    0x5280, // ADDQ.L #1,D0
    0x7235, // MOVEQ #53,D1
    0x4E71, // NOP: also allows the color write to propagate
    0x60FE, // BRA.S *
];

pub fn rom_machine(code: &[u16]) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::default());
    let mut rom = vec![0xFF; 256 * 1024];
    rom[0..4].copy_from_slice(&BOOT_SSP.to_be_bytes());
    rom[4..8].copy_from_slice(&BOOT_PC.to_be_bytes());
    for (index, word) in code.iter().enumerate() {
        rom[0x1000 + index * 2..0x1002 + index * 2].copy_from_slice(&word.to_be_bytes());
    }
    machine.physical_memory.write_bytes_debug(0xF80000, &rom);
    machine.reset();
    machine
}

pub fn retire(machine: &mut A500Machine, next_pc: u32) {
    let start = machine.cck;
    for _ in 0..4096 {
        if machine.step_cck() {
            assert_eq!(machine.cpu.state.instruction_pc, next_pc);
            assert!(
                machine.cck > start,
                "instruction made no master-clock progress"
            );
            return;
        }
        assert!(!machine.cpu.state.halted, "unexpected CPU halt");
    }
    panic!("instruction did not retire within 4096 CCKs");
}

pub fn assert_boot_entry(machine: &A500Machine) {
    assert!(machine.physical_memory.is_low_memory_overlay_active());
    assert_eq!(machine.cck, 0);
    assert_eq!(machine.cpu.cycle_counter(), 0);
    assert_eq!(machine.cpu.state.sr(), 0x2700);
    assert_eq!(machine.cpu.state.ssp(), BOOT_SSP);
    assert_eq!(machine.cpu.state.a_long(7), BOOT_SSP);
    assert_eq!(machine.cpu.state.instruction_pc, BOOT_PC);
    assert_eq!(machine.cpu.state.pc, BOOT_PC + 4);
    assert_eq!(machine.cpu.state.ir, BOOT_CODE[0]);
    assert_eq!(machine.cpu.state.prefetch, BOOT_CODE[1]);
    assert!(!machine.cpu.state.stopped);
    assert!(!machine.cpu.state.halted);
    assert!(!machine.cpu.state.reset_line_asserted);
    assert!(!machine.keyboard.reset_line_asserted);
}

pub fn execute_boot(machine: &mut A500Machine) {
    for next_pc in [
        BOOT_PC + 2,
        BOOT_PC + 8,
        BOOT_PC + 16,
        BOOT_PC + 18,
        BOOT_PC + 20,
        BOOT_PC + 22,
    ] {
        retire(machine, next_pc);
        assert!(!machine.cpu.state.reset_line_asserted);
        assert!(!machine.keyboard.reset_line_asserted);
        assert_eq!(machine.cpu.state.a_long(7), BOOT_SSP);
        if next_pc >= BOOT_PC + 8 {
            assert_eq!(
                &machine.physical_memory.slow_ram.as_ref().unwrap()[RAM_MARKER..RAM_MARKER + 2],
                &[0, 42]
            );
        }
        if next_pc >= BOOT_PC + 18 {
            assert_eq!(machine.denise.color[0], COLOR_MARKER);
        }
    }
    assert_eq!(machine.cpu.state.d_long(0), 43);
    assert_eq!(machine.cpu.state.d_long(1), 53);
    // A second reset would clear COLOR00 even if the CPU kept progressing.
    for _ in 0..8 {
        retire(machine, BOOT_PC + 22);
        assert_eq!(machine.denise.color[0], COLOR_MARKER);
        assert_eq!(machine.cpu.state.d_long(0), 43);
    }
}
