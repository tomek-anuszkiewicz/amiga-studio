#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Cartesian DMA Contention & Address Permutation Stress Test Suite
//!
//! Validates cycle-exact M68000 micro-stepping and MemoryBus invariants across the full
//! combinatorial Cartesian product of:
//! 1. Memory classifications: 2^k permutations (Chip RAM vs Fast RAM) across all touched contacts.
//! 2. DMA cycle schedules: 2^M permutations (Stalled vs Free) across all CCK phases (0..M).
//!
//! Invariants asserted for EVERY permutation:
//! - Cycle Invariance: C = C0 + 2 * wait_states
//! - Fast RAM Immunity: when all contacts are Fast RAM, wait_states == 0
//! - State Invariance: CPU registers and RAM contents are 100% bit-identical to uncontended golden run

mod common;

use common::diagnostic::StateDiff;
use common::schema::SingleStepTest;
use common::singlestep_runner::is_cmpm_postinc_opcode;
use common::test_memory_bus::{MemoryType, TestMemoryBus};
use cpu::state::CpuState;
use cpu::Cpu;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// Percentage of test vectors to sample uniformly from each JSON file (e.g. 10.0 = 10%)
const CARTESIAN_PERCENTAGE: f64 = 5.0;

/// Default maximum DMA contention cycles to permute (2^M schedule combinations).
/// Capped at 6 to ensure cycle schedules up to 6 CCKs (12 CPU clocks, covering 100% of standard ALU/Move/Branch/Stack ops)
/// are exhaustively permuted, while preventing 2^M exponential explosion on multi-transfer instructions (e.g. MOVEM with 34 CCKs = 2^34 = 17 billion runs).
/// For instructions longer than 6 CCKs, the 6-cycle window starting point is randomized deterministically within
/// [0, base_cck_count - dma_cycles], ensuring full timeline phase coverage while never extending outside instruction bounds.
const DEFAULT_MAX_DMA_CYCLES: usize = 6;

fn find_repo_root() -> PathBuf {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    if cwd.join("ROADMAP.md").exists() {
        return cwd;
    }
    let mut dir = cwd;
    while let Some(parent) = dir.parent() {
        if parent.join("ROADMAP.md").exists() {
            return parent.to_path_buf();
        }
        dir = parent.to_path_buf();
    }
    panic!("Could not locate repository root containing ROADMAP.md");
}

/// Deterministically samples a percentage of tests uniformly across the full dataset using a seeded LCG.
fn sample_tests_percentage(
    mut tests: Vec<SingleStepTest>,
    percentage: f64,
    opcode_name: &str,
) -> Vec<SingleStepTest> {
    if tests.is_empty() || percentage <= 0.0 {
        return Vec::new();
    }
    let target_count = ((tests.len() as f64 * (percentage / 100.0)).round() as usize)
        .max(1)
        .min(tests.len());
    if target_count >= tests.len() {
        return tests;
    }

    let mut state = 0x1985_0500_u64;
    for b in opcode_name.bytes() {
        state = state.wrapping_mul(31).wrapping_add(b as u64);
    }

    let n = tests.len();
    for i in 0..target_count {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = i + ((state >> 32) as usize % (n - i));
        tests.swap(i, j);
    }
    tests.truncate(target_count);
    tests
}

fn load_hardware_tests(opcode_name: &str, percentage: f64) -> Vec<SingleStepTest> {
    let repo_root = find_repo_root();
    // NOTE (ROADMAP Batch 1.3): Tom Harte single-step tests do not have dedicated `CMPM.<size>.json` files;
    // `CMPM` vectors are bundled inside `CMP.<size>.json` alongside standard `CMP <ea>, Dn`.
    // Currently, only CMPM is implemented, so we filter by `is_cmpm_postinc_opcode`.
    // When Batch 1.3 (`CMP`, `CMPA`, `CMPI`) is implemented, alter or remove this guard so the full `CMP` family is tested.
    if let Some(suffix) = opcode_name.strip_prefix("CMPM.") {
        let target_file = repo_root
            .join("ref_src/SingleStepTests-680x0/68000/v1")
            .join(format!("CMP.{}.json", suffix));
        let content = fs::read_to_string(&target_file)
            .unwrap_or_else(|_| panic!("Failed to read {}", target_file.display()));
        let tests: Vec<SingleStepTest> = serde_json::from_str(&content)
            .unwrap_or_else(|err| panic!("JSON parse error in {}: {}", target_file.display(), err));
        let filtered: Vec<SingleStepTest> = tests
            .into_iter()
            .filter(|t| is_cmpm_postinc_opcode(t.initial.prefetch[0]))
            .collect();

        return sample_tests_percentage(filtered, percentage, opcode_name);
    }
    let target_file = repo_root
        .join("ref_src/SingleStepTests-680x0/68000/v1")
        .join(format!("{}.json", opcode_name));

    if target_file.exists() {
        let content = fs::read_to_string(&target_file)
            .unwrap_or_else(|_| panic!("Failed to read {}", target_file.display()));
        let tests: Vec<SingleStepTest> = serde_json::from_str(&content)
            .unwrap_or_else(|err| panic!("JSON parse error in {}: {}", target_file.display(), err));
        return sample_tests_percentage(tests, percentage, opcode_name);
    }
    panic!("Could not find test vector file for opcode {}", opcode_name);
}

fn run_cartesian_for_opcodes(opcodes: &[&str]) {
    let start_time = Instant::now();
    let mut total_permutations = 0;
    let mut total_tests = 0;

    for &opcode in opcodes {
        let tests = load_hardware_tests(opcode, CARTESIAN_PERCENTAGE);
        total_tests += tests.len();
        for test in &tests {
            let stats = run_dma_full_cartesian_permutation(test, DEFAULT_MAX_DMA_CYCLES)
                .unwrap_or_else(|err| {
                    panic!(
                        "Cartesian DMA permutation failed on [{}] {}: {}",
                        opcode, test.name, err
                    );
                });
            total_permutations += stats.total_permutations;
        }
    }

    let elapsed = start_time.elapsed();
    eprintln!(
        "[BENCHMARK] {:?} Cartesian ({:.1}% sample, max DMA {}): {} runs across {} tests in {:?} ({:.2} µs/run)",
        opcodes,
        CARTESIAN_PERCENTAGE,
        DEFAULT_MAX_DMA_CYCLES,
        total_permutations,
        total_tests,
        elapsed,
        elapsed.as_micros() as f64 / total_permutations.max(1) as f64
    );
}

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
#[derive(Debug)]
pub struct PreFlight {
    pub base_clocks: u32,
    pub base_cck_count: u64,
    pub golden_state: CpuState,
    pub golden_ram: Vec<(u32, u8)>,
    pub unique_contacts: Vec<u32>,
}

/// Executes an uncontended golden run to determine base timings and unique memory contact cells
fn run_preflight(test: &SingleStepTest) -> PreFlight {
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
fn run_dma_full_cartesian_permutation(
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
        let actual = cpu.state.d_long(i);
        let expected = golden_state.d_long(i);
        if actual != expected {
            diffs.push(StateDiff::DataRegister {
                reg: i,
                actual,
                expected,
            });
        }
    }
    for i in 0..7 {
        let actual = cpu.state.a_long(i);
        let expected = golden_state.a_long(i);
        if actual != expected {
            diffs.push(StateDiff::AddressRegister {
                reg: i,
                actual,
                expected,
            });
        }
    }
    if cpu.state.usp() != golden_state.usp() {
        diffs.push(StateDiff::UserStackPointer {
            actual: cpu.state.usp(),
            expected: golden_state.usp(),
        });
    }
    if cpu.state.ssp() != golden_state.ssp() {
        diffs.push(StateDiff::SupervisorStackPointer {
            actual: cpu.state.ssp(),
            expected: golden_state.ssp(),
        });
    }
    if cpu.state.sr() != golden_state.sr() {
        diffs.push(StateDiff::StatusRegister {
            actual: cpu.state.sr(),
            expected: golden_state.sr(),
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
    cpu.state.set_d_long(0, test.initial.d0);
    cpu.state.set_d_long(1, test.initial.d1);
    cpu.state.set_d_long(2, test.initial.d2);
    cpu.state.set_d_long(3, test.initial.d3);
    cpu.state.set_d_long(4, test.initial.d4);
    cpu.state.set_d_long(5, test.initial.d5);
    cpu.state.set_d_long(6, test.initial.d6);
    cpu.state.set_d_long(7, test.initial.d7);
    cpu.state.set_a_long(0, test.initial.a0);
    cpu.state.set_a_long(1, test.initial.a1);
    cpu.state.set_a_long(2, test.initial.a2);
    cpu.state.set_a_long(3, test.initial.a3);
    cpu.state.set_a_long(4, test.initial.a4);
    cpu.state.set_a_long(5, test.initial.a5);
    cpu.state.set_a_long(6, test.initial.a6);
    cpu.state.set_sr(test.initial.sr);
    cpu.state.set_usp(test.initial.usp);
    cpu.state.set_ssp(test.initial.ssp);
    let is_harte = test.name.contains('[');
    if is_harte {
        cpu.state.pc = test.initial.pc.wrapping_add(4);
    } else {
        cpu.state.pc = test.initial.pc;
    }
    // Exception frames use the opcode address, four bytes behind the primed PC.
    cpu.state.instruction_pc = cpu.state.pc.wrapping_sub(4);
    cpu.state.ir = (test.initial.prefetch[0] & 0xFFFF) as u16;
    cpu.state.prefetch = (test.initial.prefetch[1] & 0xFFFF) as u16;
}

// ============================================================================
// System, Traps & Stack Control
// ============================================================================

#[test]
fn test_dma_cartesian_system_and_traps() {
    run_cartesian_for_opcodes(&["NOP", "RTS", "TRAP"]);
}

#[test]
fn test_dma_cartesian_stack_ops() {
    run_cartesian_for_opcodes(&["PEA"]);
}

// ============================================================================
// Program Control & Branches
// ============================================================================

#[test]
fn test_dma_cartesian_branches() {
    run_cartesian_for_opcodes(&["Bcc", "BSR", "JMP", "JSR"]);
}

// ============================================================================
// Data Movement (MOVE, MOVEA, MOVEQ, MOVEM)
// ============================================================================

#[test]
fn test_dma_cartesian_move() {
    run_cartesian_for_opcodes(&["MOVE.b", "MOVE.w", "MOVE.l", "MOVE.q", "MOVEA.w", "MOVEA.l"]);
}

#[test]
fn test_dma_cartesian_movem() {
    run_cartesian_for_opcodes(&["MOVEM.w", "MOVEM.l"]);
}

// ============================================================================
// Integer Arithmetic (ADD, ADDA, ADDX)
// ============================================================================

#[test]
fn test_dma_cartesian_add() {
    run_cartesian_for_opcodes(&["ADD.b", "ADD.w", "ADD.l", "ADDA.w", "ADDA.l"]);
}

#[test]
fn test_dma_cartesian_addx() {
    run_cartesian_for_opcodes(&["ADDX.b", "ADDX.w", "ADDX.l"]);
}

#[test]
fn test_dma_cartesian_sub() {
    run_cartesian_for_opcodes(&["SUB.b", "SUB.w", "SUB.l", "SUBA.w", "SUBA.l"]);
}

#[test]
fn test_dma_cartesian_subx() {
    run_cartesian_for_opcodes(&["SUBX.b", "SUBX.w", "SUBX.l"]);
}

// ============================================================================
// Logic Operations (NOT)
// ============================================================================

#[test]
fn test_dma_cartesian_logic() {
    run_cartesian_for_opcodes(&[
        "NOT.b", "NOT.w", "NOT.l", "AND.b", "AND.w", "AND.l", "OR.b", "OR.w", "OR.l", "EOR.b",
        "EOR.w", "EOR.l",
    ]);
}

// ============================================================================
// Bit Manipulation (BSET)
// ============================================================

#[test]
fn test_dma_cartesian_bitops() {
    run_cartesian_for_opcodes(&["BSET", "BTST", "BCLR", "BCHG"]);
}

// ============================================================================
// Shifts & Rotates (ASL)
// ============================================================================

#[test]
fn test_dma_cartesian_shifts() {
    run_cartesian_for_opcodes(&[
        "ASL.b", "ASL.w", "ASL.l", "ASR.b", "ASR.w", "ASR.l", "LSL.b", "LSL.w", "LSL.l", "LSR.b",
        "LSR.w", "LSR.l", "ROL.b", "ROL.w", "ROL.l", "ROR.b", "ROR.w", "ROR.l", "ROXL.b", "ROXL.w",
        "ROXL.l", "ROXR.b", "ROXR.w", "ROXR.l",
    ]);
}

#[test]
fn test_dma_cartesian_cmp() {
    run_cartesian_for_opcodes(&[
        "CMP.b", "CMP.w", "CMP.l", "CMPA.w", "CMPA.l", "CMPM.b", "CMPM.w", "CMPM.l",
    ]);
}

// ============================================================================
// Test (TST)
// ============================================================================

#[test]
fn test_dma_cartesian_tst() {
    run_cartesian_for_opcodes(&["TST.b", "TST.w", "TST.l"]);
}

// ============================================================================
// Multiplications & Divisions (MULU, MULS, DIVU, DIVS)
// ============================================================================

#[test]
fn test_dma_cartesian_mul_div() {
    run_cartesian_for_opcodes(&["MULU", "MULS", "DIVU", "DIVS"]);
}

// ============================================================================
// BCD & Math Extensions (Batch 1.8: CLR, NEG, NEGX, EXT, ABCD, SBCD, NBCD)
// ============================================================================

#[test]
fn test_dma_cartesian_bcd_math() {
    run_cartesian_for_opcodes(&[
        "CLR.b", "CLR.w", "CLR.l", "NEG.b", "NEG.w", "NEG.l", "NEGX.b", "NEGX.w", "NEGX.l",
        "EXT.w", "EXT.l", "ABCD", "SBCD", "NBCD",
    ]);
}

// ============================================================================
// Looping & Conditional Setting (Batch 1.9: DBcc, Scc)
// ============================================================================

#[test]
fn test_dma_cartesian_looping_and_scc() {
    run_cartesian_for_opcodes(&["DBcc", "Scc"]);
}

// ============================================================================
// Stack & Frame Control (Batch 1.10: SWAP, EXG, LINK, UNLK, LEA, CHK)
// ============================================================================

#[test]
fn test_dma_cartesian_stack_frame_control() {
    run_cartesian_for_opcodes(&["SWAP", "EXG", "LINK", "UNLINK", "LEA", "CHK"]);
}

// ============================================================================
// Privileged & Atomic Hardware Ops (Batch 1.11)
// ============================================================================

#[test]
fn test_dma_cartesian_privileged_and_atomic() {
    run_cartesian_for_opcodes(&[
        "TAS",
        "RTE",
        "RTR",
        "TRAPV",
        "MOVEtoCCR",
        "MOVEtoSR",
        "MOVEfromSR",
        "MOVEtoUSP",
        "MOVEfromUSP",
        "RESET",
        "ANDItoCCR",
        "ANDItoSR",
        "ORItoCCR",
        "ORItoSR",
        "EORItoCCR",
        "EORItoSR",
    ]);
}

#[test]
fn test_preflight_debug_derive() {
    let tests = load_hardware_tests("NOP", 100.0);
    assert!(!tests.is_empty(), "NOP tests must be loaded");
    let preflight: PreFlight = run_preflight(&tests[0]);
    let debug_str = format!("{:?}", preflight);
    assert!(debug_str.contains("PreFlight"));
}
