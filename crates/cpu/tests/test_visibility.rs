#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Verification of M68000 public API surface and encapsulation integrity

use cpu::{Cpu, OPCODE_DESCRIPTOR_TABLE};

#[test]
fn test_m68000_public_api_surface_and_encapsulation() {
    let cpu = Cpu::new();
    assert_eq!(cpu.state.pc, 0);

    // Verify opcode descriptor table accessibility while internal instruction submodules remain encapsulated
    let nop_desc = &OPCODE_DESCRIPTOR_TABLE[0x4E71];
    assert!(!nop_desc.steps.is_empty());

    let move_b_desc = &OPCODE_DESCRIPTOR_TABLE[0x1000]; // MOVE.B D0, D0
    assert!(!move_b_desc.steps.is_empty());
    assert_eq!(move_b_desc.reg_src, 0);
    assert_eq!(move_b_desc.reg_dst, 0);
}

#[test]
fn test_m68000_core_rehydration_and_state() {
    let mut cpu = Cpu::new();
    cpu.state.ir = 0x4E71; // NOP
    cpu.rehydrate_micro_steps();
    assert!(!cpu.state.micro.current_steps.is_empty());
}

#[test]
fn test_condition_gt_evaluation() {
    let mut cpu = Cpu::new();
    // GT condition code is 0x0E: (N == V) && !Z
    // 1. N=0, V=0, Z=0 -> true
    cpu.state.set_ccr_nzvc(false, false, false, false);
    assert!(cpu.state.eval_condition(0x0E));

    // 2. N=1, V=1, Z=0 -> true
    cpu.state.set_ccr_nzvc(true, false, true, false);
    assert!(cpu.state.eval_condition(0x0E));

    // 3. N=1, V=0, Z=0 -> false
    cpu.state.set_ccr_nzvc(true, false, false, false);
    assert!(!cpu.state.eval_condition(0x0E));

    // 4. Z=1 -> false
    cpu.state.set_ccr_nzvc(false, true, false, false);
    assert!(!cpu.state.eval_condition(0x0E));
}
