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

use cpu::Cpu;
use physical_memory::PhysicalMemory;

/// Helper to set up a test CPU and PhysicalMemory with mapped Chip RAM and initialized prefetch pipeline
fn setup_test_machine(base_pc: u32) -> (Cpu, PhysicalMemory) {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    // Default stack pointer to 0x008000
    cpu.state.write_a(7, 0x008000);
    cpu.state.pc = base_pc;
    (cpu, bus)
}

/// Initializes CPU prefetch queue from memory at `base_pc`
fn prime_prefetch(cpu: &mut Cpu, bus: &mut PhysicalMemory) {
    let pc = cpu.state.pc;
    cpu.state.ir = bus.read_word_debug(pc);
    cpu.state.prefetch[0] = bus.read_word_debug(pc.wrapping_add(2));
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
        cpu.state.sr & 0x1F,
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

    cpu.state.write_a(0, 0x002000);
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

    cpu.state.write_a(0, 0x002000);
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
    cpu.state.write_a(0, 0x003000);
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
    cpu.state.write_a(0, 0x003000);
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
    cpu.state.write_a(0, 0x004000);
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
    cpu.state.sr &= !0x04;
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

    cpu.state.write_a(0, 0x1234_5678);
    cpu.state.write_a(7, 0x006000);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 12,
        "PEA (An) must take exactly 12 CPU clocks (6 CCKs)"
    );
    assert_eq!(
        cpu.state.read_a(7),
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

    cpu.state.write_a(0, 0x002000);
    cpu.state.write_a(7, 0x006000);
    prime_prefetch(&mut cpu, &mut bus);

    let clocks = cpu.step_instruction(&mut bus);
    assert_eq!(
        clocks, 16,
        "JSR (An) must take exactly 16 CPU clocks (8 CCKs)"
    );
    assert_eq!(
        cpu.state.read_a(7),
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
