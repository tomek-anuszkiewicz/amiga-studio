#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{Access, RecordingBus};
use cpu::{Cpu, MicroStep};
use physical_memory::{BusResult, PhysicalMemory};

#[test]
fn test_micro_state_initial_and_reset() {
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.micro().micro_step, 0);
    assert_eq!(cpu.state.micro().clocks_remaining, 0);

    cpu.state.micro_mut().micro_step = 2;
    cpu.state.micro_mut().clocks_remaining = 4;
    cpu.state.micro_mut().reset();
    assert_eq!(cpu.state.micro().micro_step, 0);
    assert_eq!(cpu.state.micro().clocks_remaining, 0);
}

#[test]
fn test_cck_unblocked_read_cycle() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x55AA);

    let mut cpu = Cpu::new();
    cpu.state.micro_mut().ea_addr = 0x002000;

    // CCK1 step: initiates bus read
    let res1 = cpu.step_bus_read_src_word(&mut bus);
    assert_eq!(res1, BusResult::Ready(()));
    assert_eq!(cpu.state.micro().source as u16, 0x55AA);
}

#[test]
fn test_cck_read_contention_stall_at_cck1() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x002000, 0x1234);

    let mut cpu = Cpu::new();
    cpu.state.micro_mut().ea_addr = 0x002000;

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
    assert_eq!(cpu.state.micro().source as u16, 0x1234);
}

#[test]
fn test_cck_write_contention_stall_at_cck2() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    cpu.state.micro_mut().ea_addr = 0x003000;
    cpu.state.micro_mut().destination = 0xABCD;

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

    cpu.state.micro_mut().current_steps = &TEST_STEPS;
    cpu.state.micro_mut().micro_step = 0;
    cpu.state.micro_mut().clocks_remaining = 0;

    let initial_cycles = cpu.cycle_counter();

    // 1st CCK step: consumes 2 clocks, 2 remaining
    let finished1 = cpu.step_cck(&mut bus);
    assert!(!finished1);
    assert_eq!(cpu.state.micro().clocks_remaining, 2);
    assert_eq!(cpu.cycle_counter(), initial_cycles + 2);

    // 2nd CCK step: consumes 2 clocks, completes step
    let finished2 = cpu.step_cck(&mut bus);
    assert!(finished2);
    assert_eq!(cpu.state.micro().clocks_remaining, 0);
    assert_eq!(cpu.cycle_counter(), initial_cycles + 4);
}

#[test]
fn test_cycle_counter_monotonic_accumulation() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.cycle_counter(), 0);
    assert_eq!(cpu.cycle_counter(), 0);

    // Write NOP ($4E71) instructions at $1000 and $1002
    bus.write_word_debug(0x1000, 0x4E71);
    bus.write_word_debug(0x1002, 0x4E71);
    bus.write_word_debug(0x1004, 0x4E71);

    cpu.state.set_pc(0x1000);
    cpu.state.set_ir(0x4E71);
    cpu.state.set_prefetch(0x4E71);
    cpu.state.set_pc(0x1004);

    // Step first NOP: should take 4 CPU clocks (2 CCK steps)
    let clocks1 = cpu.step_instruction(&mut bus);
    assert_eq!(clocks1, 4);
    assert_eq!(cpu.state.cycle_counter(), 4);
    assert_eq!(cpu.cycle_counter(), 4);

    // Step second NOP: returns 4 clocks, cycle_counter accumulates to 8
    let clocks2 = cpu.step_instruction(&mut bus);
    assert_eq!(clocks2, 4);
    assert_eq!(cpu.state.cycle_counter(), 8);
    assert_eq!(cpu.cycle_counter(), 8);

    // Manual reset of cycle counter
    cpu.reset_cycle_counter();
    assert_eq!(cpu.state.cycle_counter(), 0);
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
    assert_eq!(cpu.state.instruction_pc(), 0x0000_1000);
    assert_eq!(cpu.state.pc(), 0x0000_1004); // Prefetch pipeline primed 2 words forward

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
    assert_eq!(cpu.state.instruction_pc(), 0x00002000);

    // Standard reset zeroes data/address registers and USP
    cpu.reset(&mut bus);
    assert_eq!(cpu.state.d_long(0), 0);
    assert_eq!(cpu.state.a_long(0), 0);
    assert_eq!(cpu.state.usp(), 0);
    assert_eq!(cpu.state.ssp(), 0x00070000);
    assert_eq!(cpu.state.instruction_pc(), 0x00002000);
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
        cpu.state.is_halted(),
        "CPU must be halted on Double Bus Fault when reset PC vector is unaligned"
    );
    assert_eq!(cpu.state.instruction_pc(), 0x0000_1001);
    assert_eq!(cpu.state.pc(), 0x0000_1001);
}

// Each handler has its own WaitState branch. These tables cover those branches
// without replicating opcode or memory-schedule permutations.
fn transfer_cpu() -> Cpu {
    let mut cpu = Cpu::new();
    cpu.state.set_pc(0x1004);
    cpu.state.set_ir(0x4E71);
    cpu.state.set_prefetch(0x4E75);
    cpu.state.set_a_long(0, 0x2004);
    cpu.state.set_a_long(7, 0x6000);
    cpu.state.micro_mut().ea_addr = 0x2000;
    cpu.state.micro_mut().addr1 = 0x3000;
    cpu.state.micro_mut().addr2 = 0x4000;
    cpu.state.micro_mut().source = 0x1122_3344;
    cpu.state.micro_mut().destination = 0x5566_7788;
    cpu.state.micro_mut().ea_high = 0x1234_0000;
    cpu.state.micro_mut().ssp_base = 0x7000;
    cpu.state.micro_mut().fault_addr = 0x89AB_CDEF;
    cpu.state.micro_mut().info_word = 0x1234;
    cpu.state.micro_mut().irc = 0x4E70;
    cpu
}

type Handler = fn(&mut Cpu, &mut dyn physical_memory::AddressBus) -> BusResult<()>;
type Latch = fn(&Cpu) -> u32;

#[test]
fn test_read_handlers_retain_latches_until_ready() {
    let cases: &[(Handler, Access, Latch, u32)] = &[
        (
            Cpu::step_bus_read_src_byte,
            Access::ReadByte(0x2000 + 0),
            |cpu| cpu.state.micro().source,
            0xA1,
        ),
        (
            Cpu::step_bus_read_src_word,
            Access::ReadWord(0x2000 + 0),
            |cpu| cpu.state.micro().source,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_src_long_high,
            Access::ReadWord(0x2000 + 0),
            |cpu| cpu.state.micro().source,
            0xA1B2_0000,
        ),
        (
            Cpu::step_bus_read_src_long_low,
            Access::ReadWord(0x2000 + 2),
            |cpu| cpu.state.micro().source,
            0x1122A1B2,
        ),
        (
            Cpu::step_bus_read_src_byte,
            Access::ReadByte(0x2000 + 1),
            |cpu| cpu.state.micro().source,
            0xB2,
        ),
        (
            Cpu::step_bus_read_dst_byte,
            Access::ReadByte(0x2000 + 0),
            |cpu| cpu.state.micro().destination,
            0xA1,
        ),
        (
            Cpu::step_bus_read_dst_word,
            Access::ReadWord(0x2000 + 0),
            |cpu| cpu.state.micro().destination,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_dst_long_high,
            Access::ReadWord(0x2000 + 0),
            |cpu| cpu.state.micro().destination,
            0xA1B2_0000,
        ),
        (
            Cpu::step_bus_read_dst_long_low,
            Access::ReadWord(0x2000 + 2),
            |cpu| cpu.state.micro().destination,
            0x5566A1B2,
        ),
        (
            Cpu::step_bus_read_dst_byte,
            Access::ReadByte(0x2000 + 1),
            |cpu| cpu.state.micro().destination,
            0xB2,
        ),
        (
            Cpu::step_bus_read_addr1_byte,
            Access::ReadByte(0x3000 + 0),
            |cpu| cpu.state.micro().source,
            0xA1,
        ),
        (
            Cpu::step_bus_read_addr1_word,
            Access::ReadWord(0x3000 + 0),
            |cpu| cpu.state.micro().source,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_addr1_long_high,
            Access::ReadWord(0x3000 + 0),
            |cpu| cpu.state.micro().source,
            0xA1B2_0000,
        ),
        (
            Cpu::step_bus_read_addr1_long_low,
            Access::ReadWord(0x3000 + 2),
            |cpu| cpu.state.micro().source,
            0x1122A1B2,
        ),
        (
            Cpu::step_bus_read_addr1_byte,
            Access::ReadByte(0x3000 + 1),
            |cpu| cpu.state.micro().source,
            0xB2,
        ),
        (
            Cpu::step_bus_read_addr2_byte,
            Access::ReadByte(0x4000 + 0),
            |cpu| cpu.state.micro().destination,
            0xA1,
        ),
        (
            Cpu::step_bus_read_addr2_word,
            Access::ReadWord(0x4000 + 0),
            |cpu| cpu.state.micro().destination,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_addr2_long_high,
            Access::ReadWord(0x4000 + 0),
            |cpu| cpu.state.micro().destination,
            0xA1B2_0000,
        ),
        (
            Cpu::step_bus_read_addr2_long_low,
            Access::ReadWord(0x4000 + 2),
            |cpu| cpu.state.micro().destination,
            0x5566A1B2,
        ),
        (
            Cpu::step_bus_read_addr2_byte,
            Access::ReadByte(0x4000 + 1),
            |cpu| cpu.state.micro().destination,
            0xB2,
        ),
        (
            Cpu::step_bus_read_src_split_high,
            Access::ReadWord(0x2000),
            |cpu| cpu.state.micro().source,
            0xA1B2_3344,
        ),
        (
            Cpu::step_bus_read_dst_split_high,
            Access::ReadWord(0x2000),
            |cpu| cpu.state.micro().destination,
            0xA1B2_7788,
        ),
        (
            Cpu::step_fetch_extension_read,
            Access::ReadWord(0x1004),
            |cpu| cpu.state.prefetch() as u32,
            0xA1B2,
        ),
        (
            Cpu::step_prefetch_next_read,
            Access::ReadWord(0x1004),
            |cpu| cpu.state.micro().irc as u32,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_target_opcode_read,
            Access::ReadWord(0x2000),
            |cpu| cpu.state.micro().irc as u32,
            0xA1B2,
        ),
        (
            Cpu::step_prefetch_target_read,
            Access::ReadWord(0x2002),
            |cpu| cpu.state.prefetch() as u32,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_control_target_opcode_read,
            Access::ReadWord(0x2000),
            |cpu| cpu.state.micro().irc as u32,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_return_high,
            Access::ReadWord(0x6002),
            |cpu| cpu.state.micro().ea_high,
            0xA1B2_0000,
        ),
        (
            Cpu::step_bus_read_vector_high_read,
            Access::ReadWord(0x2000),
            |cpu| cpu.state.micro().ea_high,
            0xA1B2_0000,
        ),
        (
            Cpu::step_bus_read_vector_low_read,
            Access::ReadWord(0x2002),
            |cpu| cpu.state.micro().source,
            0xA1B2,
        ),
        (
            Cpu::step_bus_pop_stack_high_read,
            Access::ReadWord(0x6000),
            |cpu| cpu.state.micro().ea_high,
            0xA1B2_0000,
        ),
        (
            Cpu::step_bus_pop_stack_low_read,
            Access::ReadWord(0x6000),
            |cpu| cpu.state.micro().ea_addr,
            0x1234_A1B2,
        ),
        (
            Cpu::step_bus_pop_stack_sr_read,
            Access::ReadWord(0x6000),
            |cpu| cpu.state.micro().source,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_refill_first,
            Access::ReadWord(0x1002),
            |cpu| cpu.state.prefetch() as u32,
            0xA1B2,
        ),
        (
            Cpu::step_bus_read_refill_second,
            Access::ReadWord(0x1004),
            |cpu| cpu.state.micro().irc as u32,
            0xA1B2,
        ),
    ];
    for &(handler, access, latch, expected) in cases {
        for refusals in [0, 1, 3] {
            let mut cpu = transfer_cpu();
            let mut memory = PhysicalMemory::new();
            memory.map_chip_ram_to_low_memory();
            let addr = match access {
                Access::ReadByte(addr) | Access::ReadWord(addr) => addr,
                _ => panic!("invalid fixture access"),
            };
            memory.write_word_debug(addr & !1, 0xA1B2);
            cpu.state.micro_mut().ea_addr |= addr & 1;
            cpu.state.micro_mut().addr1 |= addr & 1;
            cpu.state.micro_mut().addr2 |= addr & 1;
            let mut bus = RecordingBus::new(memory, vec![(access, refusals)]);
            let before = cpu.state.clone();
            for _ in 0..refusals {
                assert_eq!(handler(&mut cpu, &mut bus), BusResult::WaitState);
                assert_eq!(
                    cpu.state, before,
                    "refused read changed CPU state: {access:?}"
                );
            }
            assert_eq!(handler(&mut cpu, &mut bus), BusResult::Ready(()));
            assert_eq!(latch(&cpu), expected, "wrong ready latch: {access:?}");
            bus.assert_complete();
        }
    }
}

#[test]
fn test_write_handlers_commit_once_after_release() {
    let cases: &[(Handler, Access)] = &[
        (
            Cpu::step_bus_write_tas_byte,
            Access::WriteByte(0x2000, 0x88),
        ),
        (
            Cpu::step_bus_write_dst_byte,
            Access::WriteByte(0x2000, 0x88),
        ),
        (
            Cpu::step_bus_write_dst_byte,
            Access::WriteByte(0x2001, 0x88),
        ),
        (
            Cpu::step_bus_write_dst_word,
            Access::WriteWord(0x2000, 0x7788),
        ),
        (
            Cpu::step_bus_write_dst_long_high,
            Access::WriteWord(0x2000, 0x5566),
        ),
        (
            Cpu::step_bus_write_dst_long_low,
            Access::WriteWord(0x2002, 0x7788),
        ),
        (
            Cpu::step_bus_write_dst_pd_word,
            Access::WriteWord(0x2000, 0x7788),
        ),
        (
            Cpu::step_bus_write_dst_pd_long_low,
            Access::WriteWord(0x2000, 0x7788),
        ),
        (
            Cpu::step_bus_write_dst_pd_long_high,
            Access::WriteWord(0x1FFE, 0x5566),
        ),
        (
            Cpu::step_bus_write_addr2_byte,
            Access::WriteByte(0x4000, 0x88),
        ),
        (
            Cpu::step_bus_write_addr2_byte,
            Access::WriteByte(0x4001, 0x88),
        ),
        (
            Cpu::step_bus_write_addr2_word,
            Access::WriteWord(0x4000, 0x7788),
        ),
        (
            Cpu::step_bus_write_addr2_pd_long_low,
            Access::WriteWord(0x4002, 0x7788),
        ),
        (
            Cpu::step_bus_write_addr2_pd_long_high,
            Access::WriteWord(0x4000, 0x5566),
        ),
        (
            Cpu::step_bus_push_stack_high_write,
            Access::WriteWord(0x6000, 0x5566),
        ),
        (
            Cpu::step_bus_push_stack_low_write,
            Access::WriteWord(0x6002, 0x7788),
        ),
        (
            Cpu::step_bus_write_trap_pclo_write,
            Access::WriteWord(0x6000 - 2, 0x3344),
        ),
        (
            Cpu::step_bus_write_trap_sr_write,
            Access::WriteWord(0x6000 - 6, 0x7788),
        ),
        (
            Cpu::step_bus_write_trap_pchi_write,
            Access::WriteWord(0x6000 - 4, 0x1122),
        ),
        (
            Cpu::step_bus_write_aerr_pclo_write,
            Access::WriteWord(0x7000 - 2, 0x3344),
        ),
        (
            Cpu::step_bus_write_aerr_sr_write,
            Access::WriteWord(0x7000 - 6, 0x7788),
        ),
        (
            Cpu::step_bus_write_aerr_pchi_write,
            Access::WriteWord(0x7000 - 4, 0x1122),
        ),
        (
            Cpu::step_bus_write_aerr_ir_write,
            Access::WriteWord(0x7000 - 8, 0x4E71),
        ),
        (
            Cpu::step_bus_write_aerr_addr_lo_write,
            Access::WriteWord(0x7000 - 10, 0xCDEF),
        ),
        (
            Cpu::step_bus_write_aerr_info_write,
            Access::WriteWord(0x7000 - 14, 0x1234),
        ),
        (
            Cpu::step_bus_write_aerr_addr_hi_write,
            Access::WriteWord(0x7000 - 12, 0x89AB),
        ),
    ];
    for &(handler, access) in cases {
        for refusals in [0, 1, 3] {
            let mut cpu = transfer_cpu();
            let mut memory = PhysicalMemory::new();
            memory.map_chip_ram_to_low_memory();
            let addr = match access {
                Access::WriteByte(addr, _) | Access::WriteWord(addr, _) => addr,
                _ => panic!("invalid fixture access"),
            };
            memory.write_word_debug(addr & !1, 0xDEAD);
            cpu.state.micro_mut().ea_addr |= addr & 1;
            cpu.state.micro_mut().addr1 |= addr & 1;
            cpu.state.micro_mut().addr2 |= addr & 1;
            let mut bus = RecordingBus::new(memory, vec![(access, refusals)]);
            let before = cpu.state.clone();
            for _ in 0..refusals {
                assert_eq!(handler(&mut cpu, &mut bus), BusResult::WaitState);
                assert_eq!(
                    cpu.state, before,
                    "refused write changed CPU state: {access:?}"
                );
                assert_eq!(bus.memory.read_word_debug(addr & !1), 0xDEAD);
            }
            assert_eq!(handler(&mut cpu, &mut bus), BusResult::Ready(()));
            match access {
                Access::WriteWord(_, data) => assert_eq!(bus.memory.read_word_debug(addr), data),
                Access::WriteByte(_, data) => {
                    assert_eq!(bus.memory.read_byte_debug(addr), data);
                    let neighbor = if addr & 1 == 0 { 0xAD } else { 0xDE };
                    assert_eq!(bus.memory.read_byte_debug(addr ^ 1), neighbor);
                }
                _ => panic!("invalid fixture access"),
            }
            bus.assert_complete();
        }
    }
}

#[test]
fn test_engine_read_and_write_phases_with_repeated_waits() {
    static READ: [MicroStep; 2] = [
        MicroStep::cck(Cpu::step_bus_read_src_word),
        MicroStep::cck_idle(),
    ];
    static WRITE: [MicroStep; 2] = [
        MicroStep::cck_idle(),
        MicroStep::cck(Cpu::step_bus_write_dst_word),
    ];
    for is_read in [true, false] {
        for refusals in [0, 1, 3] {
            let mut cpu = transfer_cpu();
            cpu.state.micro_mut().current_steps = if is_read { &READ } else { &WRITE };
            let mut memory = PhysicalMemory::new();
            memory.map_chip_ram_to_low_memory();
            memory.write_word_debug(0x2000, 0xDEAD);
            let access = if is_read {
                Access::ReadWord(0x2000)
            } else {
                Access::WriteWord(0x2000, 0x7788)
            };
            let mut bus = RecordingBus::new(memory, vec![(access, refusals)]);
            if !is_read {
                bus.memory.chip_ram_blocked = true;
                assert!(!cpu.step_cck(&mut bus));
                assert!(bus.attempts.is_empty(), "write CCK1 must leave memory idle");
                bus.memory.chip_ram_blocked = false;
            }
            for _ in 0..refusals {
                assert!(!cpu.step_cck(&mut bus));
                assert_eq!(cpu.state.micro().micro_step, u16::from(!is_read));
                assert_eq!(cpu.state.micro().clocks_remaining, 2);
                assert_eq!(cpu.state.micro().source, 0x1122_3344);
                assert_eq!(bus.memory.read_word_debug(0x2000), 0xDEAD);
            }
            let finished = cpu.step_cck(&mut bus);
            assert_eq!(finished, !is_read);
            if is_read {
                assert_eq!(cpu.state.micro().source, 0xDEAD);
                bus.memory.chip_ram_blocked = true;
                assert!(cpu.step_cck(&mut bus), "read CCK2 must leave memory idle");
            }
            bus.assert_complete();
            assert_eq!(cpu.cycle_counter(), 4 + 2 * bus.refusals);
        }
    }
}

#[test]
fn test_engine_does_not_repeat_entry_alu_during_write_retries() {
    fn increment(state: &mut cpu::CpuState, _: u8, _: u8) {
        state.set_d_long(0, state.d_long(0).wrapping_add(1));
    }
    static FUSED: [MicroStep; 1] = [MicroStep {
        bus_fn: Some(Cpu::step_bus_write_dst_word),
        alu_fn: Some(increment),
        base_clocks: 2,
    }];
    static ZERO_CLOCK: [MicroStep; 2] = [
        MicroStep::alu(increment),
        MicroStep::cck(Cpu::step_bus_write_dst_word),
    ];
    for steps in [&FUSED[..], &ZERO_CLOCK[..]] {
        let mut cpu = transfer_cpu();
        cpu.state.micro_mut().current_steps = steps;
        let mut memory = PhysicalMemory::new();
        memory.map_chip_ram_to_low_memory();
        let mut bus = RecordingBus::new(memory, vec![(Access::WriteWord(0x2000, 0x7788), 3)]);
        common::run_instruction(&mut cpu, &mut bus, 2);
        assert_eq!(cpu.state.d_long(0), 1);
    }
}

#[test]
fn test_successful_store_applies_deferred_address_and_pipeline_updates() {
    type Completion = fn(&mut cpu::CpuState);
    let cases: &[(Handler, Access, Completion)] = &[
        (
            Cpu::step_bus_write_dst_pd_word,
            Access::WriteWord(0x2000, 0x7788),
            |state| {
                state.set_ir(state.prefetch());
                state.set_prefetch(state.micro().irc);
                state.micro_mut().prefetch_retired = true;
            },
        ),
        (
            Cpu::step_bus_write_dst_pd_long_low,
            Access::WriteWord(0x2000, 0x7788),
            |state| state.set_a_long(0, 0x2000),
        ),
        (
            Cpu::step_bus_write_dst_pd_long_high,
            Access::WriteWord(0x1FFE, 0x5566),
            |state| {
                state.set_a_long(0, 0x1FFE);
                state.micro_mut().ea_addr = 0x1FFE;
                state.set_ir(state.prefetch());
                state.set_prefetch(state.micro().irc);
                state.micro_mut().prefetch_retired = true;
            },
        ),
        (
            Cpu::step_bus_write_trap_pchi_write,
            Access::WriteWord(0x5FFC, 0x1122),
            |state| state.set_a_long(7, 0x5FFA),
        ),
        (
            Cpu::step_bus_write_aerr_addr_hi_write,
            Access::WriteWord(0x6FF4, 0x89AB),
            |state| state.set_a_long(7, 0x6FF2),
        ),
    ];
    for &(handler, access, complete) in cases {
        let mut cpu = transfer_cpu();
        let mut expected = cpu.state.clone();
        complete(&mut expected);
        let mut memory = PhysicalMemory::new();
        memory.map_chip_ram_to_low_memory();
        let mut bus = RecordingBus::new(memory, vec![(access, 3)]);
        for _ in 0..3 {
            let before = cpu.state.clone();
            assert_eq!(handler(&mut cpu, &mut bus), BusResult::WaitState);
            assert_eq!(cpu.state, before);
        }
        assert_eq!(handler(&mut cpu, &mut bus), BusResult::Ready(()));
        assert_eq!(cpu.state, expected);
        bus.assert_complete();
    }
}
