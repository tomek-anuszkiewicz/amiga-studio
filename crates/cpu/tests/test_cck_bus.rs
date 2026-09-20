#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cpu::{Cpu, MicroStep};
use physical_memory::{BusResult, PhysicalMemory};

#[test]
fn test_micro_state_initial_and_reset() {
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.micro.micro_step, 0);
    assert_eq!(cpu.state.micro.clocks_remaining, 0);

    cpu.state.micro.micro_step = 2;
    cpu.state.micro.clocks_remaining = 4;
    cpu.state.micro.reset();
    assert_eq!(cpu.state.micro.micro_step, 0);
    assert_eq!(cpu.state.micro.clocks_remaining, 0);
}

#[test]
fn test_cck_unblocked_read_cycle() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x55AA);

    let mut cpu = Cpu::new();
    cpu.state.micro.ea_addr = 0x002000;

    // CCK1 step: initiates bus read
    let res1 = cpu.step_bus_read_src_word(&mut bus);
    assert_eq!(res1, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.source as u16, 0x55AA);
}

#[test]
fn test_cck_read_contention_stall_at_cck1() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x1234);

    let mut cpu = Cpu::new();
    cpu.state.micro.ea_addr = 0x002000;

    // Agnus DMA occupies Chip RAM
    bus.chip_ram_blocked = true;

    // 1st attempt: CCK1 blocked by DMA -> WaitState
    let res1 = cpu.step_bus_read_src_word(&mut bus);
    assert_eq!(res1, BusResult::WaitState);

    // 2nd attempt: still blocked -> WaitState
    let res2 = cpu.step_bus_read_src_word(&mut bus);
    assert_eq!(res2, BusResult::WaitState);

    // Agnus frees bus
    bus.chip_ram_blocked = false;

    // 3rd attempt: CCK1 unblocked -> Ready
    let res3 = cpu.step_bus_read_src_word(&mut bus);
    assert_eq!(res3, BusResult::Ready(()));
    assert_eq!(cpu.state.micro.source as u16, 0x1234);
}

#[test]
fn test_cck_write_contention_stall_at_cck2() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    cpu.state.micro.ea_addr = 0x003000;
    cpu.state.micro.destination = 0xABCD;

    // Agnus DMA grabs Chip RAM before CCK2 commit
    bus.chip_ram_blocked = true;

    // CCK2: Gary withholds _DTACK -> CPU stalls at CCK2
    let res2 = cpu.step_bus_write_dst_word(&mut bus);
    assert_eq!(res2, BusResult::WaitState);

    // Bus freed
    bus.chip_ram_blocked = false;

    // CCK2 retry: write commits to memory
    let res3 = cpu.step_bus_write_dst_word(&mut bus);
    assert_eq!(res3, BusResult::Ready(()));

    // Verify written data
    assert_eq!(bus.read_word_debug(0x003000), 0xABCD);
}

#[test]
fn test_clocks_remaining_micro_stepping() {
    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();

    // 4 CPU clocks = 2 CCK steps
    static TEST_STEPS: [MicroStep; 1] = [MicroStep {
        bus_fn: None,
        alu_fn: None,
        base_clocks: 4,
    }];

    cpu.state.micro.current_steps = &TEST_STEPS;
    cpu.state.micro.micro_step = 0;
    cpu.state.micro.clocks_remaining = 0;

    let initial_cycles = cpu.cycle_counter();

    // 1st CCK step: consumes 2 clocks, 2 remaining
    let finished1 = cpu.step_cck(&mut bus);
    assert!(!finished1);
    assert_eq!(cpu.state.micro.clocks_remaining, 2);
    assert_eq!(cpu.cycle_counter(), initial_cycles + 2);

    // 2nd CCK step: consumes 2 clocks, completes step
    let finished2 = cpu.step_cck(&mut bus);
    assert!(finished2);
    assert_eq!(cpu.state.micro.clocks_remaining, 0);
    assert_eq!(cpu.cycle_counter(), initial_cycles + 4);
}

#[test]
fn test_cycle_counter_monotonic_accumulation() {
    let mut bus = PhysicalMemory::new();
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
    cpu.state.prefetch = 0x4E71;
    cpu.state.pc = 0x1004;

    // Step first NOP: should take 4 CPU clocks (2 CCK steps)
    let clocks1 = cpu.step_instruction(&mut bus);
    assert_eq!(clocks1, 4);
    assert_eq!(cpu.state.cycle_counter, 4);
    assert_eq!(cpu.cycle_counter(), 4);

    // Step second NOP: returns 4 clocks, cycle_counter accumulates to 8
    let clocks2 = cpu.step_instruction(&mut bus);
    assert_eq!(clocks2, 4);
    assert_eq!(cpu.state.cycle_counter, 8);
    assert_eq!(cpu.cycle_counter(), 8);

    // Manual reset of cycle counter
    cpu.reset_cycle_counter();
    assert_eq!(cpu.state.cycle_counter, 0);
    assert_eq!(cpu.cycle_counter(), 0);
}

#[test]
fn test_cpu_reset_status_and_vectors() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    // Setup initial SSP = $0007_0000 and PC = $0000_1000
    bus.write_word_debug(cpu::vector::addr(cpu::vector::RESET_SSP), 0x0007);
    bus.write_word_debug(cpu::vector::addr(cpu::vector::RESET_SSP) + 2, 0x0000);
    bus.write_word_debug(cpu::vector::addr(cpu::vector::RESET_PC), 0x0000);
    bus.write_word_debug(cpu::vector::addr(cpu::vector::RESET_PC) + 2, 0x1000);

    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    assert_eq!(cpu.state.sr(), cpu::SR_RESET_DEFAULT);
    assert_eq!(cpu.state.sr(), 0x2700);
    assert!(cpu.state.is_supervisor());
    assert_eq!(cpu.state.interrupt_mask(), 7);
    assert_eq!(cpu.state.ssp(), 0x0007_0000);
    assert_eq!(cpu.state.instruction_pc, 0x0000_1000);
    assert_eq!(cpu.state.pc, 0x0000_1004); // Prefetch pipeline primed 2 words forward

    // Vector addresses calculation
    assert_eq!(cpu::vector::addr(cpu::vector::RESET_SSP), 0x00);
    assert_eq!(cpu::vector::addr(cpu::vector::RESET_PC), 0x04);
    assert_eq!(cpu::vector::addr(cpu::vector::BUS_ERROR), 0x08);
    assert_eq!(cpu::vector::addr(cpu::vector::ADDRESS_ERROR), 0x0C);
    assert_eq!(cpu::vector::addr(cpu::vector::ZERO_DIVIDE), 0x14);
    assert_eq!(cpu::vector::addr(cpu::vector::CHK), 0x18);
    assert_eq!(cpu::vector::addr(cpu::vector::TRAPV), 0x1C);
    assert_eq!(cpu::vector::addr(cpu::vector::PRIVILEGE_VIOLATION), 0x20);
    assert_eq!(cpu::vector::addr(cpu::vector::AUTOVECTOR_BASE + 1), 0x64);
    assert_eq!(cpu::vector::addr(cpu::vector::TRAP_BASE), 0x80);
}

#[test]
fn test_function_code_constants_and_helpers() {
    use cpu::function_code;
    use cpu::micro::{data_fc, prog_fc};

    assert_eq!(function_code::USER_DATA, 1);
    assert_eq!(function_code::USER_PROGRAM, 2);
    assert_eq!(function_code::SUPERVISOR_DATA, 5);
    assert_eq!(function_code::SUPERVISOR_PROGRAM, 6);
    assert_eq!(function_code::CPU_SPACE, 7);

    let mut state = cpu::CpuState::default();
    // Default reset state is supervisor
    assert!(state.is_supervisor());
    assert_eq!(data_fc(&state), function_code::SUPERVISOR_DATA);
    assert_eq!(prog_fc(&state), function_code::SUPERVISOR_PROGRAM);

    // User mode (clear S bit)
    state.set_supervisor(false);
    assert!(!state.is_supervisor());
    assert_eq!(data_fc(&state), function_code::USER_DATA);
    assert_eq!(prog_fc(&state), function_code::USER_PROGRAM);
}

#[test]
fn test_cpu_reset_and_reset_warm() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    bus.write_word_debug(0, 0x0007);
    bus.write_word_debug(2, 0x0000);
    bus.write_word_debug(4, 0x0000);
    bus.write_word_debug(6, 0x2000);

    let mut cpu = Cpu::new();
    cpu.state.set_d_long(0, 0x12345678);
    cpu.state.set_a_long(0, 0x9ABCDEF0);
    cpu.state.set_usp(0x00054321);

    // Warm reset preserves D/A registers and USP
    cpu.reset_warm(&mut bus);
    assert_eq!(cpu.state.d_long(0), 0x12345678);
    assert_eq!(cpu.state.a_long(0), 0x9ABCDEF0);
    assert_eq!(cpu.state.usp(), 0x00054321);
    assert_eq!(cpu.state.ssp(), 0x00070000);
    assert_eq!(cpu.state.instruction_pc, 0x00002000);

    // Standard reset zeroes data/address registers and USP
    cpu.reset(&mut bus);
    assert_eq!(cpu.state.d_long(0), 0);
    assert_eq!(cpu.state.a_long(0), 0);
    assert_eq!(cpu.state.usp(), 0);
    assert_eq!(cpu.state.ssp(), 0x00070000);
    assert_eq!(cpu.state.instruction_pc, 0x00002000);
}

#[test]
fn test_cpu_reset_unaligned_pc_triggers_double_bus_fault() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    // Setup valid SSP = $0007_0000, but odd unaligned PC = $0000_1001
    bus.write_word_debug(0, 0x0007);
    bus.write_word_debug(2, 0x0000);
    bus.write_word_debug(4, 0x0000);
    bus.write_word_debug(6, 0x1001);

    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    // On physical M68000 silicon, unaligned PC at reset triggers an immediate Double Bus Fault
    assert!(
        cpu.state.halted,
        "CPU must be halted on Double Bus Fault when reset PC vector is unaligned"
    );
    assert_eq!(cpu.state.instruction_pc, 0x0000_1001);
    assert_eq!(cpu.state.pc, 0x0000_1001);
}
