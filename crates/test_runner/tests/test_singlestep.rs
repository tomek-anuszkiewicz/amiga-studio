use test_runner::runner::run_test_file;

/// Default number of test cases to run per opcode from each test suite
const DEFAULT_SAMPLE_LIMIT: usize = 50;

/// Helper function to execute a test against both MAME and Real 68k (Tom Harte) suites
fn run_dual_test(name: &str, limit: usize) {
    let mame_path = format!("ref_src/SingleStepTests-m68000/v1/{}.json", name);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json.gz", name);

    // 1. MAME SingleStepTests suite
    let mame_res = run_test_file(&mame_path, Some(limit))
        .unwrap_or_else(|err| panic!("Failed to open/parse MAME test '{}': {}", mame_path, err));
    let (mame_passed, mame_failed) = mame_res;
    assert!(
        mame_passed > 0,
        "No MAME tests passed for {} (total executed: {})",
        name,
        mame_passed + mame_failed
    );
    assert_eq!(
        mame_failed, 0,
        "MAME tests failed for {}: {}/{} failed",
        name,
        mame_failed,
        mame_passed + mame_failed
    );

    // 2. Real 68k (Tom Harte) SingleStepTests-680x0 suite
    let harte_res = run_test_file(&harte_path, Some(limit))
        .unwrap_or_else(|err| panic!("Failed to open/parse Real 68k test '{}': {}", harte_path, err));
    let (harte_passed, harte_failed) = harte_res;
    assert!(
        harte_passed > 0,
        "No Real 68k tests passed for {} (total executed: {})",
        name,
        harte_passed + harte_failed
    );
    assert_eq!(
        harte_failed, 0,
        "Real 68k (Tom Harte) tests failed for {}: {}/{} failed",
        name,
        harte_failed,
        harte_passed + harte_failed
    );
}

// ============================================================================
// System, Control Flow & Exceptions
// ============================================================================

#[test]
fn test_nop() {
    run_dual_test("NOP", 100);
}

#[test]
fn test_rts() {
    run_dual_test("RTS", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_trap() {
    run_dual_test("TRAP", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bcc() {
    run_dual_test("Bcc", DEFAULT_SAMPLE_LIMIT);
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

// ============================================================================
// Data Movement (MOVE & MOVEA)
// ============================================================================

#[test]
fn test_move_b() {
    run_dual_test("MOVE.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_w() {
    run_dual_test("MOVE.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_l() {
    run_dual_test("MOVE.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movea_w() {
    run_dual_test("MOVEA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movea_l() {
    run_dual_test("MOVEA.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Integer Arithmetic (ADD, ADDA, SUB, SUBA)
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

// ============================================================================
// Logical Operations (AND, OR)
// ============================================================================

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

// ============================================================================
// Bit Manipulation (BTST, BSET, BCLR, BCHG)
// ============================================================================

#[test]
fn test_btst() {
    run_dual_test("BTST", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bset() {
    run_dual_test("BSET", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bclr() {
    run_dual_test("BCLR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bchg() {
    run_dual_test("BCHG", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Shifts & Rotates (ASL, ASR, LSL, LSR)
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

// ============================================================================
// Compare & Test Operations (Milestone 1.1)
// ============================================================================

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

