//! Integration tests for synthetic Agnus DMA contention stress runner
//!
//! Validates that under single-cycle and multi-cycle burst DMA bus contention:
//! 1. State Invariance holds (registers and memory identical to uncontended run)
//! 2. Cycle Invariance holds (CPU clocks increase by exactly 2 * wait_cycles)

use std::fs::File;
use std::io::BufReader;
use test_runner::dma_harness::{run_dma_burst_contention, run_dma_contention_sweep};
use test_runner::schema::SingleStepTest;

fn load_mame_tests(name: &str, limit: usize) -> Vec<SingleStepTest> {
    let path = format!("ref_src/SingleStepTests-m68000/v1/{}.json", name);
    let candidate = std::path::Path::new(&path);
    let resolved = if candidate.exists() {
        candidate.to_path_buf()
    } else {
        std::path::Path::new("../..").join(&path)
    };

    let file = File::open(&resolved)
        .unwrap_or_else(|err| panic!("Failed to open test file '{}': {}", resolved.display(), err));
    let reader = BufReader::new(file);
    let tests: Vec<SingleStepTest> = serde_json::from_reader(reader)
        .unwrap_or_else(|err| panic!("Failed to parse test file '{}': {}", resolved.display(), err));
    tests.into_iter().take(limit).collect()
}

#[test]
fn test_dma_contention_sweep_nop() {
    let tests = load_mame_tests("NOP", 20);
    for test in tests {
        run_dma_contention_sweep(&test, 10)
            .unwrap_or_else(|err| panic!("DMA contention sweep failed on {}: {}", test.name, err));
    }
}

#[test]
fn test_dma_burst_contention_nop() {
    let tests = load_mame_tests("NOP", 20);
    for test in tests {
        // Burst stall of 4 CCK cycles
        run_dma_burst_contention(&test, 0, 4)
            .unwrap_or_else(|err| panic!("DMA burst contention failed on {}: {}", test.name, err));
    }
}

#[test]
fn test_dma_contention_sweep_pea() {
    let tests = load_mame_tests("PEA", 10);
    for test in tests {
        // PEA (An) is mode 2
        let opcode = (test.initial.prefetch[0] & 0xFFFF) as u16;
        let mode = ((opcode >> 3) & 0x07) as u8;
        if mode == 2 {
            run_dma_contention_sweep(&test, 10).unwrap_or_else(|err| {
                panic!("DMA contention sweep failed on {}: {}", test.name, err)
            });
        }
    }
}
