//! Synthetic Agnus DMA contention test runner
//!
//! Sweeps DMA bus contention stalls across every Color Clock (CCK) phase of an instruction,
//! asserting two fundamental hardware invariants:
//! 1. State Invariance: Final CPU registers and RAM contents are 100% bit-identical
//!    to the uncontended execution.
//! 2. Cycle Invariance: Total CPU clocks increase by exactly 2 * wait_cycles.

use crate::diagnostic::StateDiff;
use crate::schema::SingleStepTest;
use m68000::{Cpu, StepResult};
use memory_bus::MemoryBus;

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

impl std::error::Error for DmaContentionFailure {}

/// Executes a SingleStepTest with DMA stalls swept across all CCK phases of instruction execution
pub fn run_dma_contention_sweep(
    test: &SingleStepTest,
    max_phases: usize,
) -> Result<(), DmaContentionFailure> {
    // 1. Establish golden baseline without contention
    let mut golden_bus = MemoryBus::new_test();
    golden_bus.set_unmapped_byte(0x00);
    golden_bus.load_test_ram(&test.initial.ram);

    let mut golden_cpu = Cpu::new();
    init_cpu_state(&mut golden_cpu, test);

    let mut base_cck_count = 0;
    loop {
        let res = golden_cpu.step_cck(&mut golden_bus);
        base_cck_count += 1;
        if res.is_completed() || res == StepResult::Halted || res == StepResult::Stopped {
            break;
        }
    }
    let base_clocks = golden_cpu.instruction_clocks;
    let golden_state = golden_cpu.state.clone();

    // Collect golden modified RAM
    let mut golden_ram = Vec::new();
    for entry in &test.final_state.ram {
        let addr = entry[0] & 0x00FF_FFFF;
        golden_ram.push((addr, golden_bus.read_byte_debug(addr)));
    }

    // 2. Sweep single-cycle DMA stalls across all phases: 0..base_cck_count
    let sweep_limit = base_cck_count.min(max_phases);
    for stall_phase in 0..sweep_limit {
        let mut bus = MemoryBus::new_test();
        bus.set_unmapped_byte(0x00);
        bus.load_test_ram(&test.initial.ram);

        let mut cpu = Cpu::new();
        init_cpu_state(&mut cpu, test);

        let mut cck_step = 0;
        loop {
            // Inject 1-CCK Chip RAM DMA stall at target phase
            bus.chip_ram_blocked = cck_step == stall_phase;

            let res = cpu.step_cck(&mut bus);
            cck_step += 1;
            if res.is_completed() || res == StepResult::Halted || res == StepResult::Stopped {
                break;
            }
            if cck_step > (base_cck_count * 4 + 100) {
                // Watchdog threshold
                break;
            }
        }

        // Verify Invariant 1: Cycle Invariance
        let expected_clocks = base_clocks + (2 * cpu.wait_cycles);
        if cpu.instruction_clocks != expected_clocks {
            return Err(DmaContentionFailure {
                test_name: test.name.clone(),
                stall_phase,
                burst_length: 1,
                base_clocks,
                actual_clocks: cpu.instruction_clocks,
                wait_cycles: cpu.wait_cycles,
                diffs: vec![StateDiff::CycleLength {
                    actual: cpu.instruction_clocks,
                    expected: expected_clocks,
                }],
            });
        }

        // Verify Invariant 2: State Invariance (Registers & RAM)
        let mut diffs = Vec::new();
        for i in 0..8 {
            if cpu.state.d[i] != golden_state.d[i] {
                diffs.push(StateDiff::DataRegister {
                    reg: i,
                    actual: cpu.state.d[i],
                    expected: golden_state.d[i],
                });
            }
        }
        for i in 0..7 {
            if cpu.state.a[i] != golden_state.a[i] {
                diffs.push(StateDiff::AddressRegister {
                    reg: i,
                    actual: cpu.state.a[i],
                    expected: golden_state.a[i],
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
        for (addr, expected_byte) in &golden_ram {
            let actual_byte = bus.read_byte_debug(*addr);
            if actual_byte != *expected_byte {
                diffs.push(StateDiff::RamByte {
                    address: *addr,
                    actual: actual_byte,
                    expected: *expected_byte,
                });
            }
        }

        if !diffs.is_empty() {
            return Err(DmaContentionFailure {
                test_name: test.name.clone(),
                stall_phase,
                burst_length: 1,
                base_clocks,
                actual_clocks: cpu.instruction_clocks,
                wait_cycles: cpu.wait_cycles,
                diffs,
            });
        }
    }

    Ok(())
}

/// Executes a SingleStepTest with a multi-cycle burst DMA stall injected at a specific phase
pub fn run_dma_burst_contention(
    test: &SingleStepTest,
    burst_start_phase: usize,
    burst_length: usize,
) -> Result<(), DmaContentionFailure> {
    // 1. Establish golden baseline without contention
    let mut golden_bus = MemoryBus::new_test();
    golden_bus.set_unmapped_byte(0x00);
    golden_bus.load_test_ram(&test.initial.ram);

    let mut golden_cpu = Cpu::new();
    init_cpu_state(&mut golden_cpu, test);

    let mut base_cck_count = 0;
    loop {
        let res = golden_cpu.step_cck(&mut golden_bus);
        base_cck_count += 1;
        if res.is_completed() || res == StepResult::Halted || res == StepResult::Stopped {
            break;
        }
    }
    let base_clocks = golden_cpu.instruction_clocks;
    let golden_state = golden_cpu.state.clone();

    let mut golden_ram = Vec::new();
    for entry in &test.final_state.ram {
        let addr = entry[0] & 0x00FF_FFFF;
        golden_ram.push((addr, golden_bus.read_byte_debug(addr)));
    }

    // 2. Inject burst stall
    let mut bus = MemoryBus::new_test();
    bus.set_unmapped_byte(0x00);
    bus.load_test_ram(&test.initial.ram);

    let mut cpu = Cpu::new();
    init_cpu_state(&mut cpu, test);

    let mut cck_step = 0;
    loop {
        // Assert stall for burst_length consecutive CCKs starting at burst_start_phase
        bus.chip_ram_blocked =
            cck_step >= burst_start_phase && cck_step < (burst_start_phase + burst_length);

        let res = cpu.step_cck(&mut bus);
        cck_step += 1;
        if res.is_completed() || res == StepResult::Halted || res == StepResult::Stopped {
            break;
        }
        if cck_step > (base_cck_count * 4 + burst_length * 2 + 100) {
            break;
        }
    }

    // Assert Cycle Invariance
    let expected_clocks = base_clocks + (2 * cpu.wait_cycles);
    if cpu.instruction_clocks != expected_clocks {
        return Err(DmaContentionFailure {
            test_name: test.name.clone(),
            stall_phase: burst_start_phase,
            burst_length,
            base_clocks,
            actual_clocks: cpu.instruction_clocks,
            wait_cycles: cpu.wait_cycles,
            diffs: vec![StateDiff::CycleLength {
                actual: cpu.instruction_clocks,
                expected: expected_clocks,
            }],
        });
    }

    // Assert State Invariance
    let mut diffs = Vec::new();
    for i in 0..8 {
        if cpu.state.d[i] != golden_state.d[i] {
            diffs.push(StateDiff::DataRegister {
                reg: i,
                actual: cpu.state.d[i],
                expected: golden_state.d[i],
            });
        }
    }
    for i in 0..7 {
        if cpu.state.a[i] != golden_state.a[i] {
            diffs.push(StateDiff::AddressRegister {
                reg: i,
                actual: cpu.state.a[i],
                expected: golden_state.a[i],
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
            details: "SR changed under burst DMA contention".to_string(),
            diverging_flags: vec![],
        });
    }
    if cpu.state.pc != golden_state.pc {
        diffs.push(StateDiff::ProgramCounter {
            actual: cpu.state.pc,
            expected: golden_state.pc,
        });
    }
    for (addr, expected_byte) in &golden_ram {
        let actual_byte = bus.read_byte_debug(*addr);
        if actual_byte != *expected_byte {
            diffs.push(StateDiff::RamByte {
                address: *addr,
                actual: actual_byte,
                expected: *expected_byte,
            });
        }
    }

    if !diffs.is_empty() {
        return Err(DmaContentionFailure {
            test_name: test.name.clone(),
            stall_phase: burst_start_phase,
            burst_length,
            base_clocks,
            actual_clocks: cpu.instruction_clocks,
            wait_cycles: cpu.wait_cycles,
            diffs,
        });
    }

    Ok(())
}

fn init_cpu_state(cpu: &mut Cpu, test: &SingleStepTest) {
    cpu.state.d = [
        test.initial.d0,
        test.initial.d1,
        test.initial.d2,
        test.initial.d3,
        test.initial.d4,
        test.initial.d5,
        test.initial.d6,
        test.initial.d7,
    ];
    cpu.state.a = [
        test.initial.a0,
        test.initial.a1,
        test.initial.a2,
        test.initial.a3,
        test.initial.a4,
        test.initial.a5,
        test.initial.a6,
    ];
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
    cpu.state.prefetch[0] = (test.initial.prefetch[1] & 0xFFFF) as u16;
    cpu.state.prefetch[1] = 0;
}
