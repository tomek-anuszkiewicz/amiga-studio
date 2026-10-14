#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! MC68000 manual Sections 6.3 and 8: exception boundaries absent from silicon inputs.
mod common;
use common::{retire_to, run_instruction, Access, RecordingBus};
use cpu::Cpu;
use physical_memory::PhysicalMemory;

fn fixture(opcode: u16, sr: u16) -> (Cpu, PhysicalMemory) {
    let mut memory = PhysicalMemory::new();
    memory.map_chip_ram_to_low_memory();
    for vector in [3, 4, 8, 9, 10, 11, 28, 32] {
        memory.write_word_debug(vector * 4, 0);
        memory.write_word_debug(vector * 4 + 2, 0x2000);
    }
    for address in [0x1000, 0x1002, 0x1004, 0x1006, 0x2000, 0x2002, 0x2004] {
        memory.write_word_debug(address, 0x4e71);
    }
    memory.write_word_debug(0x1000, opcode);
    let mut cpu = Cpu::new();
    cpu.state.set_ssp(0x8000);
    cpu.state.set_usp(0x6000);
    cpu.state.set_sr(sr);
    cpu.set_pc_and_prime_prefetch(0x1000, &mut memory);
    (cpu, memory)
}

fn frame(sp: u32, pc: u32, sr: u16, vector: u32) -> Vec<Access> {
    vec![
        Access::WriteWord(sp - 2, pc as u16),
        Access::WriteWord(sp - 6, sr),
        Access::WriteWord(sp - 4, (pc >> 16) as u16),
        Access::ReadWord(vector * 4),
        Access::ReadWord(vector * 4 + 2),
        Access::ReadWord(0x2000),
        Access::ReadWord(0x2002),
    ]
}

#[test]
fn illegal_and_emulator_lines_stack_the_rejected_opcode_pc() {
    for (opcode, vector) in [(0x4afc, 4), (0x7100, 4), (0xa123, 10), (0xf123, 11)] {
        for sr in [0x0015, 0x2015] {
            for waits in [0, 1, 3] {
                let (mut cpu, memory) = fixture(opcode, sr);
                let mut bus = RecordingBus::new(
                    memory,
                    frame(0x8000, 0x1000, sr, vector)
                        .into_iter()
                        .map(|access| (access, waits))
                        .collect(),
                );
                run_instruction(&mut cpu, &mut bus, 34);
                assert_eq!(cpu.state.instruction_pc(), 0x2000);
                assert_eq!(cpu.state.ssp(), 0x7ffa);
                assert_eq!(cpu.state.usp(), 0x6000);
                assert_eq!(cpu.state.sr(), 0x2015);
            }
        }
    }
}

#[test]
fn trace_precedes_pending_interrupt_and_stacks_the_next_opcode() {
    let (mut cpu, mut memory) = fixture(0x4e71, 0xa015);
    cpu.state.set_ipl(4);
    assert_eq!(cpu.step_instruction(&mut memory), 4);
    assert_eq!(cpu.step_instruction(&mut memory), 34);
    assert_eq!(memory.read_word_debug(0x7ffa), 0xa015);
    assert_eq!(memory.read_word_debug(0x7ffe), 0x1002);
    assert_eq!(cpu.state.interrupt_mask(), 0);
    assert_eq!(cpu.step_instruction(&mut memory), 44);
    assert_eq!(memory.read_word_debug(0x7ff4), 0x2015);
    assert_eq!(memory.read_word_debug(0x7ff8), 0x2000);
    assert_eq!(cpu.state.interrupt_mask(), 4);
}

#[test]
fn trace_is_sampled_before_an_instruction_clears_t_and_wakes_stop() {
    for (opcode, clocks, return_pc) in [(0x46fc, 16, 0x1004), (0x4e72, 4, 0x1004)] {
        let (mut cpu, mut memory) = fixture(opcode, 0xa000);
        memory.write_word_debug(0x1002, 0x2000);
        cpu.set_pc_and_prime_prefetch(0x1000, &mut memory);
        assert_eq!(cpu.step_instruction(&mut memory), clocks);
        assert_eq!(cpu.step_instruction(&mut memory), 34);
        assert!(!cpu.state.is_stopped());
        assert_eq!(memory.read_word_debug(0x7ffa), 0x2000);
        assert_eq!(memory.read_word_debug(0x7ffe), return_pc);
        assert_eq!(cpu.state.instruction_pc(), 0x2000);
    }
}

#[test]
fn accepted_interrupt_level_survives_pin_change_before_entry() {
    let (mut cpu, mut memory) = fixture(0x4e71, 0x2000);
    memory.write_word_debug(25 * 4 + 2, 0x3000);
    cpu.state.set_ipl(4);
    cpu.step_instruction(&mut memory);
    cpu.state.set_ipl(0);
    cpu.step_instruction(&mut memory);
    assert_eq!(cpu.state.instruction_pc(), 0x2000);
    assert_eq!(cpu.state.interrupt_mask(), 4);
}

#[test]
fn group_zero_double_fault_stops_bus_activity_until_external_reset() {
    for odd_stack in [false, true] {
        let (mut cpu, mut memory) = fixture(0x3010, 0xa000); // MOVE.W (A0),D0
        cpu.state.set_a_long(0, 0x3001);
        if odd_stack {
            cpu.state.set_ssp(0x8001);
        } else {
            memory.write_word_debug(14, 0x2001);
        }
        cpu.step_instruction(&mut memory);
        assert!(cpu.state.is_halted());
        let clocks = cpu.cycle_counter();
        assert_eq!(cpu.step_instruction(&mut memory), 0);
        assert!(!cpu.step_cck(&mut memory));
        assert_eq!(cpu.cycle_counter(), clocks);
        memory.write_word_debug(0, 0);
        memory.write_word_debug(2, 0x8000);
        memory.write_word_debug(4, 0);
        memory.write_word_debug(6, 0x1000);
        memory.write_word_debug(0x1000, 0x702A); // replace the faulting program for reboot
        memory.write_word_debug(0x1002, 0x5280);
        memory.write_word_debug(0x1004, 0x7235);
        cpu.reset_warm(&mut memory);
        assert!(!cpu.state.is_halted());
        assert_eq!(cpu.state.instruction_pc(), 0x1000);
        assert_eq!(cpu.state.pc(), 0x1004);
        assert_eq!(cpu.state.sr(), 0x2700);
        for next in [0x1002, 0x1004, 0x1006] {
            retire_to(&mut cpu, &mut memory, next);
        }
        assert_eq!(cpu.state.d_long(0), 43);
        assert_eq!(cpu.state.d_long(1), 53);
    }
}

#[test]
fn trace_handler_clears_saved_t_and_returns_through_nested_trap_frames() {
    for nested_trap in [false, true] {
        let (mut cpu, mut memory) = fixture(if nested_trap { 0x4E40 } else { 0x702A }, 0xA000);
        for (index, word) in [0x7235, 0x5281, 0x7837].into_iter().enumerate() {
            memory.write_word_debug(0x1002 + index as u32 * 2, word);
        }
        memory.write_word_debug(9 * 4 + 2, 0x3000);
        let handler: &[u16] = if nested_trap {
            &[0x5283, 0x026F, 0x7FFF, 6, 0x4E73] // clear T in the outer trap frame
        } else {
            &[0x5283, 0x0257, 0x7FFF, 0x4E73] // clear T in this trace frame
        };
        for (index, word) in handler.iter().enumerate() {
            memory.write_word_debug(0x3000 + index as u32 * 2, *word);
        }
        memory.write_word_debug(0x2000, 0x5282); // trap entry counter
        memory.write_word_debug(0x2002, 0x4E73);
        cpu.set_pc_and_prime_prefetch(0x1000, &mut memory);
        retire_to(
            &mut cpu,
            &mut memory,
            if nested_trap { 0x2000 } else { 0x1002 },
        );
        retire_to(&mut cpu, &mut memory, 0x3000);
        assert_eq!(cpu.state.ssp(), if nested_trap { 0x7FF4 } else { 0x7FFA });
        assert_eq!(
            memory.read_word_debug(cpu.state.ssp() + 4),
            if nested_trap { 0x2000 } else { 0x1002 }
        );
        retire_to(&mut cpu, &mut memory, 0x3002);
        retire_to(
            &mut cpu,
            &mut memory,
            if nested_trap { 0x3008 } else { 0x3006 },
        );
        retire_to(
            &mut cpu,
            &mut memory,
            if nested_trap { 0x2000 } else { 0x1002 },
        );
        if nested_trap {
            retire_to(&mut cpu, &mut memory, 0x2002);
            retire_to(&mut cpu, &mut memory, 0x1002);
        }
        assert!(!cpu.state.is_trace());
        for next in [0x1004, 0x1006, 0x1008] {
            retire_to(&mut cpu, &mut memory, next);
        }
        assert_eq!(cpu.state.d_long(1), 54);
        assert_eq!(cpu.state.d_long(2), u32::from(nested_trap));
        assert_eq!(cpu.state.d_long(3), 1);
        assert_eq!(cpu.state.d_long(4), 55);
        assert_eq!(cpu.state.ssp(), 0x8000);
        assert_eq!(cpu.state.usp(), 0x6000);
    }
}

#[test]
fn trace_follows_trap_but_not_decode_privilege_or_address_exceptions() {
    for opcode in [0x4afc, 0xa123, 0xf123, 0x4e40, 0x4e70] {
        let sr = if opcode == 0x4e70 { 0x8000 } else { 0xa000 };
        let (mut cpu, mut memory) = fixture(opcode, sr);
        assert_eq!(cpu.step_instruction(&mut memory), 34);
        if opcode == 0x4e40 {
            assert_eq!(cpu.step_instruction(&mut memory), 34);
            assert_eq!(cpu.state.ssp(), 0x7ff4);
            assert_eq!(memory.read_word_debug(0x7ff8), 0x2000);
            assert_eq!(cpu.step_instruction(&mut memory), 4);
        } else {
            assert_eq!(cpu.step_instruction(&mut memory), 4);
            assert_eq!(cpu.state.ssp(), 0x7ffa);
        }
    }
    let (mut cpu, mut memory) = fixture(0x3010, 0xa000);
    cpu.state.set_a_long(0, 0x3001);
    cpu.step_instruction(&mut memory);
    assert_eq!(cpu.step_instruction(&mut memory), 4);
    assert_eq!(cpu.state.ssp(), 0x7ff2);
}

#[test]
fn trace_shared_frame_retries_preserve_transfer_order_and_add_only_wait_clocks() {
    for waits in [0, 1, 3] {
        let (mut cpu, mut memory) = fixture(0x4e71, 0xa015);
        cpu.step_instruction(&mut memory);
        let script = frame(0x8000, 0x1002, 0xa015, 9);
        let mut bus = RecordingBus::new(memory, script.into_iter().map(|a| (a, waits)).collect());
        run_instruction(&mut cpu, &mut bus, 34);
        assert_eq!(cpu.state.instruction_pc(), 0x2000);
        assert_eq!(cpu.state.sr(), 0x2015);
    }
}

#[test]
fn trace_frame_callbacks_follow_the_shared_cck_timeline() {
    let (mut cpu, mut memory) = fixture(0x4e71, 0xa015);
    cpu.step_instruction(&mut memory);
    let mut bus = RecordingBus::new(
        memory,
        frame(0x8000, 0x1002, 0xa015, 9)
            .into_iter()
            .map(|a| (a, 0))
            .collect(),
    );
    let start = cpu.cycle_counter();
    let mut callbacks = vec![];
    for _ in 0..17 {
        let before = bus.attempts.len();
        let done = cpu.step_cck(&mut bus);
        if bus.attempts.len() != before {
            callbacks.push(cpu.cycle_counter() - start);
        }
        if done {
            break;
        }
    }
    // Shared 34-clock frame: writes in CCK2, reads in CCK1, one idle before lookahead.
    assert_eq!(callbacks, [8, 12, 16, 18, 22, 26, 32]);
    bus.assert_complete();
}

#[test]
fn interrupt_levels_one_through_six_select_their_autovectors() {
    for level in 1..=6 {
        let (mut cpu, mut memory) = fixture(0x4e71, 0x0015);
        memory.write_word_debug((24 + u32::from(level)) * 4, 0);
        memory.write_word_debug((24 + u32::from(level)) * 4 + 2, 0x2000);
        cpu.state.set_ipl(level);
        assert_eq!(cpu.step_instruction(&mut memory), 4);
        assert_eq!(cpu.step_instruction(&mut memory), 44);
        assert_eq!(cpu.state.interrupt_mask(), level);
        assert_eq!(cpu.state.usp(), 0x6000);
        assert_eq!(memory.read_word_debug(0x7ffa), 0x0015);
        assert_eq!(memory.read_word_debug(0x7ffe), 0x1002);
    }
}

#[test]
fn external_reset_reads_vectors_without_sanitizing_or_stacking() {
    let (mut cpu, mut memory) = fixture(0x4e71, 0x2000);
    cpu.state.set_d_long(0, 0x1234);
    cpu.state.set_a_long(0, 0x5678);
    for odd_target in [true, false] {
        memory.write_word_debug(0, 0x1234);
        memory.write_word_debug(2, 0x8001);
        memory.write_word_debug(4, 0);
        memory.write_word_debug(6, if odd_target { 0x1001 } else { 0x1000 });
        cpu.reset_warm(&mut memory);
        assert_eq!(cpu.state.ssp(), 0x1234_8001);
        assert_eq!(cpu.state.is_halted(), odd_target);
        assert_eq!(cpu.state.d_long(0), 0x1234);
        assert_eq!(cpu.state.a_long(0), 0x5678);
        assert_eq!(cpu.state.usp(), 0x6000);
        assert_eq!(
            cpu.cycle_counter(),
            0,
            "host reset is an setup API without clocked transfers"
        );
    }
    cpu.reset(&mut memory);
    assert_eq!(cpu.state.d_long(0), 0);
    assert_eq!(cpu.state.a_long(0), 0);
    assert_eq!(cpu.state.usp(), 0);
}

#[test]
fn odd_short_frame_handler_enters_address_error_without_reading_the_target() {
    let (mut cpu, mut memory) = fixture(0x4e40, 0x2000);
    memory.write_word_debug(32 * 4 + 2, 0x2001);
    cpu.step_instruction(&mut memory);
    assert!(!cpu.state.is_halted());
    assert_eq!(cpu.state.instruction_pc(), 0x2000);
    assert_eq!(
        cpu.state.ssp(),
        0x7fec,
        "short frame followed by diagnostic frame"
    );
    assert_eq!(
        memory.read_word_debug(0x7ff0),
        0x2001,
        "fault address low word"
    );
    assert_eq!(cpu.step_instruction(&mut memory), 4);
}

#[test]
fn rte_enables_trace_for_the_following_instruction() {
    let (mut cpu, mut memory) = fixture(0x4e73, 0x2000);
    memory.write_word_debug(0x8000, 0xa000);
    memory.write_word_debug(0x8002, 0);
    memory.write_word_debug(0x8004, 0x1002);
    assert_eq!(cpu.step_instruction(&mut memory), 20);
    assert_eq!(cpu.state.instruction_pc(), 0x1002);
    assert_eq!(cpu.step_instruction(&mut memory), 4);
    assert_eq!(cpu.step_instruction(&mut memory), 34);
    assert_eq!(memory.read_word_debug(0x8004), 0x1004);
}
