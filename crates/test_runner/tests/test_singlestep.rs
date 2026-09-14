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

/// Helper function to execute a test against Real 68k (Tom Harte) hardware test suite
fn run_test(name: &str, limit: usize) {
    run_test_with_mode(name, limit, VerifyMode::StateOnly);
}

/// Helper function to execute a test against Tom Harte hardware suite with specified verification mode
fn run_test_with_mode(name: &str, limit: usize, mode: VerifyMode) {
    let effective = resolve_limit(limit);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", name);

    let harte_res = run_test_file_with_mode(&harte_path, effective, mode).unwrap_or_else(|err| {
        panic!(
            "Failed to open/parse Real 68k test '{}': {}",
            harte_path, err
        )
    });
    let (passed, failed) = harte_res;
    assert!(
        passed > 0,
        "No Real 68k tests passed for {} (total executed: {})",
        name,
        passed + failed
    );
    assert_eq!(
        failed,
        0,
        "Real 68k (Tom Harte) tests failed for {}: {}/{} failed",
        name,
        failed,
        passed + failed
    );
}

/// Helper function to execute a filtered test against Tom Harte hardware suite
fn run_test_filtered<F>(name: &str, limit: usize, filter: F)
where
    F: Fn(&SingleStepTest) -> bool + Copy,
{
    let effective = resolve_limit(limit);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", name);

    let harte_res =
        run_test_file_filtered_with_mode(&harte_path, effective, VerifyMode::StateOnly, filter)
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to open/parse Real 68k test '{}': {}",
                    harte_path, err
                )
            });
    let (passed, failed) = harte_res;
    assert!(
        passed > 0,
        "No Real 68k tests passed for {} (total executed: {})",
        name,
        passed + failed
    );
    assert_eq!(
        failed,
        0,
        "Real 68k (Tom Harte) tests failed for {}: {}/{} failed",
        name,
        failed,
        passed + failed
    );
}

// ============================================================================
// System, Control Flow & Exceptions (Microcode Archetypes)
// ============================================================================

#[test]
fn test_nop() {
    run_test_with_mode("NOP", 100, VerifyMode::Full);
}

#[test]
fn test_rts() {
    run_test("RTS", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_trap() {
    run_test_with_mode("TRAP", DEFAULT_SAMPLE_LIMIT, VerifyMode::Full);
}

#[test]
fn test_bcc() {
    run_test("Bcc", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bsr() {
    run_test("BSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_jmp() {
    run_test("JMP", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_jsr() {
    run_test("JSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_pea() {
    run_test("PEA", DEFAULT_SAMPLE_LIMIT);
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
    run_test("MOVE.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_q() {
    run_test("MOVE.q", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movea_w() {
    run_test("MOVEA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movea_l() {
    run_test("MOVEA.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movem_w() {
    run_test("MOVEM.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_movem_l() {
    run_test("MOVEM.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Integer & Extended Arithmetic (ADD, ADDA, ADDX Archetypes)
// ============================================================================

#[test]
fn test_add_b() {
    run_test("ADD.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_add_w() {
    run_test("ADD.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_add_l() {
    run_test("ADD.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_adda_w() {
    run_test("ADDA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_adda_l() {
    run_test("ADDA.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_addx_b() {
    run_test("ADDX.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_addx_w() {
    run_test("ADDX.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_addx_l() {
    run_test("ADDX.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_sub_b() {
    run_test("SUB.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_sub_w() {
    run_test("SUB.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_sub_l() {
    run_test("SUB.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_suba_w() {
    run_test("SUBA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_suba_l() {
    run_test("SUBA.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_subx_b() {
    run_test("SUBX.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_subx_w() {
    run_test("SUBX.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_subx_l() {
    run_test("SUBX.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Logic Operations (NOT, AND, OR, EOR Archetypes)
// ============================================================================

#[test]
fn test_not_b() {
    run_test("NOT.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_not_w() {
    run_test("NOT.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_not_l() {
    run_test("NOT.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_and_b() {
    run_test("AND.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_and_w() {
    run_test("AND.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_and_l() {
    run_test("AND.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_or_b() {
    run_test("OR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_or_w() {
    run_test("OR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_or_l() {
    run_test("OR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eor_b() {
    run_test("EOR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eor_w() {
    run_test("EOR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eor_l() {
    run_test("EOR.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Bit Manipulation (BSET Archetype)
// ============================================================================

#[test]
fn test_bset() {
    run_test("BSET", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Shifts & Rotates (ASL Archetype)
// ============================================================================

#[test]
fn test_asl_b() {
    run_test("ASL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asl_w() {
    run_test("ASL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asl_l() {
    run_test("ASL.l", DEFAULT_SAMPLE_LIMIT);
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
    run_test_filtered("CMP.b", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

#[test]
fn test_cmpm_w() {
    run_test_filtered("CMP.w", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

#[test]
fn test_cmpm_l() {
    run_test_filtered("CMP.l", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

#[test]
fn test_cmp_b() {
    run_test("CMP.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmp_w() {
    run_test("CMP.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmp_l() {
    run_test("CMP.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmpa_w() {
    run_test("CMPA.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_cmpa_l() {
    run_test("CMPA.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Test (TST Archetype)
// ============================================================================

#[test]
fn test_tst_b() {
    run_test("TST.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_tst_w() {
    run_test("TST.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_tst_l() {
    run_test("TST.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Shifts & Rotates (Batch 1.4: ASR, LSL, LSR, ROL, ROR, ROXL, ROXR)
// ============================================================================

#[test]
fn test_asr_b() {
    run_test("ASR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asr_w() {
    run_test("ASR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_asr_l() {
    run_test("ASR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsl_b() {
    run_test("LSL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsl_w() {
    run_test("LSL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsl_l() {
    run_test("LSL.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsr_b() {
    run_test("LSR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsr_w() {
    run_test("LSR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lsr_l() {
    run_test("LSR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rol_b() {
    run_test("ROL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rol_w() {
    run_test("ROL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rol_l() {
    run_test("ROL.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ror_b() {
    run_test("ROR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ror_w() {
    run_test("ROR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ror_l() {
    run_test("ROR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxl_b() {
    run_test("ROXL.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxl_w() {
    run_test("ROXL.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxl_l() {
    run_test("ROXL.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxr_b() {
    run_test("ROXR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxr_w() {
    run_test("ROXR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_roxr_l() {
    run_test("ROXR.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Bit Manipulation (Batch 1.5: BSET, BTST, BCLR, BCHG)
// ============================================================================

#[test]
fn test_btst() {
    run_test("BTST", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bclr() {
    run_test("BCLR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_bchg() {
    run_test("BCHG", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Moves (Batch 1.6: MOVE.B, MOVE.W, MOVE.L)
// ============================================================================

#[test]
fn test_move_b() {
    run_test("MOVE.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_l() {
    run_test("MOVE.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Multiplications & Divisions (Batch 1.7: MULU, MULS, DIVU, DIVS)
// ============================================================================

#[test]
fn test_mulu() {
    run_test("MULU", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_muls() {
    run_test("MULS", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_divu() {
    run_test("DIVU", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_divs() {
    run_test("DIVS", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// BCD & Math Extensions (Batch 1.8: CLR, NEG, NEGX, EXT, ABCD, SBCD, NBCD)
// ============================================================================

#[test]
fn test_clr_b() {
    run_test("CLR.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_clr_w() {
    run_test("CLR.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_clr_l() {
    run_test("CLR.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_neg_b() {
    run_test("NEG.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_neg_w() {
    run_test("NEG.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_neg_l() {
    run_test("NEG.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_negx_b() {
    run_test("NEGX.b", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_negx_w() {
    run_test("NEGX.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_negx_l() {
    run_test("NEGX.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ext_w() {
    run_test("EXT.w", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ext_l() {
    run_test("EXT.l", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_abcd() {
    run_test("ABCD", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_sbcd() {
    run_test("SBCD", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_nbcd() {
    run_test("NBCD", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Looping & Conditional Setting (Batch 1.9: DBcc, Scc)
// ============================================================================

#[test]
fn test_dbcc() {
    run_test("DBcc", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_scc() {
    run_test("Scc", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Stack & Frame Control (Batch 1.10: SWAP, EXG, LINK, UNLK, LEA, CHK)
// ============================================================================

#[test]
fn test_swap() {
    run_test("SWAP", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_exg() {
    run_test("EXG", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_link() {
    run_test("LINK", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_unlink() {
    run_test("UNLINK", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_lea() {
    run_test("LEA", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_chk() {
    run_test("CHK", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Privileged & Atomic Hardware Ops (Batch 1.11)
// ============================================================================

#[test]
fn test_move_to_ccr() {
    run_test("MOVEtoCCR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_to_sr() {
    run_test("MOVEtoSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_from_sr() {
    run_test("MOVEfromSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_to_usp() {
    run_test("MOVEtoUSP", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_move_from_usp() {
    run_test("MOVEfromUSP", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_stop() {
    use m68000::Cpu;
    use physical_memory::MemoryBus;

    let mut cpu = Cpu::new();
    let mut bus = MemoryBus::new();
    cpu.state.set_supervisor(true);
    cpu.state.set_sr(0x2700);
    cpu.state.ir = 0x4E72; // STOP #$2000
    cpu.state.prefetch[0] = 0x2000;
    cpu.state.prefetch[1] = 0x4E71;

    cpu.step_instruction(&mut bus);
    assert!(cpu.state.stopped);
    assert_eq!(cpu.state.sr, 0x2000);
}

#[test]
fn test_reset() {
    run_test("RESET", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_tas() {
    run_test("TAS", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rte() {
    run_test("RTE", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_rtr() {
    run_test("RTR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_trapv() {
    run_test("TRAPV", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_andi_to_ccr() {
    run_test("ANDItoCCR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_andi_to_sr() {
    run_test("ANDItoSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ori_to_ccr() {
    run_test("ORItoCCR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_ori_to_sr() {
    run_test("ORItoSR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eori_to_ccr() {
    run_test("EORItoCCR", DEFAULT_SAMPLE_LIMIT);
}

#[test]
fn test_eori_to_sr() {
    run_test("EORItoSR", DEFAULT_SAMPLE_LIMIT);
}
