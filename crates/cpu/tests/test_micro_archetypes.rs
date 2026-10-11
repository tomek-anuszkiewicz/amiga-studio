#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Cycle-exact micro-step instruction decomposition unit tests
//!
//! Verifies the 6 representative archetypes of Step 1 in ROADMAP.md:
//! 1. Internal Register ALU (4 clocks / 2 CCKs): NOP, MOVE.w Dx, Dy
//! 2. Memory Read (8 clocks / 4 CCKs): MOVE.w (Ax), Dy
//! 3. Memory Write (8 clocks / 4 CCKs, Class 1): MOVE.w Dx, (Ay)
//! 4. Read-Modify-Write (12 clocks / 6 CCKs, Class 0): ADD.w Dx, (Ay)
//! 5. Conditional Branching (8 vs 10 clocks): Bcc.s / BRA.s (untaken: 8 clocks, taken: 10 clocks)
//! 6. Stack Push & Subroutine Call: PEA (An) (12 clocks) and JSR (An) (16 clocks)
//!
//! Also validates bus contention stalls (Agnus DMA locking Chip RAM during CCK1 for reads, CCK2 for writes).

mod common;

use common::{run_instruction, Access, RecordingBus};
use cpu::Cpu;
use physical_memory::PhysicalMemory;

/// Helper to set up a test CPU and PhysicalMemory with mapped Chip RAM and initialized prefetch pipeline
fn setup_test_machine(base_pc: u32) -> (Cpu, PhysicalMemory) {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    // Default stack pointer to 0x008000
    cpu.state.set_a_long(7, 0x008000);
    cpu.state.pc = base_pc;
    (cpu, bus)
}

/// Initializes CPU prefetch queue from memory at `base_pc`
fn prime_prefetch(cpu: &mut Cpu, bus: &mut PhysicalMemory) {
    let pc = cpu.state.pc;
    cpu.state.ir = bus.read_word_debug(pc);
    cpu.state.prefetch = bus.read_word_debug(pc.wrapping_add(2));
    cpu.state.pc = pc.wrapping_add(4);
}

#[test]
fn test_archetype1_nop_4_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: NOP (0x4E71), NOP (0x4E71)
    bus.write_word_debug(0x001000, 0x4E71);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(clocks, 4, "NOP must take exactly 4 CPU clocks (2 CCKs)");
    assert_eq!(cpu.state.pc, 0x001006);
}

#[test]
fn test_archetype1_move_reg_to_reg_4_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: MOVE.w D0, D1 (0x3200)
    bus.write_word_debug(0x001000, 0x3200);
    bus.write_word_debug(0x001002, 0x4E71); // Next opcode (NOP)
    bus.write_word_debug(0x001004, 0x4E71);
    prime_prefetch(&mut cpu, &mut bus);

    cpu.state.set_d_long(0, 0x1234_5678);
    cpu.state.set_d_long(1, 0x0000_0000);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 4,
        "MOVE.w Dx, Dy must take exactly 4 CPU clocks (2 CCKs)"
    );
    assert_eq!(cpu.state.d_long(1), 0x0000_5678);
    assert_eq!(
        cpu.state.ccr(),
        0x00,
        "CCR: N=0, Z=0, V=0, C=0 for positive non-zero"
    );
}

#[test]
fn test_archetype2_move_mem_read_8_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: MOVE.w (A0), D0 (0x3010)
    bus.write_word_debug(0x001000, 0x3010);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);
    // Data in Chip RAM
    bus.write_word_debug(0x002000, 0xCAFE);

    cpu.state.set_a_long(0, 0x002000);
    cpu.state.set_d_long(0, 0);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 8,
        "MOVE.w (Ax), Dy must take exactly 8 CPU clocks (4 CCKs)"
    );
    assert_eq!(cpu.state.d_word(0), 0xCAFE);
}

#[test]
fn test_archetype2_move_mem_read_dma_contention_stall_at_cck1() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: MOVE.w (A0), D0 (0x3010)
    bus.write_word_debug(0x001000, 0x3010);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);
    bus.write_word_debug(0x002000, 0xCAFE);

    cpu.state.set_a_long(0, 0x002000);
    cpu.state.set_d_long(0, 0);
    prime_prefetch(&mut cpu, &mut bus);

    // Agnus DMA occupies Chip RAM before read cycle begins
    bus.chip_ram_blocked = true;

    // Color Clock 1: instruction handler initiates bus read, but Gary withholds _DTACK at CCK1 -> WaitState
    let r1 = cpu.step_cck(&mut bus);
    assert!(!r1);
    assert_eq!(cpu.state.micro.micro_step, 0);

    // Color Clock 2: still blocked -> WaitState
    let r2 = cpu.step_cck(&mut bus);
    assert!(!r2);
    assert_eq!(cpu.state.micro.micro_step, 0);

    // Agnus completes DMA transfer and frees the bus
    bus.chip_ram_blocked = false;

    // Color Clock 3: CCK1 succeeds and advances to CCK2
    let r3 = cpu.step_cck(&mut bus);
    assert!(!r3);
    assert_eq!(cpu.state.micro.micro_step, 1);

    // Color Clock 4: CCK2 completes transaction and latches data
    let r4 = cpu.step_cck(&mut bus);
    assert!(!r4);

    // Next Color Clocks: Prefetch next instruction word (CCK1 and CCK2)
    let r5 = cpu.step_cck(&mut bus);
    assert!(!r5);
    let r6 = cpu.step_cck(&mut bus);
    assert!(r6);

    // Total clocks = 8 base clocks + 2 * (2 wait states) = 12 clocks
    assert_eq!(cpu.cycle_counter(), 12);
    assert_eq!(cpu.state.d_word(0), 0xCAFE);
}

#[test]
fn test_archetype3_move_mem_write_8_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: MOVE.w D0, (A0) (0x3080)
    bus.write_word_debug(0x001000, 0x3080);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);

    cpu.state.set_d_long(0, 0x0000_BEEF);
    cpu.state.set_a_long(0, 0x003000);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 8,
        "MOVE.w Dx, (Ay) must take exactly 8 CPU clocks (4 CCKs)"
    );
    assert_eq!(bus.read_word_debug(0x003000), 0xBEEF);
}

#[test]
fn test_archetype3_move_mem_write_dma_contention_stall_at_cck2() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: MOVE.w D0, (A0) (0x3080)
    bus.write_word_debug(0x001000, 0x3080);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);

    cpu.state.set_d_long(0, 0x0000_BEEF);
    cpu.state.set_a_long(0, 0x003000);
    prime_prefetch(&mut cpu, &mut bus);

    // Color Clock 1: Initiates write cycle, outputs address/data on bus, finishes CCK1
    let r1 = cpu.step_cck(&mut bus);
    assert!(!r1);
    assert_eq!(cpu.state.micro.micro_step, 1);

    // Before CCK2 commit, Agnus DMA grabs Chip RAM
    bus.chip_ram_blocked = true;

    // Color Clock 2: CCK2 write blocked by DMA -> WaitState
    let r2 = cpu.step_cck(&mut bus);
    assert!(!r2);
    assert_eq!(cpu.state.micro.micro_step, 1);
    assert_ne!(bus.read_word_debug(0x003000), 0xBEEF);

    // Color Clock 3: still blocked -> WaitState
    let r3 = cpu.step_cck(&mut bus);
    assert!(!r3);
    assert_eq!(cpu.state.micro.micro_step, 1);
    assert_ne!(bus.read_word_debug(0x003000), 0xBEEF);

    // Agnus frees bus
    bus.chip_ram_blocked = false;

    // Color Clock 4: CCK2 succeeds, write commits to Chip RAM
    let r4 = cpu.step_cck(&mut bus);
    assert!(!r4);
    assert_eq!(cpu.state.micro.micro_step, 2);
    assert_eq!(bus.read_word_debug(0x003000), 0xBEEF);

    // Next Color Clocks: Prefetch next instruction word (CCK1 and CCK2)
    let r5 = cpu.step_cck(&mut bus);
    assert!(!r5);
    let r6 = cpu.step_cck(&mut bus);
    assert!(r6);

    assert_eq!(cpu.cycle_counter(), 12);
}

#[test]
fn test_archetype4_rmw_add_12_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: ADD.w D0, (A0) (0xD150)
    bus.write_word_debug(0x001000, 0xD150);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);
    bus.write_word_debug(0x004000, 0x0020);

    cpu.state.set_d_long(0, 0x0000_0015);
    cpu.state.set_a_long(0, 0x004000);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 12,
        "ADD.w Dx, (Ay) RMW must take exactly 12 CPU clocks (6 CCKs)"
    );
    assert_eq!(bus.read_word_debug(0x004000), 0x0035);
}

#[test]
fn test_archetype5_bcc_untaken_8_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: BEQ.s +4 (0x6704) with Z=0 (not taken)
    bus.write_word_debug(0x001000, 0x6704);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);

    // Clear Z flag in CCR
    cpu.state.set_ccr_z_only(false);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 8,
        "Bcc.s untaken must take exactly 8 CPU clocks (4 CCKs)"
    );
    assert_eq!(
        cpu.state.pc, 0x001006,
        "PC should fall through sequentially to next instruction"
    );
}

#[test]
fn test_archetype5_bcc_taken_10_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: BRA.s +4 (0x6004) -> branch target is 0x001000 + 2 + 4 = 0x001006
    bus.write_word_debug(0x001000, 0x6004);
    bus.write_word_debug(0x001002, 0x4E71);
    // Target instruction: NOP at 0x001006
    bus.write_word_debug(0x001006, 0x4E71);
    bus.write_word_debug(0x001008, 0x4E71);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 10,
        "Bcc.s taken / BRA.s must take exactly 10 CPU clocks (5 CCKs)"
    );
    assert_eq!(cpu.state.ir, 0x4E71, "Target opcode must be loaded into IR");
    assert_eq!(cpu.state.pc, 0x00100A, "PC must be primed at target + 4");
}

#[test]
fn test_archetype6_pea_12_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: PEA (A0) (0x4850)
    bus.write_word_debug(0x001000, 0x4850);
    bus.write_word_debug(0x001002, 0x4E71);
    bus.write_word_debug(0x001004, 0x4E71);

    cpu.state.set_a_long(0, 0x1234_5678);
    cpu.state.set_a_long(7, 0x006000);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 12,
        "PEA (An) must take exactly 12 CPU clocks (6 CCKs)"
    );
    assert_eq!(
        cpu.state.a_long(7),
        0x005FFC,
        "Stack pointer must decrement by 4"
    );

    let hi = bus.read_word_debug(0x005FFC);
    let lo = bus.read_word_debug(0x005FFE);
    let pushed = ((hi as u32) << 16) | (lo as u32);
    assert_eq!(
        pushed, 0x1234_5678,
        "Full 32-bit address must be pushed to stack"
    );
}

#[test]
fn test_archetype6_jsr_16_clocks() {
    let (mut cpu, mut bus) = setup_test_machine(0x001000);
    // Program: JSR (A0) (0x4E90)
    bus.write_word_debug(0x001000, 0x4E90);
    bus.write_word_debug(0x001002, 0x4E71); // Instruction following JSR

    // Target subroutine at 0x002000
    bus.write_word_debug(0x002000, 0x4E71); // Subroutine body (NOP)
    bus.write_word_debug(0x002002, 0x4E75); // RTS

    cpu.state.set_a_long(0, 0x002000);
    cpu.state.set_a_long(7, 0x006000);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 16,
        "JSR (An) must take exactly 16 CPU clocks (8 CCKs)"
    );
    assert_eq!(
        cpu.state.a_long(7),
        0x005FFC,
        "Stack pointer must decrement by 4 for return address"
    );

    let hi = bus.read_word_debug(0x005FFC);
    let lo = bus.read_word_debug(0x005FFE);
    let return_pc = ((hi as u32) << 16) | (lo as u32);
    assert_eq!(
        return_pc, 0x001002,
        "Return PC must point to next sequential instruction"
    );
    assert_eq!(
        cpu.state.ir, 0x4E71,
        "Subroutine first opcode must be loaded into IR"
    );
    assert_eq!(cpu.state.pc, 0x002004, "PC must be primed at target + 4");
}

#[test]
fn test_micro_step_predicates_and_work_detection() {
    fn dummy_bus(
        _cpu: &mut Cpu,
        _bus: &mut dyn physical_memory::AddressBus,
    ) -> physical_memory::BusResult<()> {
        physical_memory::BusResult::Ready(())
    }
    fn dummy_alu(_state: &mut cpu::CpuState, _src: u8, _dst: u8) {}

    let cck_step = cpu::MicroStep::cck(dummy_bus);
    assert!(
        cck_step.has_work(),
        "CCK step with base clocks must report has_work = true"
    );
    assert!(
        !cck_step.is_instantaneous(),
        "CCK step has 2 clocks, cannot be instantaneous"
    );

    let idle_step = cpu::MicroStep::cck_idle();
    assert!(
        idle_step.has_work(),
        "CCK idle step has 2 base clocks, reports has_work = true"
    );
    assert!(
        !idle_step.is_instantaneous(),
        "CCK idle step consumes clocks, cannot be instantaneous"
    );

    let alu_step = cpu::MicroStep::alu(dummy_alu);
    assert!(
        alu_step.has_work(),
        "ALU step has ALU callback, reports has_work = true"
    );
    assert!(
        alu_step.is_instantaneous(),
        "ALU step with 0 clocks must report is_instantaneous = true"
    );

    let empty_step = cpu::MicroStep {
        bus_fn: None,
        alu_fn: None,
        base_clocks: 0,
    };
    assert!(
        !empty_step.has_work(),
        "Empty step with 0 clocks and no callbacks must report has_work = false"
    );
    assert!(
        !empty_step.is_instantaneous(),
        "Empty step without ALU callback cannot be instantaneous"
    );
}

#[test]
fn test_condition_evaluation_and_defensive_bounds() {
    let state = cpu::CpuState::default();
    assert!(
        state.eval_condition(0x00),
        "Condition 0 (True) must be true"
    );
    assert!(
        !state.eval_condition(0x01),
        "Condition 1 (False) must be false"
    );
}

// Refusals target named accesses, including later accesses after successful work.
fn recording_machine(
    opcode: u16,
    extension: u16,
    script: Vec<(Access, usize)>,
) -> (Cpu, RecordingBus) {
    let (mut cpu, mut memory) = setup_test_machine(0x1000);
    memory.write_word_debug(0x1000, opcode);
    memory.write_word_debug(0x1002, extension);
    memory.write_word_debug(0x1004, 0x4E71);
    memory.write_word_debug(0x1006, 0x4E71);
    prime_prefetch(&mut cpu, &mut memory);
    (cpu, RecordingBus::new(memory, script))
}

#[test]
fn test_long_halves_and_address_updates_under_waits() {
    for (first, second) in [(0, 0), (1, 0), (3, 0), (0, 1), (0, 3), (1, 1), (3, 3)] {
        for opcode in [0x2010, 0x2020, 0x2080, 0x20C0, 0x2100] {
            let is_read = opcode == 0x2010 || opcode == 0x2020;
            let predecrement = opcode == 0x2020 || opcode == 0x2100;
            let script = if is_read {
                vec![
                    (Access::ReadWord(0x2000), first),
                    (Access::ReadWord(0x2002), second),
                    (Access::ReadWord(0x1004), 0),
                ]
            } else if predecrement {
                vec![
                    (Access::ReadWord(0x1004), 1),
                    (Access::WriteWord(0x2002, 0x5678), first),
                    (Access::WriteWord(0x2000, 0x1234), second),
                ]
            } else {
                vec![
                    (Access::WriteWord(0x2000, 0x1234), first),
                    (Access::WriteWord(0x2002, 0x5678), second),
                    (Access::ReadWord(0x1004), 0),
                ]
            };
            let (mut cpu, mut bus) = recording_machine(opcode, 0x4E71, script);
            cpu.state
                .set_a_long(0, if predecrement { 0x2004 } else { 0x2000 });
            cpu.state
                .set_d_long(0, if is_read { 0xDEAD_BEEF } else { 0x1234_5678 });
            bus.memory
                .write_word_debug(0x2000, if is_read { 0x1234 } else { 0xDEAD });
            bus.memory
                .write_word_debug(0x2002, if is_read { 0x5678 } else { 0xBEEF });
            run_instruction(&mut cpu, &mut bus, if opcode == 0x2020 { 14 } else { 12 });
            assert_eq!(cpu.state.d_long(0), 0x1234_5678);
            assert_eq!(bus.memory.read_word_debug(0x2000), 0x1234);
            assert_eq!(bus.memory.read_word_debug(0x2002), 0x5678);
            assert_eq!(
                cpu.state.a_long(0),
                if opcode == 0x20C0 { 0x2004 } else { 0x2000 }
            );
            assert_eq!(cpu.state.pc, 0x1006);
            assert_eq!(cpu.state.ir, 0x4E71);
        }
    }
}

#[test]
fn test_rmw_retries_do_not_repeat_alu_or_completed_read() {
    for (read_waits, write_waits) in [(0, 0), (1, 0), (3, 0), (0, 1), (0, 3), (1, 1), (3, 3)] {
        let script = vec![
            (Access::ReadWord(0x2000), read_waits),
            (Access::ReadWord(0x1004), 3),
            (Access::WriteWord(0x2000, 0x0035), write_waits),
        ];
        let (mut cpu, mut bus) = recording_machine(0xD150, 0x4E71, script);
        cpu.state.set_a_long(0, 0x2000);
        cpu.state.set_d_long(0, 0x15);
        bus.memory.write_word_debug(0x2000, 0x20);
        run_instruction(&mut cpu, &mut bus, 12);
        assert_eq!(bus.memory.read_word_debug(0x2000), 0x35);
        assert_eq!(cpu.state.d_long(0), 0x15);
        assert_eq!(cpu.state.ccr(), 0);
    }
}

#[test]
fn test_byte_lanes_and_postincrement_are_not_repeated() {
    for (opcode, reg, stride) in [
        (0x1018, 0, 1),
        (0x10C0, 0, 1),
        (0x101F, 7, 2),
        (0x1EC0, 7, 2),
    ] {
        for lane in [0, 1] {
            for waits in [0, 1, 3] {
                let is_read = opcode == 0x1018 || opcode == 0x101F;
                let addr = 0x2000 + lane;
                let access = if is_read {
                    Access::ReadByte(addr)
                } else {
                    Access::WriteByte(addr, 0xA5)
                };
                let (mut cpu, mut bus) = recording_machine(
                    opcode,
                    0x4E71,
                    vec![(access, waits), (Access::ReadWord(0x1004), 1)],
                );
                cpu.state.set_a_long(reg, addr);
                cpu.state.set_d_long(0, 0x1234_56A5);
                bus.memory.write_word_debug(0x2000, 0xA5A5);
                run_instruction(&mut cpu, &mut bus, 8);
                assert_eq!(cpu.state.a_long(reg), addr + stride);
                assert_eq!(cpu.state.d_long(0), 0x1234_56A5);
            }
        }
    }
}

#[test]
fn test_extension_and_branch_target_waits_preserve_pipeline() {
    for waits in [0, 1, 3] {
        let (mut cpu, mut bus) = recording_machine(
            0x3028,
            4,
            vec![
                (Access::ReadWord(0x1004), waits),
                (Access::ReadWord(0x2004), waits),
                (Access::ReadWord(0x1006), waits),
            ],
        );
        cpu.state.set_a_long(0, 0x2000);
        bus.memory.write_word_debug(0x2004, 0xCAFE);
        run_instruction(&mut cpu, &mut bus, 12);
        assert_eq!(cpu.state.d_word(0), 0xCAFE);
        assert_eq!(cpu.state.pc, 0x1008);
        let (mut cpu, mut bus) = recording_machine(
            0x6004,
            0x4E71,
            vec![
                (Access::ReadWord(0x1006), waits),
                (Access::ReadWord(0x1008), waits),
            ],
        );
        bus.memory.write_word_debug(0x1008, 0x4E75);
        run_instruction(&mut cpu, &mut bus, 10);
        assert_eq!(cpu.state.pc, 0x100A);
        assert_eq!(cpu.state.ir, 0x4E71);
        assert_eq!(cpu.state.prefetch, 0x4E75);
    }
}

#[test]
fn test_stack_push_pop_and_exception_waits_commit_once() {
    for waits in [0, 1, 3] {
        let (mut cpu, mut bus) = recording_machine(
            0x4850,
            0x4E71,
            vec![
                (Access::ReadWord(0x1004), waits),
                (Access::WriteWord(0x5FFC, 0x1234), waits),
                (Access::WriteWord(0x5FFE, 0x5678), waits),
            ],
        );
        cpu.state.set_a_long(0, 0x1234_5678);
        cpu.state.set_a_long(7, 0x6000);
        run_instruction(&mut cpu, &mut bus, 12);
        assert_eq!(cpu.state.a_long(7), 0x5FFC);
        assert_eq!(bus.memory.read_word_debug(0x5FFC), 0x1234);
        assert_eq!(bus.memory.read_word_debug(0x5FFE), 0x5678);

        let script = vec![
            (Access::ReadWord(0x2000), waits),
            (Access::WriteWord(0x5FFC, 0), waits),
            (Access::WriteWord(0x5FFE, 0x1002), waits),
            (Access::ReadWord(0x2002), waits),
        ];
        let (mut cpu, mut bus) = recording_machine(0x4E90, 0x4E71, script);
        cpu.state.set_a_long(0, 0x2000);
        cpu.state.set_a_long(7, 0x6000);
        bus.memory.write_word_debug(0x2000, 0x4E71);
        bus.memory.write_word_debug(0x2002, 0x4E75);
        run_instruction(&mut cpu, &mut bus, 16);
        assert_eq!(cpu.state.a_long(7), 0x5FFC);
        assert_eq!(cpu.state.pc, 0x2004);
        assert_eq!(bus.memory.read_word_debug(0x5FFE), 0x1002);

        for (opcode, status, expected_sr) in [
            (0x4E75, 0, 0x2700),
            (0x4E77, 0xFFFF, 0x271F),
            (0x4E73, 0x2700, 0x2700),
            (0x4E73, 0x0015, 0x0015),
        ] {
            let restores_status = opcode != 0x4E75;
            // Silicon reads PC high, status, PC low for RTR/RTE.
            let mut script = if restores_status {
                vec![
                    (Access::ReadWord(0x6002), waits),
                    (Access::ReadWord(0x6000), waits),
                    (Access::ReadWord(0x6004), waits),
                ]
            } else {
                vec![
                    (Access::ReadWord(0x6000), waits),
                    (Access::ReadWord(0x6002), waits),
                ]
            };
            script.extend([
                (Access::ReadWord(0x2000), waits),
                (Access::ReadWord(0x2002), waits),
            ]);
            let (mut cpu, mut bus) = recording_machine(opcode, 0x4E71, script);
            cpu.state.set_a_long(7, 0x6000);
            cpu.state.set_usp(0x7000);
            bus.memory.write_word_debug(0x6000, status);
            bus.memory
                .write_word_debug(0x6002, if restores_status { 0 } else { 0x2000 });
            bus.memory.write_word_debug(0x6004, 0x2000);
            bus.memory.write_word_debug(0x2000, 0x4E71);
            bus.memory.write_word_debug(0x2002, 0x4E75);
            run_instruction(&mut cpu, &mut bus, if restores_status { 20 } else { 16 });
            assert_eq!(cpu.state.sr(), expected_sr);
            assert_eq!(
                cpu.state.ssp(),
                if restores_status { 0x6006 } else { 0x6004 }
            );
            assert_eq!(cpu.state.usp(), 0x7000);
            assert_eq!(
                cpu.state.a_long(7),
                if expected_sr & 0x2000 == 0 {
                    0x7000
                } else {
                    cpu.state.ssp()
                }
            );
            assert_eq!(cpu.state.pc, 0x2004);
        }
        for (opcode, old_sr, return_pc, vector_addr) in [
            (0x4E40, 0x2700, 0x1002, 0x80),
            (0x4E73, 0x0000, 0x1000, 0x20),
        ] {
            let script = vec![
                (Access::WriteWord(0x5FFE, return_pc), waits),
                (Access::WriteWord(0x5FFA, old_sr), waits),
                (Access::WriteWord(0x5FFC, 0), waits),
                (Access::ReadWord(vector_addr), waits),
                (Access::ReadWord(vector_addr + 2), waits),
                (Access::ReadWord(0x2000), waits),
                (Access::ReadWord(0x2002), waits),
            ];
            let (mut cpu, mut bus) = recording_machine(opcode, 0x4E71, script);
            cpu.state.set_ssp(0x6000);
            cpu.state.set_usp(0x7000);
            cpu.state.set_sr(old_sr);
            cpu.state.instruction_pc = 0x1000;
            bus.memory.write_word_debug(vector_addr, 0);
            bus.memory.write_word_debug(vector_addr + 2, 0x2000);
            bus.memory.write_word_debug(0x2000, 0x4E71);
            bus.memory.write_word_debug(0x2002, 0x4E75);
            run_instruction(&mut cpu, &mut bus, 34);
            assert_eq!(cpu.state.ssp(), 0x5FFA);
            assert_eq!(cpu.state.usp(), 0x7000);
            assert_eq!(cpu.state.pc, 0x2004);
            assert_eq!(cpu.state.ir, 0x4E71);
        }
    }
}

#[test]
fn test_movep_waits_retain_byte_stride_and_shift_state() {
    for (opcode, is_read, bytes) in [
        (0x0108, true, 2),
        (0x0148, true, 4),
        (0x0188, false, 2),
        (0x01C8, false, 4),
    ] {
        for waits in [0, 1, 3] {
            let data = if bytes == 2 {
                vec![0x56, 0x78]
            } else {
                vec![0x12, 0x34, 0x56, 0x78]
            };
            let mut script = vec![(Access::ReadWord(0x1004), waits)];
            for (index, &byte) in data.iter().enumerate() {
                let addr = 0x2004 + index as u32 * 2;
                script.push((
                    if is_read {
                        Access::ReadByte(addr)
                    } else {
                        Access::WriteByte(addr, byte)
                    },
                    waits,
                ));
            }
            script.push((Access::ReadWord(0x1006), waits));
            let (mut cpu, mut bus) = recording_machine(opcode, 4, script);
            cpu.state.set_a_long(0, 0x2000);
            cpu.state
                .set_d_long(0, if is_read { 0xAAAA_0000 } else { 0x1234_5678 });
            for (index, &byte) in data.iter().enumerate() {
                bus.memory
                    .write_byte_debug(0x2004 + index as u32 * 2, if is_read { byte } else { 0xEE });
                bus.memory.write_byte_debug(0x2005 + index as u32 * 2, 0xCC);
            }
            run_instruction(&mut cpu, &mut bus, if bytes == 2 { 16 } else { 24 });
            assert_eq!(cpu.state.a_long(0), 0x2000);
            if is_read {
                assert_eq!(
                    cpu.state.d_long(0),
                    if bytes == 2 { 0xAAAA_5678 } else { 0x1234_5678 }
                );
            }
            for (index, &byte) in data.iter().enumerate() {
                assert_eq!(bus.memory.read_byte_debug(0x2004 + index as u32 * 2), byte);
                assert_eq!(bus.memory.read_byte_debug(0x2005 + index as u32 * 2), 0xCC);
            }
        }
    }
}

#[test]
fn test_movem_waits_preserve_register_word_and_terminal_progress() {
    // Two nonadjacent registers exercise mask scanning and later-register retries.
    for (opcode, is_read, predecrement, postincrement, long) in [
        (0x4890, false, false, false, false),
        (0x48D0, false, false, false, true),
        (0x48A0, false, true, false, false),
        (0x48E0, false, true, false, true),
        (0x4C90, true, false, false, false),
        (0x4CD0, true, false, false, true),
        (0x4C98, true, false, true, false),
        (0x4CD8, true, false, true, true),
    ] {
        for empty in [false, true] {
            for waits in [0, 1, 3] {
                let mut script = vec![(Access::ReadWord(0x1004), waits)];
                let mut words = Vec::new();
                let mut addr = if predecrement { 0x2010 } else { 0x2000 };
                if !empty {
                    let values = if predecrement {
                        [0x5566_7788u32, 0x1122_3344]
                    } else {
                        [0x1122_3344u32, 0x5566_7788]
                    };
                    for value in values {
                        let halves = if long {
                            if predecrement {
                                vec![value as u16, (value >> 16) as u16]
                            } else {
                                vec![(value >> 16) as u16, value as u16]
                            }
                        } else {
                            vec![value as u16]
                        };
                        for data in halves {
                            if predecrement {
                                addr -= 2;
                            }
                            words.push((addr, data));
                            script.push((
                                if is_read {
                                    Access::ReadWord(addr)
                                } else {
                                    Access::WriteWord(addr, data)
                                },
                                waits,
                            ));
                            if !predecrement {
                                addr += 2;
                            }
                        }
                    }
                }
                if is_read {
                    script.push((Access::ReadWord(addr), waits));
                }
                script.push((Access::ReadWord(0x1006), waits));
                let mask = if empty {
                    0
                } else if predecrement {
                    0xA000
                } else {
                    5
                };
                let (mut cpu, mut bus) = recording_machine(opcode, mask, script);
                cpu.state
                    .set_a_long(0, if predecrement { 0x2010 } else { 0x2000 });
                cpu.state.set_d_long(0, 0x1122_3344);
                cpu.state.set_d_long(2, 0x5566_7788);
                for &(target, data) in &words {
                    bus.memory
                        .write_word_debug(target, if is_read { data } else { 0xDEAD });
                }
                let base = if is_read { 12 } else { 8 };
                run_instruction(&mut cpu, &mut bus, base + 4 * words.len() as u64);
                for &(target, data) in &words {
                    assert_eq!(bus.memory.read_word_debug(target), data);
                }
                assert_eq!(
                    cpu.state.a_long(0),
                    if predecrement || postincrement {
                        addr
                    } else {
                        0x2000
                    }
                );
                if is_read && !empty {
                    assert_eq!(cpu.state.d_long(0), if long { 0x1122_3344 } else { 0x3344 });
                    assert_eq!(cpu.state.d_long(2), if long { 0x5566_7788 } else { 0x7788 });
                }
                assert_eq!(cpu.state.pc, 0x1008);
            }
        }
    }
}

#[test]
fn test_status_change_refill_waits_preserve_both_queue_words() {
    for waits in [0, 1, 3] {
        let (mut cpu, mut bus) = recording_machine(
            0x46C0,
            0x4E71,
            vec![
                (Access::ReadWord(0x1002), waits),
                (Access::ReadWord(0x1004), waits),
            ],
        );
        cpu.state.set_d_long(0, 0x2700);
        bus.memory.write_word_debug(0x1004, 0x4E75);
        run_instruction(&mut cpu, &mut bus, 12);
        assert_eq!(cpu.state.pc, 0x1006);
        assert_eq!(cpu.state.ir, 0x4E71);
        assert_eq!(cpu.state.prefetch, 0x4E75);
    }
}
