use m68000::{Cpu, StepResult};
use memory_bus::{function_code, BusAccessSize, BusCycle, CckPhase, MemoryBus};

#[test]
fn test_micro_state_initial_and_reset() {
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert!(!cpu.state.micro.is_bus_busy());
    assert_eq!(cpu.state.micro.internal_clocks, 0);
    assert_eq!(cpu.wait_cycles, 0);

    cpu.state.micro.phase = CckPhase::Cck2;
    cpu.state.micro.internal_clocks = 4;
    cpu.state.micro.reset();
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.state.micro.internal_clocks, 0);
}

#[test]
fn test_cck_unblocked_read_cycle() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x55AA);

    let mut cpu = Cpu::new();
    let read_cycle = BusCycle::new_read(0x002000, BusAccessSize::Word, function_code::USER_PROGRAM);
    cpu.initiate_bus_cycle(read_cycle);

    assert!(cpu.state.micro.is_bus_busy());
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);

    // CCK1 step
    let res1 = cpu.step_cck(&mut bus);
    assert_eq!(res1, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);
    assert!(cpu.state.micro.is_bus_busy());
    assert_eq!(cpu.wait_cycles, 0);

    // CCK2 step
    let res2 = cpu.step_cck(&mut bus);
    assert_eq!(res2, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert!(!cpu.state.micro.is_bus_busy());
    assert_eq!(cpu.wait_cycles, 0);
}

#[test]
fn test_cck_read_contention_stall_at_cck1() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x1234);

    let mut cpu = Cpu::new();
    let read_cycle = BusCycle::new_read(0x002000, BusAccessSize::Word, function_code::USER_DATA);
    cpu.initiate_bus_cycle(read_cycle);

    // Agnus DMA occupies Chip RAM
    bus.lock_chip_ram();

    // 1st attempt: CCK1 blocked by DMA -> WaitState, CPU remains at CCK1
    let res1 = cpu.step_cck(&mut bus);
    assert_eq!(res1, StepResult::WaitState);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.wait_cycles, 1);

    // 2nd attempt: still blocked -> WaitState, wait_cycles increments
    let res2 = cpu.step_cck(&mut bus);
    assert_eq!(res2, StepResult::WaitState);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.wait_cycles, 2);

    // Agnus frees bus
    bus.unlock_chip_ram();

    // 3rd attempt: CCK1 unblocked -> advances to CCK2
    let res3 = cpu.step_cck(&mut bus);
    assert_eq!(res3, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);
    assert_eq!(cpu.wait_cycles, 2);

    // 4th attempt: CCK2 completes transaction
    let res4 = cpu.step_cck(&mut bus);
    assert_eq!(res4, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert!(!cpu.state.micro.is_bus_busy());
    assert_eq!(cpu.wait_cycles, 2);
}

#[test]
fn test_cck_write_contention_stall_at_cck2() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    let write_cycle = BusCycle::new_write(
        0x003000,
        0xABCD,
        BusAccessSize::Word,
        function_code::USER_DATA,
    );
    cpu.initiate_bus_cycle(write_cycle);

    // CCK1: CPU outputs address/data onto bus (always succeeds for write)
    let res1 = cpu.step_cck(&mut bus);
    assert_eq!(res1, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);
    assert_eq!(cpu.wait_cycles, 0);

    // Now Agnus DMA grabs Chip RAM before CCK2 commit
    bus.lock_chip_ram();

    // CCK2: Gary withholds _DTACK -> CPU stalls at CCK2
    let res2 = cpu.step_cck(&mut bus);
    assert_eq!(res2, StepResult::WaitState);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck2);
    assert_eq!(cpu.wait_cycles, 1);

    // Bus freed
    bus.unlock_chip_ram();

    // CCK2 retry: write commits to memory
    let res3 = cpu.step_cck(&mut bus);
    assert_eq!(res3, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert!(!cpu.state.micro.is_bus_busy());
    assert_eq!(cpu.wait_cycles, 1);

    // Verify written data
    assert_eq!(bus.read_word_debug(0x003000), 0xABCD);
}

#[test]
fn test_internal_clocks_stepping() {
    let mut bus = MemoryBus::new();
    let mut cpu = Cpu::new();

    cpu.state.micro.internal_clocks = 4; // 4 CPU clocks = 2 CCKs

    let res1 = cpu.step_cck(&mut bus);
    assert_eq!(res1, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.internal_clocks, 2);

    let res2 = cpu.step_cck(&mut bus);
    assert_eq!(res2, StepResult::StepCompleted);
    assert_eq!(cpu.state.micro.internal_clocks, 0);
}
