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
