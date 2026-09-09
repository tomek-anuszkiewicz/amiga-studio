use m68000::{Cpu, MicroStep};
use memory_bus::{BusResult, CckPhase, MemoryBus};

#[test]
fn test_micro_state_initial_and_reset() {
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.state.micro.clocks_remaining, -1);

    cpu.state.micro.phase = CckPhase::Cck2;
    cpu.state.micro.clocks_remaining = 4;
    cpu.state.micro.reset();
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.state.micro.clocks_remaining, -1);
}

#[test]
fn test_cck_unblocked_read_cycle() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x55AA);

    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);

    // CCK1 step
    let res1 = cpu.step_read_word_at(&mut bus, 0x002000);
    assert_eq!(res1, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);

    // CCK2 step
    let res2 = cpu.step_read_word_at(&mut bus, 0x002000);
    assert_eq!(res2, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.state.micro.source as u16, 0x55AA);
}

#[test]
fn test_cck_read_contention_stall_at_cck1() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x1234);

    let mut cpu = Cpu::new();

    // Agnus DMA occupies Chip RAM
    bus.lock_chip_ram();

    // 1st attempt: CCK1 blocked by DMA -> WaitState, CPU remains at CCK1
    let res1 = cpu.step_read_word_at(&mut bus, 0x002000);
    assert_eq!(res1, BusResult::WaitState);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);

    // 2nd attempt: still blocked -> WaitState
    let res2 = cpu.step_read_word_at(&mut bus, 0x002000);
    assert_eq!(res2, BusResult::WaitState);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);

    // Agnus frees bus
    bus.unlock_chip_ram();

    // 3rd attempt: CCK1 unblocked -> advances to CCK2
    let res3 = cpu.step_read_word_at(&mut bus, 0x002000);
    assert_eq!(res3, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);

    // 4th attempt: CCK2 completes transaction
    let res4 = cpu.step_read_word_at(&mut bus, 0x002000);
    assert_eq!(res4, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.state.micro.source as u16, 0x1234);
}

#[test]
fn test_cck_write_contention_stall_at_cck2() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();

    // CCK1: CPU outputs address/data onto bus (always succeeds for write)
    let res1 = cpu.step_write_word_at(&mut bus, 0x003000, 0xABCD);
    assert_eq!(res1, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);

    // Now Agnus DMA grabs Chip RAM before CCK2 commit
    bus.lock_chip_ram();

    // CCK2: Gary withholds _DTACK -> CPU stalls at CCK2
    let res2 = cpu.step_write_word_at(&mut bus, 0x003000, 0xABCD);
    assert_eq!(res2, BusResult::WaitState);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);

    // Bus freed
    bus.unlock_chip_ram();

    // CCK2 retry: write commits to memory
    let res3 = cpu.step_write_word_at(&mut bus, 0x003000, 0xABCD);
    assert_eq!(res3, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);

    // Verify written data
    assert_eq!(bus.read_word_debug(0x003000), 0xABCD);
}

#[test]
fn test_clocks_remaining_micro_stepping() {
    let mut bus = MemoryBus::new();
    let mut cpu = Cpu::new();

    // 4 CPU clocks = 2 CCK steps
    static TEST_STEPS: [MicroStep; 1] = [MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: None,
        base_clocks: 4,
    }];

    cpu.state.micro.current_steps = &TEST_STEPS;
    cpu.state.micro.micro_step = 0;
    cpu.state.micro.clocks_remaining = -1;

    let initial_cycles = cpu.cycle_counter();

    // 1st CCK step: consumes 2 clocks, 2 remaining
    let finished1 = cpu.step_cck(&mut bus);
    assert!(!finished1);
    assert_eq!(cpu.state.micro.clocks_remaining, 2);
    assert_eq!(cpu.cycle_counter(), initial_cycles + 2);

    // 2nd CCK step: consumes 2 clocks, completes step
    let finished2 = cpu.step_cck(&mut bus);
    assert!(finished2);
    assert_eq!(cpu.state.micro.clocks_remaining, -1);
    assert_eq!(cpu.cycle_counter(), initial_cycles + 4);
}

#[test]
fn test_cycle_counter_monotonic_accumulation() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.cycle_counter, 0);
    assert_eq!(cpu.cycle_counter(), 0);

    // Write NOP ($4E71) instructions at $1000 and $1002
    bus.write_word_debug(0x1000, 0x4E71);
    bus.write_word_debug(0x1002, 0x4E71);
    bus.write_word_debug(0x1004, 0x4E71);

    cpu.state.pc = 0x1000;
    cpu.state.ir = 0x4E71;
    cpu.state.prefetch[0] = 0x4E71;
    cpu.state.pc = 0x1004;

    // Step first NOP: should take 4 CPU clocks (2 CCK steps)
    let clocks1 = cpu.step_instruction(&mut bus);
    assert_eq!(clocks1, 4);
    assert_eq!(cpu.instruction_clocks, 4);
    assert_eq!(cpu.state.cycle_counter, 4);
    assert_eq!(cpu.cycle_counter(), 4);

    // Step second NOP: instruction_clocks resets to 4, cycle_counter accumulates to 8
    let clocks2 = cpu.step_instruction(&mut bus);
    assert_eq!(clocks2, 4);
    assert_eq!(cpu.instruction_clocks, 4);
    assert_eq!(cpu.state.cycle_counter, 8);
    assert_eq!(cpu.cycle_counter(), 8);

    // Manual reset of cycle counter
    cpu.reset_cycle_counter();
    assert_eq!(cpu.state.cycle_counter, 0);
    assert_eq!(cpu.cycle_counter(), 0);
}

