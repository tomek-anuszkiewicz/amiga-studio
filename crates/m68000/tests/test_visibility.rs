//! Verification of M68000 public API surface and encapsulation integrity

use m68000::{Cpu, CpuState, Size, OPCODE_DESCRIPTOR_TABLE};

#[test]
fn test_m68000_public_api_surface_and_encapsulation() {
    let mut cpu = Cpu::new();
    assert_eq!(cpu.state.pc, 0);

    // Verify opcode descriptor table accessibility while internal instruction submodules remain encapsulated
    let nop_desc = &OPCODE_DESCRIPTOR_TABLE[0x4E71];
    assert_eq!(nop_desc.mnemonic, "NOP");
    assert_eq!(nop_desc.size, Size::None);

    let move_b_desc = &OPCODE_DESCRIPTOR_TABLE[0x1000]; // MOVE.B D0, D0
    assert_eq!(move_b_desc.mnemonic, "MOVE");
    assert_eq!(move_b_desc.size, Size::Byte);
}

#[test]
fn test_m68000_core_rehydration_and_state() {
    let mut cpu = Cpu::new();
    cpu.state.ir = 0x4E71; // NOP
    cpu.rehydrate_micro_steps();
    assert!(!cpu.state.micro.current_steps.is_empty());
}
