use m68000::addressing::{AddressingMode, IndexReg, IndexType, Size};
use m68000::{Cpu, CpuState, StepResult};
use memory_bus::MemoryBus;

#[test]
fn test_addressing_modes_and_a7_byte_quirk() {
    let mut state = CpuState::default();
    state.a[0] = 0x001000;
    state.set_a7(0x002000); // SP

    // (A0)+ with Byte size increments by 1
    let ea_a0_byte = AddressingMode::Postincrement(0);
    let addr = ea_a0_byte.resolve_address(&mut state, Size::Byte).unwrap();
    assert_eq!(addr, 0x001000);
    assert_eq!(state.a[0], 0x001001);

    // -(A0) with Byte size decrements by 1
    let ea_a0_predec = AddressingMode::Predecrement(0);
    let addr = ea_a0_predec.resolve_address(&mut state, Size::Byte).unwrap();
    assert_eq!(addr, 0x001000);
    assert_eq!(state.a[0], 0x001000);

    // CRITICAL QUIRK: (A7)+ with Byte size MUST adjust by 2 (preserving word alignment)!
    let ea_sp_byte = AddressingMode::Postincrement(7);
    let addr = ea_sp_byte.resolve_address(&mut state, Size::Byte).unwrap();
    assert_eq!(addr, 0x002000);
    assert_eq!(state.a7(), 0x002002);

    // -(A7) with Byte size MUST adjust by 2!
    let ea_sp_predec = AddressingMode::Predecrement(7);
    let addr = ea_sp_predec.resolve_address(&mut state, Size::Byte).unwrap();
    assert_eq!(addr, 0x002000);
    assert_eq!(state.a7(), 0x002000);
}

#[test]
fn test_indexed_addressing_mode() {
    let mut state = CpuState::default();
    state.a[1] = 0x004000;
    state.d[2] = 0x0000_0020; // Index +32

    let index = IndexReg {
        reg_type: IndexType::Data,
        reg_idx: 2,
        is_long: true,
    };
    // (d8, A1, D2.L) with d8 = -4
    let ea = AddressingMode::Indexed(1, index, -4);
    let addr = ea.resolve_address(&mut state, Size::Word).unwrap();
    assert_eq!(addr, 0x004000 + 32 - 4);
}

#[test]
fn test_unaligned_address_error() {
    let mut state = CpuState::default();
    state.a[0] = 0x001001; // Odd address!

    let ea = AddressingMode::AddressIndirect(0);
    // Word access to odd address must trigger AddressError
    let res = ea.resolve_address(&mut state, Size::Word);
    assert!(res.is_err());
}

#[test]
fn test_move_instruction() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    cpu.state.d[0] = 0x0000_1234;

    // MOVE.W D0, D1 (Opcode: 0x3200)
    cpu.state.ir = 0x3200;
    cpu.state.prefetch[0] = 0x4E71; // NOP
    cpu.state.pc = 0x001004;

    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d[1] & 0xFFFF, 0x1234);
    assert!(!cpu.state.get_n());
    assert!(!cpu.state.get_z());
    assert!(!cpu.state.get_v());
    assert!(!cpu.state.get_c());
}

#[test]
fn test_add_sub_ccr() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    cpu.state.d[0] = 5;
    cpu.state.d[1] = 10;

    // ADD.L D0, D1 (Opcode: 0xD280)
    cpu.state.ir = 0xD280;
    cpu.state.prefetch[0] = 0x4E71;
    cpu.state.pc = 0x001004;

    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d[1], 15);
    assert!(!cpu.state.get_c());
    assert!(!cpu.state.get_z());

    // SUB.L D1, D0 (5 - 15 = -10, borrow sets C and X, sets N) (Opcode: 0x9081)
    cpu.state.ir = 0x9081;
    cpu.state.prefetch[0] = 0x4E71;
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d[0], (-10i32) as u32);
    assert!(cpu.state.get_n());
    assert!(cpu.state.get_c());
    assert!(cpu.state.get_x());
}
