use test_runner::is_cmpm_postinc_opcode;
use test_runner::runner::{run_test_file_filtered_with_mode, run_test_file_with_mode, VerifyMode};
use test_runner::schema::SingleStepTest;

/// Default number of test cases to run per opcode from each test suite
const DEFAULT_SAMPLE_LIMIT: usize = 50;

/// Resolves effective sample limit, allowing full execution via SINGLESTEP_FULL=1 or custom limit via SINGLESTEP_LIMIT=<N>
fn resolve_limit(explicit: usize) -> Option<usize> {
    if let Ok(val) = std::env::var("SINGLESTEP_FULL") {
        if val == "1" || val.eq_ignore_ascii_case("true") {
            return None;
        }
    }
    if let Ok(val) = std::env::var("SINGLESTEP_LIMIT") {
        if val.eq_ignore_ascii_case("full") || val.eq_ignore_ascii_case("all") {
            return None;
        }
        if let Ok(num) = val.parse::<usize>() {
            return Some(num);
        }
    }
    Some(explicit)
}

/// Helper function to execute a test against both MAME and Real 68k (Tom Harte) suites
fn run_dual_test(name: &str, limit: usize) {
    run_dual_test_with_mode(name, limit, VerifyMode::StateOnly);
}

/// Helper function to execute a test against both suites with specified verification mode
fn run_dual_test_with_mode(name: &str, limit: usize, mode: VerifyMode) {
    let effective = resolve_limit(limit);
    let mame_path = format!("ref_src/SingleStepTests-m68000/v1/{}.json", name);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", name);

    // 1. MAME SingleStepTests suite
    let mame_res = run_test_file_with_mode(&mame_path, effective, mode)
        .unwrap_or_else(|err| panic!("Failed to open/parse MAME test '{}': {}", mame_path, err));
    let (mame_passed, mame_failed) = mame_res;
    assert!(
        mame_passed > 0,
        "No MAME tests passed for {} (total executed: {})",
        name,
        mame_passed + mame_failed
    );
    assert_eq!(
        mame_failed,
        0,
        "MAME tests failed for {}: {}/{} failed",
        name,
        mame_failed,
        mame_passed + mame_failed
    );

    // 2. Real 68k (Tom Harte) SingleStepTests-680x0 suite
    let harte_res = run_test_file_with_mode(&harte_path, effective, mode).unwrap_or_else(|err| {
        panic!(
            "Failed to open/parse Real 68k test '{}': {}",
            harte_path, err
        )
    });
    let (harte_passed, harte_failed) = harte_res;
    assert!(
        harte_passed > 0,
        "No Real 68k tests passed for {} (total executed: {})",
        name,
        harte_passed + harte_failed
    );
    assert_eq!(
        harte_failed,
        0,
        "Real 68k (Tom Harte) tests failed for {}: {}/{} failed",
        name,
        harte_failed,
        harte_passed + harte_failed
    );
}

/// Helper function to execute a filtered test against both suites
fn run_dual_test_filtered<F>(name: &str, limit: usize, filter: F)
where
    F: Fn(&SingleStepTest) -> bool + Copy,
{
    let effective = resolve_limit(limit);
    let mame_path = format!("ref_src/SingleStepTests-m68000/v1/{}.json", name);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", name);

    // 1. MAME SingleStepTests suite
    let mame_res =
        run_test_file_filtered_with_mode(&mame_path, effective, VerifyMode::StateOnly, filter)
            .unwrap_or_else(|err| {
                panic!("Failed to open/parse MAME test '{}': {}", mame_path, err)
            });
    let (mame_passed, mame_failed) = mame_res;
    assert!(
        mame_passed > 0,
        "No MAME tests passed for {} (total executed: {})",
        name,
        mame_passed + mame_failed
    );
    assert_eq!(
        mame_failed,
        0,
        "MAME tests failed for {}: {}/{} failed",
        name,
        mame_failed,
        mame_passed + mame_failed
    );

    // 2. Real 68k (Tom Harte) SingleStepTests-680x0 suite
    let harte_res =
        run_test_file_filtered_with_mode(&harte_path, effective, VerifyMode::StateOnly, filter)
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to open/parse Real 68k test '{}': {}",
                    harte_path, err
                )
            });
    let (harte_passed, harte_failed) = harte_res;
    assert!(
        harte_passed > 0,
        "No Real 68k tests passed for {} (total executed: {})",
        name,
        harte_passed + harte_failed
    );
    assert_eq!(
        harte_failed,
        0,
        "Real 68k (Tom Harte) tests failed for {}: {}/{} failed",
        name,
        harte_failed,
        harte_passed + harte_failed
    );
}

// ============================================================================
// System, Control Flow & Exceptions (Microcode Archetypes)
// ============================================================================

#[test]
fn test_nop() {
    run_dual_test_with_mode("NOP", 100, VerifyMode::Full);
}

#[test]
fn test_rts() {
    run_dual_test("RTS", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_trap() {
    run_dual_test_with_mode("TRAP", DEFAULT_SAMPLE_LIMIT, VerifyMode::Full);
}

#[test]
fn test_bcc() {
    run_dual_test("Bcc", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bsr() {
    run_dual_test("BSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_jmp() {
    run_dual_test("JMP", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_jsr() {
    run_dual_test("JSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_pea() {
    run_dual_test("PEA", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_pea_an_full_verification() {
    let base_path = "ref_src/SingleStepTests-680x0/68000/v1/PEA.json";
    let harte_path = if std::path::Path::new(base_path).exists() {
        std::path::PathBuf::from(base_path)
    } else {
        std::path::Path::new("../..").join(base_path)
    };
    let file = std::fs::File::open(&harte_path).expect("Failed to open PEA test file");
    let tests: Vec<test_runner::schema::SingleStepTest> =
        serde_json::from_reader(std::io::BufReader::new(file)).expect("Failed to parse PEA tests");

    let an_tests: Vec<_> = tests
        .into_iter()
        .filter(|t| t.name.contains("[PEA (A") && t.name.contains(")]"))
        .take(50)
        .collect();

    assert!(!an_tests.is_empty(), "No PEA (An) tests found");
    for (idx, test) in an_tests.iter().enumerate() {
        if let Err(failure) = test_runner::runner::run_single_test_detail(
            test,
            harte_path.to_str().unwrap(),
            idx,
            VerifyMode::Full,
        ) {
            panic!(
                "PEA (An) Full Verification Failed on {}:\n{}",
                test.name,
                failure.format_diagnostic()
            );
        }
    }
}

// ============================================================================
// Data Movement (MOVE.W, MOVEA, MOVEQ, MOVEM Archetypes)
// ============================================================================

#[test]
fn test_move_w() {
    run_dual_test("MOVE.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_q() {
    run_dual_test("MOVE.q", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movea_w() {
    run_dual_test("MOVEA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movea_l() {
    run_dual_test("MOVEA.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movem_w() {
    run_dual_test("MOVEM.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movem_l() {
    run_dual_test("MOVEM.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Integer & Extended Arithmetic (ADD, ADDA, ADDX Archetypes)
// ============================================================================

#[test]
fn test_add_b() {
    run_dual_test("ADD.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_add_w() {
    run_dual_test("ADD.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_add_l() {
    run_dual_test("ADD.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_adda_w() {
    run_dual_test("ADDA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_adda_l() {
    run_dual_test("ADDA.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_addx_b() {
    run_dual_test("ADDX.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_addx_w() {
    run_dual_test("ADDX.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_addx_l() {
    run_dual_test("ADDX.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_sub_b() {
    run_dual_test("SUB.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_sub_w() {
    run_dual_test("SUB.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_sub_l() {
    run_dual_test("SUB.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_suba_w() {
    run_dual_test("SUBA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_suba_l() {
    run_dual_test("SUBA.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_subx_b() {
    run_dual_test("SUBX.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_subx_w() {
    run_dual_test("SUBX.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_subx_l() {
    run_dual_test("SUBX.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Logic Operations (NOT, AND, OR, EOR Archetypes)
// ============================================================================

#[test]
fn test_not_b() {
    run_dual_test("NOT.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_not_w() {
    run_dual_test("NOT.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_not_l() {
    run_dual_test("NOT.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_and_b() {
    run_dual_test("AND.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_and_w() {
    run_dual_test("AND.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_and_l() {
    run_dual_test("AND.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_or_b() {
    run_dual_test("OR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_or_w() {
    run_dual_test("OR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_or_l() {
    run_dual_test("OR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eor_b() {
    run_dual_test("EOR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eor_w() {
    run_dual_test("EOR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eor_l() {
    run_dual_test("EOR.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Bit Manipulation (BSET Archetype)
// ============================================================================

#[test]
fn test_bset() {
    run_dual_test("BSET", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Shifts & Rotates (ASL Archetype)
// ============================================================================

#[test]
fn test_asl_b() {
    run_dual_test("ASL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asl_w() {
    run_dual_test("ASL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asl_l() {
    run_dual_test("ASL.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Comparisons (CMPM Archetype)
// NOTE (ROADMAP Batch 1.3): `CMP.<size>.json` contains both standard `CMP` and `CMPM`.
// Once standard `CMP`, `CMPA`, and `CMPI` are implemented, add `test_cmp_b`,
// `test_cmp_w`, and `test_cmp_l` for the full files while retaining or adjusting
// these `CMPM`-filtered tests.
// ============================================================================

#[test]
fn test_cmpm_b() {
    run_dual_test_filtered("CMP.b", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

#[test]
fn test_cmpm_w() {
    run_dual_test_filtered("CMP.w", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

#[test]
fn test_cmpm_l() {
    run_dual_test_filtered("CMP.l", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

#[test]
fn test_cmp_b() {
    run_dual_test("CMP.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmp_w() {
    run_dual_test("CMP.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmp_l() {
    run_dual_test("CMP.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmpa_w() {
    run_dual_test("CMPA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmpa_l() {
    run_dual_test("CMPA.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Test (TST Archetype)
// ============================================================================

#[test]
fn test_tst_b() {
    run_dual_test("TST.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_tst_w() {
    run_dual_test("TST.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_tst_l() {
    run_dual_test("TST.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Shifts & Rotates (Batch 1.4: ASR, LSL, LSR, ROL, ROR, ROXL, ROXR)
// ============================================================================

#[test]
fn test_asr_b() {
    run_dual_test("ASR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asr_w() {
    run_dual_test("ASR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asr_l() {
    run_dual_test("ASR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsl_b() {
    run_dual_test("LSL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsl_w() {
    run_dual_test("LSL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsl_l() {
    run_dual_test("LSL.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsr_b() {
    run_dual_test("LSR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsr_w() {
    run_dual_test("LSR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsr_l() {
    run_dual_test("LSR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rol_b() {
    run_dual_test("ROL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rol_w() {
    run_dual_test("ROL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rol_l() {
    run_dual_test("ROL.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ror_b() {
    run_dual_test("ROR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ror_w() {
    run_dual_test("ROR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ror_l() {
    run_dual_test("ROR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxl_b() {
    run_dual_test("ROXL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxl_w() {
    run_dual_test("ROXL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxl_l() {
    run_dual_test("ROXL.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxr_b() {
    run_dual_test("ROXR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxr_w() {
    run_dual_test("ROXR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxr_l() {
    run_dual_test("ROXR.l", DEFAULT_SAMPLE_LIMIT);
}
