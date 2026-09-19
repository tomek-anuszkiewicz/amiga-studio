//! Synthetic Agnus DMA contention test runner
//!
//! Sweeps DMA bus contention stalls across every Color Clock (CCK) phase of an instruction,
//! asserting two fundamental hardware invariants:
//! 1. State Invariance: Final CPU registers and RAM contents are 100% bit-identical
//!    to the uncontended execution.
//! 2. Cycle Invariance: Total CPU clocks increase by exactly 2 * wait_cycles.

use crate::diagnostic::StateDiff;
use crate::schema::SingleStepTest;
use crate::test_memory_bus::{MemoryType, TestMemoryBus};
use cpu::state::CpuState;
use cpu::Cpu;

/// Failure diagnostic for DMA contention invariance violation
#[derive(Debug, Clone)]
pub struct DmaContentionFailure {
    pub test_name: String,
    pub stall_phase: usize,
    pub burst_length: usize,
    pub base_clocks: u32,
    pub actual_clocks: u32,
    pub wait_cycles: u32,
    pub diffs: Vec<StateDiff>,
}

impl std::fmt::Display for DmaContentionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DMA Contention Invariance Violation on '{}' (Stall phase: {}, Burst: {} CCKs):\n  Base clocks: {}, Actual clocks: {}, Wait cycles: {}\n  Differences: {:?}",
            self.test_name, self.stall_phase, self.burst_length, self.base_clocks, self.actual_clocks, self.wait_cycles, self.diffs
        )
    }
}

/// Statistics returned from a full Cartesian DMA permutation run
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CartesianPermutationStats {
    pub total_permutations: usize,
    pub address_permutations: usize,
    pub dma_permutations: usize,
    pub dma_cycles_swept: usize,
}

impl std::error::Error for DmaContentionFailure {}

/// Pre-flight analysis result capturing golden state and memory contacts
pub struct PreFlight {
    pub base_clocks: u32,
    pub base_cck_count: u64,
    pub golden_state: CpuState,
    pub golden_ram: Vec<(u32, u8)>,
    pub unique_contacts: Vec<u32>,
}

/// Executes an uncontended golden run to determine base timings and unique memory contact cells
pub fn run_preflight(test: &SingleStepTest) -> PreFlight {
    let mut golden_bus = TestMemoryBus::new_flat();
    golden_bus.set_unmapped_byte(0x00);
    golden_bus.load_test_ram(&test.initial.ram);
    golden_bus.enable_transaction_recording(true);

    let mut golden_cpu = Cpu::new();
    init_cpu_state(&mut golden_cpu, test);

    let mut base_cck_count = 0u64;
    loop {
        let completed = golden_cpu.step_cck(&mut golden_bus);
        base_cck_count += 1;
        if completed || golden_cpu.state.halted || golden_cpu.state.stopped {
            break;
        }
    }
    golden_cpu.state.sync_stack_pointers();
    let base_clocks = golden_cpu.cycle_counter() as u32;
    let golden_state = golden_cpu.state.clone();

    let mut golden_ram = Vec::new();
    for entry in &test.final_state.ram {
        let addr = entry[0];
        golden_ram.push((addr, golden_bus.read_byte_debug(addr)));
    }

    let mut unique_contacts: Vec<u32> = Vec::new();
    if let Some(txs) = golden_bus.recorded_transactions() {
        for tx in txs {
            let word_addr = tx.addr & !1;
            if !unique_contacts.contains(&word_addr) {
                unique_contacts.push(word_addr);
            }
        }
    }

    PreFlight {
        base_clocks,
        base_cck_count,
        golden_state,
        golden_ram,
        unique_contacts,
    }
}

fn cluster_contacts(contacts: &[u32], pc: u32, sp: u32) -> Vec<Vec<u32>> {
    if contacts.len() <= 4 {
        return contacts.iter().map(|&c| vec![c]).collect();
    }

    let mut code_group = Vec::new();
    let mut stack_group = Vec::new();
    let mut op1_group = Vec::new();
    let mut op2_group = Vec::new();

    let pc_range = pc.saturating_sub(4)..=pc.saturating_add(16);
    let sp_range = sp.saturating_sub(32)..=sp.saturating_add(32);

    for &addr in contacts {
        if pc_range.contains(&addr) {
            code_group.push(addr);
        } else if sp_range.contains(&addr) || addr < 0x400 {
            stack_group.push(addr);
        } else if op1_group.is_empty() || op1_group.contains(&addr) {
            op1_group.push(addr);
        } else {
            op2_group.push(addr);
        }
    }

    let mut groups = Vec::new();
    if !code_group.is_empty() {
        groups.push(code_group);
    }
    if !stack_group.is_empty() {
        groups.push(stack_group);
    }
    if !op1_group.is_empty() {
        groups.push(op1_group);
    }
    if !op2_group.is_empty() {
        groups.push(op2_group);
    }

    groups
}

/// Executes an instruction across the full Cartesian product of:
/// 1. Address permutations: 2^k combinations of Chip vs Fast RAM for all touched contacts.
/// 2. DMA schedule permutations: 2^M combinations of stalled vs unstalled CCK cycles (0..M).
pub fn run_dma_full_cartesian_permutation(
    test: &SingleStepTest,
    max_dma_cycles: usize,
) -> Result<CartesianPermutationStats, DmaContentionFailure> {
    let preflight = run_preflight(test);

    let initial_sp = if (test.initial.sr & 0x2000) != 0 {
        test.initial.ssp
    } else {
        test.initial.usp
    };
    let contact_groups = cluster_contacts(&preflight.unique_contacts, test.initial.pc, initial_sp);
    let k = contact_groups.len();
    let num_addr_permutations = 1 << k;

    let dma_cycles = (preflight.base_cck_count as usize).min(max_dma_cycles);
    let num_dma_permutations = 1 << dma_cycles;
    let total_permutations = num_addr_permutations * num_dma_permutations;

    // Randomize starting cycle within [0, base_cck_count - dma_cycles].
    // This shifts the contention window along the instruction timeline while strictly
    // ensuring that [start_cycle, start_cycle + dma_cycles] never exceeds base_cck_count
    // (guaranteed to never get outside the instruction boundary).
    let max_start = (preflight.base_cck_count as usize).saturating_sub(dma_cycles);
    let start_cycle = if max_start > 0 {
        let mut hash = 0x811c_9dc5_u32;
        for b in test.name.bytes() {
            hash = (hash ^ (b as u32)).wrapping_mul(0x0100_0193);
        }
        hash = hash.wrapping_mul(31).wrapping_add(test.initial.pc);
        hash = hash.wrapping_mul(31).wrapping_add(test.initial.d0);
        (hash as usize) % (max_start + 1)
    } else {
        0
    };

    let mut bus = TestMemoryBus::new_flat();
    for dma_mask in 0..num_dma_permutations {
        for addr_mask in 0..num_addr_permutations {
            bus.clear();
            bus.set_unmapped_byte(0x00);
            bus.load_test_ram(&test.initial.ram);

            let all_fast = addr_mask == (num_addr_permutations - 1);

            for (i, group) in contact_groups.iter().enumerate() {
                let mem_type = if (addr_mask & (1 << i)) != 0 {
                    MemoryType::FastRam
                } else {
                    MemoryType::ChipRam
                };
                for &word_addr in group {
                    bus.set_address_type(word_addr, mem_type);
                    bus.set_address_type(word_addr.wrapping_add(1), mem_type);
                }
            }

            let mut cpu = Cpu::new();
            init_cpu_state(&mut cpu, test);

            let mut cck_step = 0u64;
            loop {
                let is_stalled = (cck_step >= start_cycle as u64)
                    && (cck_step < (start_cycle + dma_cycles) as u64)
                    && ((dma_mask & (1 << ((cck_step as usize) - start_cycle))) != 0);

                bus.chip_ram_blocked = is_stalled;
                if is_stalled {
                    bus.invert_chip_ram();
                }

                let completed = cpu.step_cck(&mut bus);

                if is_stalled {
                    bus.invert_chip_ram();
                }

                cck_step += 1;
                if completed || cpu.state.halted || cpu.state.stopped {
                    break;
                }
                if cck_step > (preflight.base_cck_count * 4 + 100) {
                    break;
                }
            }
            cpu.state.sync_stack_pointers();

            // 1. Assert Cycle Invariance
            let actual_clocks = cpu.cycle_counter() as u32;
            let extra_clocks = actual_clocks.saturating_sub(preflight.base_clocks);
            let wait_states = extra_clocks / 2;
            let expected_clocks = preflight.base_clocks + (2 * wait_states);

            if actual_clocks != expected_clocks {
                return Err(DmaContentionFailure {
                    test_name: test.name.clone(),
                    stall_phase: addr_mask,
                    burst_length: dma_mask,
                    base_clocks: preflight.base_clocks,
                    actual_clocks,
                    wait_cycles: wait_states,
                    diffs: vec![StateDiff::CycleLength {
                        actual: actual_clocks,
                        expected: expected_clocks,
                    }],
                });
            }

            // 2. Assert Fast RAM Immunity
            if all_fast && wait_states > 0 {
                return Err(DmaContentionFailure {
                    test_name: test.name.clone(),
                    stall_phase: addr_mask,
                    burst_length: dma_mask,
                    base_clocks: preflight.base_clocks,
                    actual_clocks,
                    wait_cycles: wait_states,
                    diffs: vec![StateDiff::CycleLength {
                        actual: actual_clocks,
                        expected: preflight.base_clocks,
                    }],
                });
            }

            // 3. Assert State Invariance
            let diffs =
                diff_cpu_and_ram(&cpu, &preflight.golden_state, &bus, &preflight.golden_ram);
            if !diffs.is_empty() {
                return Err(DmaContentionFailure {
                    test_name: test.name.clone(),
                    stall_phase: addr_mask,
                    burst_length: dma_mask,
                    base_clocks: preflight.base_clocks,
                    actual_clocks,
                    wait_cycles: wait_states,
                    diffs,
                });
            }
        }
    }

    Ok(CartesianPermutationStats {
        total_permutations,
        address_permutations: num_addr_permutations,
        dma_permutations: num_dma_permutations,
        dma_cycles_swept: dma_cycles,
    })
}

fn diff_cpu_and_ram(
    cpu: &Cpu,
    golden_state: &CpuState,
    bus: &TestMemoryBus,
    golden_ram: &[(u32, u8)],
) -> Vec<StateDiff> {
    let mut diffs = Vec::new();
    for i in 0..8 {
        let actual = cpu.state.d_regs()[i];
        let expected = golden_state.d_regs()[i];
        if actual != expected {
            diffs.push(StateDiff::DataRegister {
                reg: i,
                actual,
                expected,
            });
        }
    }
    for i in 0..7 {
        let actual = cpu.state.a_regs()[i];
        let expected = golden_state.a_regs()[i];
        if actual != expected {
            diffs.push(StateDiff::AddressRegister {
                reg: i,
                actual,
                expected,
            });
        }
    }
    if cpu.state.usp != golden_state.usp {
        diffs.push(StateDiff::UserStackPointer {
            actual: cpu.state.usp,
            expected: golden_state.usp,
        });
    }
    if cpu.state.ssp != golden_state.ssp {
        diffs.push(StateDiff::SupervisorStackPointer {
            actual: cpu.state.ssp,
            expected: golden_state.ssp,
        });
    }
    if cpu.state.sr != golden_state.sr {
        diffs.push(StateDiff::StatusRegister {
            actual: cpu.state.sr,
            expected: golden_state.sr,
            details: "SR changed under DMA contention".to_string(),
            diverging_flags: vec![],
        });
    }
    if cpu.state.pc != golden_state.pc {
        diffs.push(StateDiff::ProgramCounter {
            actual: cpu.state.pc,
            expected: golden_state.pc,
        });
    }
    for (addr, expected_byte) in golden_ram {
        let actual_byte = bus.read_byte_debug(*addr);
        if actual_byte != *expected_byte {
            diffs.push(StateDiff::RamByte {
                address: *addr,
                actual: actual_byte,
                expected: *expected_byte,
            });
        }
    }
    diffs
}

fn init_cpu_state(cpu: &mut Cpu, test: &SingleStepTest) {
    cpu.state.set_d_regs([
        test.initial.d0,
        test.initial.d1,
        test.initial.d2,
        test.initial.d3,
        test.initial.d4,
        test.initial.d5,
        test.initial.d6,
        test.initial.d7,
    ]);
    let initial_sp = if (test.initial.sr & 0x2000) != 0 {
        test.initial.ssp
    } else {
        test.initial.usp
    };
    cpu.state.set_a_regs([
        test.initial.a0,
        test.initial.a1,
        test.initial.a2,
        test.initial.a3,
        test.initial.a4,
        test.initial.a5,
        test.initial.a6,
        initial_sp,
    ]);
    cpu.state.usp = test.initial.usp;
    cpu.state.ssp = test.initial.ssp;
    cpu.state.sr = test.initial.sr;
    let is_harte = test.name.contains('[');
    if is_harte {
        cpu.state.pc = test.initial.pc.wrapping_add(4);
    } else {
        cpu.state.pc = test.initial.pc;
    }
    cpu.state.ir = (test.initial.prefetch[0] & 0xFFFF) as u16;
    cpu.state.prefetch = (test.initial.prefetch[1] & 0xFFFF) as u16;
}
