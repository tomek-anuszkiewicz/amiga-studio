//! Golden Master Execution Trace Regression Test Suite
//!
//! Validates that all 108 synthesized benchmark programs produce deterministic,
//! regression-free execution traces matching verified golden hashes.
//!
//! =========================================================================================
//! ⚠️ ANTI-TAMPER POLICY & INVARIANCE CONTRACT:
//! DO NOT MODIFY GOLDEN_TRACE_HASHES TO "FIX" A FAILING TEST!
//! The golden hashes below represent verified hardware truth for M68000 execution traces.
//! If this test fails, it indicates an unintentional regression in CPU instruction execution,
//! micro-step sequencing, effective address calculation, register modification, or cycle timing.
//! Blindly updating these constants to silence a test failure is strictly prohibited by
//! AGENTS.md and spec-compliance.md.
//! =========================================================================================

use std::fs;
use std::path::PathBuf;
use test_runner::benchmark::{all_benchmark_specs, trace_program, BenchmarkProgramBuilder};

/// Fast, deterministic, platform-independent 64-bit FNV-1a hash.
/// Normalizes CRLF (\r\n) to LF (\n) to ensure cross-platform reproducibility.
fn fnv1a_64_normalized(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in text.as_bytes() {
        if b != b'\r' {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

/// Locates repository root for accessing `tests/benchmarks/traces/` on disk
fn find_repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    while !dir.join("ROADMAP.md").exists() {
        if !dir.pop() {
            return None;
        }
    }
    Some(dir)
}

/// Verified golden trace hashes for all 108 M68000 benchmark specifications (K = 20)
pub const GOLDEN_TRACE_HASHES: &[(&str, u64)] = &[
    ("BASE-00", 0x3DB1601E018F4FED),
    ("MOV-01", 0xACA13BF6D6B7484C),
    ("MOV-02", 0xB1CFA3469C6EACA6),
    ("MOV-03", 0xCCC80D7A4BA03E50),
    ("MOV-04", 0x834EAD9BA3311832),
    ("MOV-05", 0x39C96E9B3CE73B3D),
    ("MOV-06", 0xA6435440E344C360),
    ("MOV-07", 0x26BBFCF34AE4C22D),
    ("MOV-08", 0x3FAA419F41035BA1),
    ("MOV-09", 0xD978ECE070535BE4),
    ("MOV-10", 0x77ABE4A4A4C39191),
    ("MOV-11", 0x71121F0DCD61C63F),
    ("MOV-12", 0x527E338330C1BDB9),
    ("MOV-13", 0xEF035F61871F661C),
    ("MOV-14", 0x24A3E9525FBC0643),
    ("MOV-15", 0x7915FEAE1644F9F7),
    ("MOV-16", 0x9DAA8885E1A8A547),
    ("MOV-17", 0x98899B85BEF2A64E),
    ("MOV-18", 0x7866B95FCF372EF7),
    ("MOV-19", 0x71DE887AEE49C70B),
    ("MOV-20", 0x20DD947DCFEE8E98),
    ("MOV-21", 0x0C6F4CEBBFA2FC4B),
    ("MOV-22", 0x96BD1AA1F3CB4C69),
    ("MOV-23", 0xE05BCF31C6DDFAD5),
    ("ARITH-01", 0x25B381C296236676),
    ("ARITH-02", 0x1CDD7D2E40954D2A),
    ("ARITH-03", 0xC363BCFA647EBC15),
    ("ARITH-04", 0x768F9BFB5F957DCB),
    ("ARITH-05", 0x30E8F97B25B0C279),
    ("ARITH-06", 0x27382415EB3004A4),
    ("ARITH-07", 0xA4F56ACDAEE5329C),
    ("ARITH-08", 0xC8B0E8A08165899D),
    ("ARITH-09", 0x45C0BCC43182C491),
    ("ARITH-10", 0xAEB6DE2B35EF3A76),
    ("ARITH-11", 0x7D9A202DF8D79A69),
    ("ARITH-12", 0xAA0AE91F3CA66B80),
    ("ARITH-13", 0x94DA0CA89911CD05),
    ("ARITH-14", 0xC4D5DBF4A601D7D3),
    ("ARITH-15", 0xE4D4473FC8BA4B63),
    ("ARITH-16", 0xC21DFC1939F5305E),
    ("ARITH-17", 0x3E276915C753B0FF),
    ("ARITH-18", 0x54861D617FCDDB0F),
    ("ARITH-18b", 0x5E1FD29722126351),
    ("ARITH-18c", 0x5BF26C3562D907CD),
    ("ARITH-19", 0x87AA9ECB81F839A9),
    ("ARITH-19b", 0x99B14F01833D190A),
    ("ARITH-19c", 0x85555608B3E67BA2),
    ("ARITH-20", 0xFF07FFF2528BA8F4),
    ("ARITH-21", 0xD81562D023316FF3),
    ("ARITH-22", 0xF2548E5C74BEF87C),
    ("ARITH-23", 0x84972DC483F6B35D),
    ("ARITH-24", 0x34CCCF7905E57736),
    ("ARITH-25", 0x2A0F5D0362023E78),
    ("LOGIC-01", 0xA55CD702185FC816),
    ("LOGIC-02", 0x2CCACE8CF2FE7905),
    ("LOGIC-03", 0x3FFD3C486872F8D5),
    ("LOGIC-04", 0x13B9582944F68CED),
    ("LOGIC-05", 0x347FC850BB779951),
    ("LOGIC-06", 0xA8438D072F2F44F0),
    ("LOGIC-07", 0x8B2E683C08433D61),
    ("LOGIC-08", 0xE87E3282D2D8CF2F),
    ("LOGIC-09", 0x0007BDD739354D1F),
    ("LOGIC-10", 0x06BF067DCB2875AB),
    ("LOGIC-11", 0x7C1884FFAB281A09),
    ("LOGIC-12", 0x8F31C331BC4189FA),
    ("LOGIC-13", 0x28A20FDD5512D62D),
    ("LOGIC-14", 0xA61624ACD2AE6574),
    ("SHIFT-01", 0xE6DAB54B97EE7AF8),
    ("SHIFT-02", 0xB43E6AD27537458B),
    ("SHIFT-03", 0xB5BC229C7AAD96AE),
    ("SHIFT-04", 0xA7B1BA76B22AC2C7),
    ("SHIFT-05", 0x9AA55DD5110ABC5E),
    ("SHIFT-06", 0xE8131E29086809F0),
    ("SHIFT-07", 0xA5F6C515DE4DEE13),
    ("SHIFT-08", 0x71F045CDA83C22B1),
    ("CMP-01", 0x60C79E8CFAF44F52),
    ("CMP-02", 0x852F4650526B3F10),
    ("CMP-03", 0x345DAF7E6AED2E43),
    ("CMP-04", 0xD4DE82350D04ED00),
    ("CMP-05", 0x6367F77D7AE2FEAF),
    ("CMP-06", 0xAC831586887E65F1),
    ("CMP-07", 0x6A16E11C81A1E3B3),
    ("CMP-08", 0x49B4D1A6189B1D2C),
    ("CMP-09", 0xB15E3026A2503387),
    ("CMP-10", 0xE1E567F79B18E872),
    ("BCD-01", 0x1ADC4842A64DA4DE),
    ("BCD-02", 0x612616535851B961),
    ("BCD-03", 0x30ED54297D87A346),
    ("BCD-04", 0x8A36B386101477EF),
    ("BCD-05", 0xF673290103A7FB30),
    ("FLOW-01", 0x5B5CD3F1A8AB9257),
    ("FLOW-02", 0xC20500C95D2CACA9),
    ("FLOW-03", 0x48C4D9D028D81D6B),
    ("FLOW-04", 0xE781AB1F1F2BC001),
    ("FLOW-05", 0x02EF4B0A5D9295D7),
    ("FLOW-06", 0xB3D12573637C524D),
    ("FLOW-07", 0x501390A0B1BB9C81),
    ("FLOW-08", 0x5CCD30C23DF2322E),
    ("SYS-01", 0xC79895D23E1404CD),
    ("SYS-02", 0x69765DD977D6ABC9),
    ("SYS-03", 0x721E6014B05B023C),
    ("SYS-04", 0xA5DDB84723C699A1),
    ("SYS-05", 0x2FA53AF94B83FA5B),
    ("SYS-06", 0x9BF2D39B01C8A4DC),
    ("SYS-07", 0xC81FF107689A0649),
    ("SYS-08", 0xFCAAF8F32D2F76EB),
    ("SYS-09", 0x227BF392218F0C8A),
    ("SYS-10", 0x4ACF02A3B7A87F44),
];

#[test]
fn test_all_benchmark_execution_traces_against_golden_hashes() {
    let unroll_k = 20;
    let max_safety_steps = unroll_k * 3 + 50;

    let repo_root = find_repo_root();
    let traces_dir = repo_root
        .as_ref()
        .map(|r| r.join("tests/benchmarks/traces"));

    let mut failures = Vec::new();
    let mut verified_count = 0;

    for spec in all_benchmark_specs() {
        let expected_hash = match GOLDEN_TRACE_HASHES.iter().find(|(id, _)| *id == spec.id) {
            Some((_, h)) => *h,
            None => {
                failures.push(format!(
                    "Missing golden hash registration for benchmark spec [{}] ({})",
                    spec.id, spec.representative_syntax
                ));
                continue;
            }
        };

        // 1. Synthesize and execute single-pass trace in memory
        let program = BenchmarkProgramBuilder::new(*spec)
            .with_unroll(unroll_k)
            .build();
        let trace = trace_program(&program, max_safety_steps);
        let in_memory_text = trace.format_text();
        let actual_hash = fnv1a_64_normalized(&in_memory_text);

        if actual_hash != expected_hash {
            failures.push(format!(
                "\n⛔ CRITICAL REGRESSION: TRACE HASH MISMATCH for [{}] ({})\n\
                   Expected Golden Hash: 0x{:016X}\n\
                   Actual Computed Hash: 0x{:016X}\n\
                   The execution trace or CPU behavior for this benchmark program has diverged!\n\
                   ⚠️ ANTI-TAMPER RULE: DO NOT MODIFY GOLDEN_TRACE_HASHES TO SILENCE THIS TEST!\n\
                   Golden hashes represent immutable ground truth. A mismatch indicates that instruction\n\
                   execution, cycle timing, register side effects, or micro-steps have regressed in the CPU core.\n\
                   Agents are strictly forbidden from modifying GOLDEN_TRACE_HASHES in test_benchmark_trace.rs.\n\
                   ACTION REQUIRED:\n\
                   1. Run: cargo run -p test_runner --release -- bench --dump-traces --filter \"{}\"\n\
                   2. Inspect git diff for tests/benchmarks/traces/{}.trace to locate the exact diverged cycle/register.\n\
                   3. Fix the underlying regression in crates/m68000/ or crates/physical_memory/.\n",
                spec.id, spec.representative_syntax, expected_hash, actual_hash, spec.id, spec.id
            ));
            continue;
        }

        // 2. Ensure trace spec artifacts are generated and always overwritten on disk
        if let Some(ref dir) = traces_dir {
            let _ = fs::create_dir_all(dir);
            let trace_file = dir.join(format!("{}.trace", spec.id));
            let needs_write = match fs::read_to_string(&trace_file) {
                Ok(existing) => existing != in_memory_text,
                Err(_) => true,
            };
            if needs_write {
                if let Err(e) = fs::write(&trace_file, &in_memory_text) {
                    failures.push(format!(
                        "Failed to write trace file {:?}: {}",
                        trace_file, e
                    ));
                    continue;
                }
            }
            // Verify written disk file matches golden hash
            if let Ok(file_content) = fs::read_to_string(&trace_file) {
                let file_hash = fnv1a_64_normalized(&file_content);
                if file_hash != expected_hash {
                    failures.push(format!(
                        "Disk trace file hash mismatch for [{}] ({}) generated",
                        spec.id, spec.representative_syntax
                    ));
                    continue;
                }
            }
        }

        verified_count += 1;
    }

    assert!(
        failures.is_empty(),
        "Benchmark Trace Golden Master Verification Failed:\n{}",
        failures.join("\n")
    );

    assert_eq!(
        verified_count,
        GOLDEN_TRACE_HASHES.len(),
        "All {} registered benchmark traces must be verified",
        GOLDEN_TRACE_HASHES.len()
    );

    println!(
        "\n=== 100% VERIFIED: ALL {} BENCHMARK TRACES MATCH GOLDEN MASTER HASHES ===",
        verified_count
    );
}
