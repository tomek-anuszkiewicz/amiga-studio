#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! System instruction boundaries and externally observable RESET duration.
mod common;
use common::{run_instruction, Access, RecordingBus};
use cpu::Cpu;
use physical_memory::PhysicalMemory;

#[test]
fn reset_pin_is_asserted_for_124_clocks() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x1000, 0x4e70);
    bus.write_word_debug(0x1002, 0x4e71);
    bus.write_word_debug(0x1004, 0x4e71);
    let mut cpu = Cpu::new();
    cpu.set_pc_and_prime_prefetch(0x1000, &mut bus);
    let mut asserted = 0;
    for tick in 1..=66 {
        let done = cpu.step_cck(&mut bus);
        assert_eq!(done, tick == 66);
        if cpu.state.reset_line_asserted {
            asserted += 2;
        }
    }
    assert_eq!(asserted, 124, "Motorola RESET output duration");
    assert!(!cpu.state.reset_line_asserted);
    assert_eq!(cpu.cycle_counter(), 132);
    assert_eq!(cpu.state.instruction_pc, 0x1002);
}

#[test]
fn privileged_system_instructions_reject_user_mode_before_operand_access() {
    // Vector 8 entry is 34 clocks; the saved PC is the rejected opcode address.
    for opcode in [
        0x46d0, 0x4e60, 0x4e68, 0x027c, 0x007c, 0x0a7c, 0x4e72, 0x4e70,
    ] {
        for waits in [0, 1, 3] {
            let mut memory = PhysicalMemory::new();
            memory.map_chip_ram_to_low_memory();
            memory.write_word_debug(0x20, 0);
            memory.write_word_debug(0x22, 0x2000);
            memory.write_word_debug(0x2000, 0x4e71);
            memory.write_word_debug(0x2002, 0x4e75);
            let mut cpu = Cpu::new();
            cpu.state.set_ssp(0x8000);
            cpu.state.set_usp(0x6000);
            cpu.state.set_sr(0x0015);
            cpu.state.pc = 0x1004;
            cpu.state.instruction_pc = 0x1000;
            cpu.state.ir = opcode;
            cpu.state.prefetch = 0x2700;
            cpu.state.set_a_long(0, 0x3000);
            let transfers = [
                Access::WriteWord(0x7ffe, 0x1000),
                Access::WriteWord(0x7ffa, 0x0015),
                Access::WriteWord(0x7ffc, 0),
                Access::ReadWord(0x20),
                Access::ReadWord(0x22),
                Access::ReadWord(0x2000),
                Access::ReadWord(0x2002),
            ];
            let mut bus =
                RecordingBus::new(memory, transfers.into_iter().map(|a| (a, waits)).collect());
            run_instruction(&mut cpu, &mut bus, 34);
            assert_eq!(cpu.state.usp(), 0x6000);
            assert_eq!(cpu.state.a_long(0), 0x3000);
            assert_eq!(cpu.state.sr(), 0x2015);
            assert!(!cpu.state.stopped);
            assert!(!cpu.state.reset_line_asserted);
        }
    }
}

#[test]
fn stop_retires_at_four_clocks_without_bus_access_and_selects_the_loaded_stack() {
    for sr in [0x2000, 0x2700, 0x0015] {
        let mut cpu = Cpu::new();
        cpu.state.set_ssp(0x8000);
        cpu.state.set_usp(0x6000);
        cpu.state.pc = 0x1004;
        cpu.state.instruction_pc = 0x1000;
        cpu.state.ir = 0x4e72;
        cpu.state.prefetch = sr;
        let mut bus = RecordingBus::new(PhysicalMemory::new(), vec![]);
        assert!(!cpu.step_cck(&mut bus));
        assert!(!cpu.state.stopped);
        assert!(cpu.step_cck(&mut bus));
        assert_eq!(cpu.cycle_counter(), 4);
        assert!(cpu.state.stopped);
        assert_eq!(cpu.state.sr(), sr);
        assert_eq!(cpu.state.instruction_pc, 0x1004);
        assert_eq!(
            cpu.state.a_long(7),
            if sr & 0x2000 != 0 { 0x8000 } else { 0x6000 }
        );
        assert!(!cpu.step_cck(&mut bus));
        assert_eq!(cpu.cycle_counter(), 4);
        bus.assert_complete();
    }
}

#[test]
fn reset_releases_output_before_a_stalled_prefetch() {
    for waits in [0, 1, 3] {
        let mut memory = PhysicalMemory::new();
        memory.map_chip_ram_to_low_memory();
        memory.write_word_debug(0x1004, 0x4e75);
        let mut cpu = Cpu::new();
        cpu.state.pc = 0x1004;
        cpu.state.instruction_pc = 0x1000;
        cpu.state.ir = 0x4e70;
        cpu.state.prefetch = 0x4e71;
        let mut bus = RecordingBus::new(memory, vec![(Access::ReadWord(0x1004), waits)]);
        run_instruction(&mut cpu, &mut bus, 132);
        assert!(!cpu.state.reset_line_asserted);
        assert_eq!(cpu.state.ir, 0x4e71);
        assert_eq!(cpu.state.prefetch, 0x4e75);
    }
}

#[test]
fn user_status_operations_preserve_system_byte_and_stack_bank() {
    // MC68000 MOVE from SR and CCR operations are unprivileged.
    for (opcode, immediate, clocks, expected_sr) in [
        (0x40c0, 0x4e71, 6, 0x0015),
        (0x44c0, 0x4e71, 12, 0x000a),
        (0x023c, 0x000a, 20, 0x0000),
        (0x003c, 0x000a, 20, 0x001f),
        (0x0a3c, 0x000a, 20, 0x001f),
    ] {
        for waits in [0, 1, 3] {
            let mut cpu = Cpu::new();
            cpu.state.set_ssp(0x8000);
            cpu.state.set_usp(0x6000);
            cpu.state.set_sr(0x0015);
            cpu.state.set_d_long(0, 0x1234_000a);
            cpu.state.pc = 0x1004;
            cpu.state.instruction_pc = 0x1000;
            cpu.state.ir = opcode;
            cpu.state.prefetch = immediate;
            let mut memory = PhysicalMemory::new();
            memory.map_chip_ram_to_low_memory();
            memory.write_word_debug(0x1002, immediate);
            memory.write_word_debug(0x1004, 0x4e71);
            memory.write_word_debug(0x1006, 0x4e75);
            let addresses = if opcode == 0x40c0 {
                vec![0x1004]
            } else if opcode == 0x44c0 {
                vec![0x1002, 0x1004]
            } else {
                vec![0x1004, 0x1004, 0x1006]
            };
            let mut bus = RecordingBus::new(
                memory,
                addresses
                    .into_iter()
                    .map(|a| (Access::ReadWord(a), waits))
                    .collect(),
            );
            run_instruction(&mut cpu, &mut bus, clocks);
            assert_eq!(cpu.state.sr(), expected_sr);
            assert_eq!(cpu.state.usp(), 0x6000);
            assert_eq!(cpu.state.ssp(), 0x8000);
            assert_eq!(cpu.state.a_long(7), 0x6000);
            if opcode == 0x40c0 {
                assert_eq!(cpu.state.d_long(0), 0x1234_0015);
            }
        }
    }
}
