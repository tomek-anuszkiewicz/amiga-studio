use m68000::micro::ea;
use m68000::{Cpu, CpuState, StepResult};
use memory_bus::MemoryBus;

#[test]
fn test_addressing_modes_and_a7_byte_quirk() {
    let mut state = CpuState::default();
    state.set_a_long(0, 0x001000);
    state.write_a(7, 0x002000); // SP

    // (A0)+ with Byte size increments by 1
    ea::ea_calc_src_pi_b(&mut state, 0, 0);
    assert_eq!(state.micro.ea_addr, 0x001000);
    assert_eq!(state.a_long(0), 0x001001);

    // -(A0) with Byte size decrements by 1
    ea::ea_calc_src_pd_b(&mut state, 0, 0);
    assert_eq!(state.micro.ea_addr, 0x001000);
    assert_eq!(state.a_long(0), 0x001000);

    // CRITICAL QUIRK: (A7)+ with Byte size MUST adjust by 2 (preserving word alignment)!
    ea::ea_calc_src_pi_b(&mut state, 7, 0);
    assert_eq!(state.micro.ea_addr, 0x002000);
    assert_eq!(state.read_a(7), 0x002002);

    // -(A7) with Byte size MUST adjust by 2!
    ea::ea_calc_src_pd_b(&mut state, 7, 0);
    assert_eq!(state.micro.ea_addr, 0x002000);
    assert_eq!(state.read_a(7), 0x002000);
}

#[test]
fn test_indexed_addressing_mode() {
    let mut state = CpuState::default();
    state.set_a_long(1, 0x004000);
    state.set_d_long(2, 0x0000_0020); // Index +32

    // Brief extension word: D2.L, disp8 = -4 (0xFC)
    // Bit 15: 0 (Data reg), Bits 14-12: 010 (D2), Bit 11: 1 (Long), Bits 7-0: 0xFC (-4)
    let ext = (0 << 15) | (2 << 12) | (1 << 11) | ((-4i8 as u8) as u16);
    state.prefetch[0] = ext;
    ea::ea_calc_src_idx_an(&mut state, 1, 0);
    assert_eq!(state.micro.ea_addr, 0x004000 + 32 - 4);
}

#[test]
fn test_unaligned_address_error() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    let mut cpu = Cpu::new();
    let res = cpu.trigger_address_error_step(
        0x001001,
        true,
        false,
        &mut bus,
    );
    assert_eq!(res, StepResult::InstructionCompleted);
}

#[test]
fn test_move_instruction() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    cpu.state.set_d_long(0, 0x0000_1234);

    // MOVE.W D0, D1 (Opcode: 0x3200)
    cpu.state.ir = 0x3200;
    cpu.state.prefetch[0] = 0x4E71; // NOP
    cpu.state.pc = 0x001004;

    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d_word(1), 0x1234);
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
    cpu.state.set_d_long(0, 5);
    cpu.state.set_d_long(1, 10);

    // ADD.L D0, D1 (Opcode: 0xD280)
    cpu.state.ir = 0xD280;
    cpu.state.prefetch[0] = 0x4E71;
    cpu.state.pc = 0x001004;

    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d_long(1), 15);
    assert!(!cpu.state.get_c());
    assert!(!cpu.state.get_z());

    // SUB.L D1, D0 (5 - 15 = -10, borrow sets C and X, sets N) (Opcode: 0x9081)
    cpu.state.micro.reset();
    cpu.state.ir = 0x9081;
    cpu.state.prefetch[0] = 0x4E71;
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d_long(0), (-10i32) as u32);
    assert!(cpu.state.get_n());
    assert!(cpu.state.get_c());
    assert!(cpu.state.get_x());
}

#[test]
fn test_nop_and_branch() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    // NOP is at 0x0FFE, prefetch (BRA.S) is at 0x1000, so PC starts at 0x1002
    cpu.state.pc = 0x001002;

    // NOP (0x4E71)
    cpu.state.ir = 0x4E71;
    cpu.state.prefetch[0] = 0x6004; // BRA.S +4 at 0x1000
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    // After NOP retires, ir should be the prefetch (BRA.S 0x6004)
    assert_eq!(cpu.state.ir, 0x6004);

    // Execute BRA.S +4 (Opcode: 0x6004)
    // Target = base_pc + 2 + displacement = 0x1000 + 2 + 4 = 0x1006
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.pc, 0x001006 + 4); // +4 because reload_pc_and_prefetch fetched 2 words
}

#[test]
fn test_logic_and_shifts() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    cpu.state.set_d_long(0, 0x0000_F0F0);
    cpu.state.set_d_long(1, 0x0000_FF00);

    // AND.W D0, D1 -> D1.W = 0xF000 (Opcode: 0xC240)
    cpu.state.ir = 0xC240;
    cpu.state.prefetch[0] = 0x4E71;
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d_word(1), 0xF000);
    assert!(cpu.state.get_n());
    assert!(!cpu.state.get_z());

    // OR.W D0, D1 -> D1.W = 0xFFF0 (Opcode: 0x8240)
    cpu.state.micro.reset();
    cpu.state.set_d_long(0, 0x0000_00F0);
    cpu.state.set_d_long(1, 0x0000_FF00);
    cpu.state.ir = 0x8240;
    cpu.state.prefetch[0] = 0x4E71;
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d_word(1), 0xFFF0);

    // LSL.W #2, D1 (Opcode: 0xE549) (count=2, LSL, Word, reg 1)
    cpu.state.micro.reset();
    cpu.state.set_d_long(1, 0x0000_0003);
    cpu.state.ir = 0xE549;
    cpu.state.prefetch[0] = 0x4E71;
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d_word(1), 0x000C);
}

#[test]
fn test_bit_manipulation() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    let mut cpu = Cpu::new();
    cpu.state.set_d_long(0, 0x0000_0004); // bit 4 is set: 0x10
    cpu.state.set_d_long(1, 0x0000_0010);

    // BTST D0, D1 (Opcode: 0x0101) (dyn bit, D0=bit 4, D1=target)
    cpu.state.ir = 0x0101;
    cpu.state.prefetch[0] = 0x4E71;
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert!(!cpu.state.get_z()); // bit 4 was 1, so Z=0

    // BCLR D0, D1 -> bit 4 cleared, D1 becomes 0 (Opcode: 0x0181)
    cpu.state.micro.reset();
    cpu.state.ir = 0x0181;
    cpu.state.prefetch[0] = 0x4E71;
    let res = cpu.step_instruction(&mut bus);
    assert_eq!(res, StepResult::InstructionCompleted);
    assert_eq!(cpu.state.d_long(1), 0);
}
