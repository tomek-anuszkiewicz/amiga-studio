use m68000::Cpu;
use memory_bus::{BusResult, CckPhase, MemoryBus};

#[test]
fn test_micro_state_initial_and_reset() {
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.micro.phase, CckPhase::Cck1);
    assert_eq!(cpu.state.micro.internal_clocks, 0);

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
fn test_internal_clocks_stepping() {
    let mut bus = MemoryBus::new();
    let mut cpu = Cpu::new();

    cpu.state.micro.internal_clocks = 4; // 4 CPU clocks = 2 CCKs

    let res1 = cpu.step_cck(&mut bus);
    assert!(!res1);
    assert_eq!(cpu.state.micro.internal_clocks, 2);

    let res2 = cpu.step_cck(&mut bus);
    assert!(!res2);
    assert_eq!(cpu.state.micro.internal_clocks, 0);
}
