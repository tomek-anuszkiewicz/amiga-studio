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

use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use test_runner::dma_harness::run_dma_full_cartesian_permutation;
use test_runner::is_cmpm_postinc_opcode;
use test_runner::schema::SingleStepTest;

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

fn load_mame_tests(opcode_name: &str, percentage: f64) -> Vec<SingleStepTest> {
    let repo_root = find_repo_root();
    // NOTE (ROADMAP Batch 1.3): MAME & Tom Harte do not have dedicated `CMPM.<size>.json` files;
    // `CMPM` vectors are bundled inside `CMP.<size>.json` alongside standard `CMP <ea>, Dn`.
    // Currently, only CMPM is implemented, so we filter by `is_cmpm_postinc_opcode`.
    // When Batch 1.3 (`CMP`, `CMPA`, `CMPI`) is implemented, alter or remove this guard so the full `CMP` family is tested.
    if let Some(suffix) = opcode_name.strip_prefix("CMPM.") {
        let target_file = repo_root
            .join("ref_src/SingleStepTests-m68000/v1")
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
        .join("ref_src/SingleStepTests-m68000/v1")
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
        let tests = load_mame_tests(opcode, CARTESIAN_PERCENTAGE);
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
