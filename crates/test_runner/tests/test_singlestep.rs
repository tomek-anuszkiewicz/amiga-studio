#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::schema::SingleStepTest;
use common::singlestep_runner::is_cmpm_postinc_opcode;
use common::singlestep_runner::{run_test_file, run_test_file_filtered};

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
    let effective = resolve_limit(limit);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", name);

    let harte_res = run_test_file(&harte_path, effective).unwrap_or_else(|err| {
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
        run_test_file_filtered(&harte_path, effective, "CMPM", filter).unwrap_or_else(|err| {
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

fn test_nop() {
    run_test("NOP", 100);
}

fn test_rts() {
    run_test("RTS", DEFAULT_SAMPLE_LIMIT);
}

fn test_trap() {
    run_test("TRAP", DEFAULT_SAMPLE_LIMIT);
}

fn test_bcc() {
    run_test("Bcc", DEFAULT_SAMPLE_LIMIT);
}

fn test_bsr() {
    run_test("BSR", DEFAULT_SAMPLE_LIMIT);
}

fn test_jmp() {
    run_test("JMP", DEFAULT_SAMPLE_LIMIT);
}

fn test_jsr() {
    run_test("JSR", DEFAULT_SAMPLE_LIMIT);
}

fn test_pea() {
    run_test("PEA", DEFAULT_SAMPLE_LIMIT);
}

fn test_move_w() {
    run_test("MOVE.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_move_q() {
    run_test("MOVE.q", DEFAULT_SAMPLE_LIMIT);
}

fn test_movea_w() {
    run_test("MOVEA.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_movea_l() {
    run_test("MOVEA.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_movem_w() {
    run_test("MOVEM.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_movem_l() {
    run_test("MOVEM.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Integer & Extended Arithmetic (ADD, ADDA, ADDX Archetypes)
// ============================================================================

fn test_add_b() {
    run_test("ADD.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_add_w() {
    run_test("ADD.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_add_l() {
    run_test("ADD.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_adda_w() {
    run_test("ADDA.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_adda_l() {
    run_test("ADDA.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_addx_b() {
    run_test("ADDX.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_addx_w() {
    run_test("ADDX.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_addx_l() {
    run_test("ADDX.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_sub_b() {
    run_test("SUB.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_sub_w() {
    run_test("SUB.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_sub_l() {
    run_test("SUB.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_suba_w() {
    run_test("SUBA.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_suba_l() {
    run_test("SUBA.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_subx_b() {
    run_test("SUBX.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_subx_w() {
    run_test("SUBX.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_subx_l() {
    run_test("SUBX.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Logic Operations (NOT, AND, OR, EOR Archetypes)
// ============================================================================

fn test_not_b() {
    run_test("NOT.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_not_w() {
    run_test("NOT.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_not_l() {
    run_test("NOT.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_and_b() {
    run_test("AND.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_and_w() {
    run_test("AND.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_and_l() {
    run_test("AND.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_or_b() {
    run_test("OR.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_or_w() {
    run_test("OR.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_or_l() {
    run_test("OR.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_eor_b() {
    run_test("EOR.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_eor_w() {
    run_test("EOR.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_eor_l() {
    run_test("EOR.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Bit Manipulation (BSET Archetype)
// ============================================================================

fn test_bset() {
    run_test("BSET", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Shifts & Rotates (ASL Archetype)
// ============================================================================

fn test_asl_b() {
    run_test("ASL.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_asl_w() {
    run_test("ASL.w", DEFAULT_SAMPLE_LIMIT);
}

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

fn test_cmpm_b() {
    run_test_filtered("CMP.b", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

fn test_cmpm_w() {
    run_test_filtered("CMP.w", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

fn test_cmpm_l() {
    run_test_filtered("CMP.l", DEFAULT_SAMPLE_LIMIT, |t| {
        is_cmpm_postinc_opcode(t.initial.prefetch[0])
    });
}

fn test_cmp_b() {
    run_test("CMP.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_cmp_w() {
    run_test("CMP.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_cmp_l() {
    run_test("CMP.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_cmpa_w() {
    run_test("CMPA.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_cmpa_l() {
    run_test("CMPA.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Test (TST Archetype)
// ============================================================================

fn test_tst_b() {
    run_test("TST.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_tst_w() {
    run_test("TST.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_tst_l() {
    run_test("TST.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Shifts & Rotates (Batch 1.4: ASR, LSL, LSR, ROL, ROR, ROXL, ROXR)
// ============================================================================

fn test_asr_b() {
    run_test("ASR.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_asr_w() {
    run_test("ASR.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_asr_l() {
    run_test("ASR.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_lsl_b() {
    run_test("LSL.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_lsl_w() {
    run_test("LSL.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_lsl_l() {
    run_test("LSL.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_lsr_b() {
    run_test("LSR.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_lsr_w() {
    run_test("LSR.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_lsr_l() {
    run_test("LSR.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_rol_b() {
    run_test("ROL.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_rol_w() {
    run_test("ROL.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_rol_l() {
    run_test("ROL.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_ror_b() {
    run_test("ROR.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_ror_w() {
    run_test("ROR.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_ror_l() {
    run_test("ROR.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_roxl_b() {
    run_test("ROXL.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_roxl_w() {
    run_test("ROXL.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_roxl_l() {
    run_test("ROXL.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_roxr_b() {
    run_test("ROXR.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_roxr_w() {
    run_test("ROXR.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_roxr_l() {
    run_test("ROXR.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Bit Manipulation (Batch 1.5: BSET, BTST, BCLR, BCHG)
// ============================================================================

fn test_btst() {
    run_test("BTST", DEFAULT_SAMPLE_LIMIT);
}

fn test_bclr() {
    run_test("BCLR", DEFAULT_SAMPLE_LIMIT);
}

fn test_bchg() {
    run_test("BCHG", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Moves (Batch 1.6: MOVE.B, MOVE.W, MOVE.L)
// ============================================================================

fn test_move_b() {
    run_test("MOVE.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_move_l() {
    run_test("MOVE.l", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Multiplications & Divisions (Batch 1.7: MULU, MULS, DIVU, DIVS)
// ============================================================================

fn test_mulu() {
    run_test("MULU", DEFAULT_SAMPLE_LIMIT);
}

fn test_muls() {
    run_test("MULS", DEFAULT_SAMPLE_LIMIT);
}

fn test_divu() {
    run_test("DIVU", DEFAULT_SAMPLE_LIMIT);
}

fn test_divs() {
    run_test("DIVS", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// BCD & Math Extensions (Batch 1.8: CLR, NEG, NEGX, EXT, ABCD, SBCD, NBCD)
// ============================================================================

fn test_clr_b() {
    run_test("CLR.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_clr_w() {
    run_test("CLR.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_clr_l() {
    run_test("CLR.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_neg_b() {
    run_test("NEG.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_neg_w() {
    run_test("NEG.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_neg_l() {
    run_test("NEG.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_negx_b() {
    run_test("NEGX.b", DEFAULT_SAMPLE_LIMIT);
}

fn test_negx_w() {
    run_test("NEGX.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_negx_l() {
    run_test("NEGX.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_ext_w() {
    run_test("EXT.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_ext_l() {
    run_test("EXT.l", DEFAULT_SAMPLE_LIMIT);
}

fn test_abcd() {
    run_test("ABCD", DEFAULT_SAMPLE_LIMIT);
}

fn test_sbcd() {
    run_test("SBCD", DEFAULT_SAMPLE_LIMIT);
}

fn test_nbcd() {
    run_test("NBCD", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Looping & Conditional Setting (Batch 1.9: DBcc, Scc)
// ============================================================================

fn test_dbcc() {
    run_test("DBcc", DEFAULT_SAMPLE_LIMIT);
}

fn test_scc() {
    run_test("Scc", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Stack & Frame Control (Batch 1.10: SWAP, EXG, LINK, UNLK, LEA, CHK)
// ============================================================================

fn test_swap() {
    run_test("SWAP", DEFAULT_SAMPLE_LIMIT);
}

fn test_exg() {
    run_test("EXG", DEFAULT_SAMPLE_LIMIT);
}

fn test_link() {
    run_test("LINK", DEFAULT_SAMPLE_LIMIT);
}

fn test_unlink() {
    run_test("UNLINK", DEFAULT_SAMPLE_LIMIT);
}

fn test_lea() {
    run_test("LEA", DEFAULT_SAMPLE_LIMIT);
}

fn test_chk() {
    run_test("CHK", DEFAULT_SAMPLE_LIMIT);
}

// ============================================================================
// Privileged & Atomic Hardware Ops (Batch 1.11)
// ============================================================================

fn test_move_to_ccr() {
    run_test("MOVEtoCCR", DEFAULT_SAMPLE_LIMIT);
}

fn test_move_to_sr() {
    run_test("MOVEtoSR", DEFAULT_SAMPLE_LIMIT);
}

fn test_move_from_sr() {
    run_test("MOVEfromSR", DEFAULT_SAMPLE_LIMIT);
}

fn test_move_to_usp() {
    run_test("MOVEtoUSP", DEFAULT_SAMPLE_LIMIT);
}

fn test_move_from_usp() {
    run_test("MOVEfromUSP", DEFAULT_SAMPLE_LIMIT);
}

fn test_stop() {
    use cpu::Cpu;
    use physical_memory::PhysicalMemory;

    let mut cpu = Cpu::new();
    let mut bus = PhysicalMemory::new();
    cpu.state.set_supervisor(true);
    cpu.state.set_sr(0x2700);
    cpu.state.ir = 0x4E72; // STOP #$2000
    cpu.state.prefetch = 0x2000;

    cpu.step_instruction(&mut bus);
    assert!(cpu.state.stopped);
    assert_eq!(cpu.state.sr(), 0x2000);
}

fn test_reset() {
    run_test("RESET", DEFAULT_SAMPLE_LIMIT);
}

fn test_tas() {
    run_test("TAS", DEFAULT_SAMPLE_LIMIT);
}

fn test_rte() {
    run_test("RTE", DEFAULT_SAMPLE_LIMIT);
}

fn test_rtr() {
    run_test("RTR", DEFAULT_SAMPLE_LIMIT);
}

fn test_trapv() {
    run_test("TRAPV", DEFAULT_SAMPLE_LIMIT);
}

fn test_andi_to_ccr() {
    run_test("ANDItoCCR", DEFAULT_SAMPLE_LIMIT);
}

fn test_andi_to_sr() {
    run_test("ANDItoSR", DEFAULT_SAMPLE_LIMIT);
}

fn test_ori_to_ccr() {
    run_test("ORItoCCR", DEFAULT_SAMPLE_LIMIT);
}

fn test_ori_to_sr() {
    run_test("ORItoSR", DEFAULT_SAMPLE_LIMIT);
}

fn test_eori_to_ccr() {
    run_test("EORItoCCR", DEFAULT_SAMPLE_LIMIT);
}

fn test_eori_to_sr() {
    run_test("EORItoSR", DEFAULT_SAMPLE_LIMIT);
}

// Both MOVEP reference files were previously absent from the silicon suite.
fn test_movep_w() {
    run_test("MOVEP.w", DEFAULT_SAMPLE_LIMIT);
}

fn test_movep_l() {
    run_test("MOVEP.l", DEFAULT_SAMPLE_LIMIT);
}

/// Runs every selected group before finalizing reports and asserting the result.
#[test]
fn test_singlestep() {
    common::singlestep_suite::run_suite(&[
        (stringify!(test_nop), test_nop),
        (stringify!(test_rts), test_rts),
        (stringify!(test_trap), test_trap),
        (stringify!(test_bcc), test_bcc),
        (stringify!(test_bsr), test_bsr),
        (stringify!(test_jmp), test_jmp),
        (stringify!(test_jsr), test_jsr),
        (stringify!(test_pea), test_pea),
        (stringify!(test_move_w), test_move_w),
        (stringify!(test_move_q), test_move_q),
        (stringify!(test_movea_w), test_movea_w),
        (stringify!(test_movea_l), test_movea_l),
        (stringify!(test_movem_w), test_movem_w),
        (stringify!(test_movem_l), test_movem_l),
        (stringify!(test_add_b), test_add_b),
        (stringify!(test_add_w), test_add_w),
        (stringify!(test_add_l), test_add_l),
        (stringify!(test_adda_w), test_adda_w),
        (stringify!(test_adda_l), test_adda_l),
        (stringify!(test_addx_b), test_addx_b),
        (stringify!(test_addx_w), test_addx_w),
        (stringify!(test_addx_l), test_addx_l),
        (stringify!(test_sub_b), test_sub_b),
        (stringify!(test_sub_w), test_sub_w),
        (stringify!(test_sub_l), test_sub_l),
        (stringify!(test_suba_w), test_suba_w),
        (stringify!(test_suba_l), test_suba_l),
        (stringify!(test_subx_b), test_subx_b),
        (stringify!(test_subx_w), test_subx_w),
        (stringify!(test_subx_l), test_subx_l),
        (stringify!(test_not_b), test_not_b),
        (stringify!(test_not_w), test_not_w),
        (stringify!(test_not_l), test_not_l),
        (stringify!(test_and_b), test_and_b),
        (stringify!(test_and_w), test_and_w),
        (stringify!(test_and_l), test_and_l),
        (stringify!(test_or_b), test_or_b),
        (stringify!(test_or_w), test_or_w),
        (stringify!(test_or_l), test_or_l),
        (stringify!(test_eor_b), test_eor_b),
        (stringify!(test_eor_w), test_eor_w),
        (stringify!(test_eor_l), test_eor_l),
        (stringify!(test_bset), test_bset),
        (stringify!(test_asl_b), test_asl_b),
        (stringify!(test_asl_w), test_asl_w),
        (stringify!(test_asl_l), test_asl_l),
        (stringify!(test_cmpm_b), test_cmpm_b),
        (stringify!(test_cmpm_w), test_cmpm_w),
        (stringify!(test_cmpm_l), test_cmpm_l),
        (stringify!(test_cmp_b), test_cmp_b),
        (stringify!(test_cmp_w), test_cmp_w),
        (stringify!(test_cmp_l), test_cmp_l),
        (stringify!(test_cmpa_w), test_cmpa_w),
        (stringify!(test_cmpa_l), test_cmpa_l),
        (stringify!(test_tst_b), test_tst_b),
        (stringify!(test_tst_w), test_tst_w),
        (stringify!(test_tst_l), test_tst_l),
        (stringify!(test_asr_b), test_asr_b),
        (stringify!(test_asr_w), test_asr_w),
        (stringify!(test_asr_l), test_asr_l),
        (stringify!(test_lsl_b), test_lsl_b),
        (stringify!(test_lsl_w), test_lsl_w),
        (stringify!(test_lsl_l), test_lsl_l),
        (stringify!(test_lsr_b), test_lsr_b),
        (stringify!(test_lsr_w), test_lsr_w),
        (stringify!(test_lsr_l), test_lsr_l),
        (stringify!(test_rol_b), test_rol_b),
        (stringify!(test_rol_w), test_rol_w),
        (stringify!(test_rol_l), test_rol_l),
        (stringify!(test_ror_b), test_ror_b),
        (stringify!(test_ror_w), test_ror_w),
        (stringify!(test_ror_l), test_ror_l),
        (stringify!(test_roxl_b), test_roxl_b),
        (stringify!(test_roxl_w), test_roxl_w),
        (stringify!(test_roxl_l), test_roxl_l),
        (stringify!(test_roxr_b), test_roxr_b),
        (stringify!(test_roxr_w), test_roxr_w),
        (stringify!(test_roxr_l), test_roxr_l),
        (stringify!(test_btst), test_btst),
        (stringify!(test_bclr), test_bclr),
        (stringify!(test_bchg), test_bchg),
        (stringify!(test_move_b), test_move_b),
        (stringify!(test_move_l), test_move_l),
        (stringify!(test_mulu), test_mulu),
        (stringify!(test_muls), test_muls),
        (stringify!(test_divu), test_divu),
        (stringify!(test_divs), test_divs),
        (stringify!(test_clr_b), test_clr_b),
        (stringify!(test_clr_w), test_clr_w),
        (stringify!(test_clr_l), test_clr_l),
        (stringify!(test_neg_b), test_neg_b),
        (stringify!(test_neg_w), test_neg_w),
        (stringify!(test_neg_l), test_neg_l),
        (stringify!(test_negx_b), test_negx_b),
        (stringify!(test_negx_w), test_negx_w),
        (stringify!(test_negx_l), test_negx_l),
        (stringify!(test_ext_w), test_ext_w),
        (stringify!(test_ext_l), test_ext_l),
        (stringify!(test_abcd), test_abcd),
        (stringify!(test_sbcd), test_sbcd),
        (stringify!(test_nbcd), test_nbcd),
        (stringify!(test_dbcc), test_dbcc),
        (stringify!(test_scc), test_scc),
        (stringify!(test_swap), test_swap),
        (stringify!(test_exg), test_exg),
        (stringify!(test_link), test_link),
        (stringify!(test_unlink), test_unlink),
        (stringify!(test_lea), test_lea),
        (stringify!(test_chk), test_chk),
        (stringify!(test_move_to_ccr), test_move_to_ccr),
        (stringify!(test_move_to_sr), test_move_to_sr),
        (stringify!(test_move_from_sr), test_move_from_sr),
        (stringify!(test_move_to_usp), test_move_to_usp),
        (stringify!(test_move_from_usp), test_move_from_usp),
        (stringify!(test_stop), test_stop),
        (stringify!(test_reset), test_reset),
        (stringify!(test_tas), test_tas),
        (stringify!(test_rte), test_rte),
        (stringify!(test_rtr), test_rtr),
        (stringify!(test_trapv), test_trapv),
        (stringify!(test_andi_to_ccr), test_andi_to_ccr),
        (stringify!(test_andi_to_sr), test_andi_to_sr),
        (stringify!(test_ori_to_ccr), test_ori_to_ccr),
        (stringify!(test_ori_to_sr), test_ori_to_sr),
        (stringify!(test_eori_to_ccr), test_eori_to_ccr),
        (stringify!(test_eori_to_sr), test_eori_to_sr),
        (stringify!(test_movep_w), test_movep_w),
        (stringify!(test_movep_l), test_movep_l),
    ])
    .expect("SingleStep verification failed; reports and diagnostics contain the group results");
}
