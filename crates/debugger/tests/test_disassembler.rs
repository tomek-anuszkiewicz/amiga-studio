use debugger::disassemble;

#[test]
fn test_disassemble_inherent_and_control() {
    let mem: [u16; 10] = [
        0x4E71, // NOP
        0x4E75, // RTS
        0x4E73, // RTE
        0x4E77, // RTR
        0x4E70, // RESET
        0x4E76, // TRAPV
        0x4AFC, // ILLEGAL
        0x4E72, 0x2700, // STOP #$2700
        0x4E42, // TRAP #2
    ];

    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "NOP");
    assert_eq!(b, 2);
    assert_eq!(d.format_line(), "00001000: 4E71             NOP");

    let (d, b) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "RTS");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "RTE");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "RTR");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "RESET");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x100A, read);
    assert_eq!(d.mnemonic, "TRAPV");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x100C, read);
    assert_eq!(d.mnemonic, "ILLEGAL");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x100E, read);
    assert_eq!(d.mnemonic, "STOP");
    assert_eq!(d.operands, "#$2700");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1012, read);
    assert_eq!(d.mnemonic, "TRAP");
    assert_eq!(d.operands, "#2");
    assert_eq!(b, 2);
}

#[test]
fn test_disassemble_stack_and_registers() {
    let mem: [u16; 11] = [
        0x4E56, 0xFFF0, // LINK A6, #-16
        0x4E5E, // UNLK A6
        0x4E68, // MOVE USP, A0
        0x4E61, // MOVE A1, USP
        0x4840, // SWAP D0
        0x4881, // EXT.W D1
        0x48C2, // EXT.L D2
        0xC141, // EXG D0, D1
        0xC149, // EXG A0, A1
        0xC189, // EXG D0, A1
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "LINK");
    assert_eq!(d.operands, "A6, #$FFF0");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "UNLK");
    assert_eq!(d.operands, "A6");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "USP, A0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "A1, USP");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x100A, read);
    assert_eq!(d.mnemonic, "SWAP");
    assert_eq!(d.operands, "D0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x100C, read);
    assert_eq!(d.mnemonic, "EXT.W");
    assert_eq!(d.operands, "D1");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x100E, read);
    assert_eq!(d.mnemonic, "EXT.L");
    assert_eq!(d.operands, "D2");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1010, read);
    assert_eq!(d.mnemonic, "EXG");
    assert_eq!(d.operands, "D0, D1");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1012, read);
    assert_eq!(d.mnemonic, "EXG");
    assert_eq!(d.operands, "A0, A1");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1014, read);
    assert_eq!(d.mnemonic, "EXG");
    assert_eq!(d.operands, "D0, A1");
    assert_eq!(b, 2);
}

#[test]
fn test_disassemble_branches_and_loops() {
    let mem = [
        0x6006, // BRA +6 -> target $1008
        0x6104, // BSR +4 -> target $1008
        0x6702, // BEQ +2 -> target $1008
        0x6600, 0x00FE, // BNE 16-bit disp (+254) -> target $1106
        0x51CB, 0xFFF4, // DBRA D3, $1004
        0x50C0, // ST D0
        0x57C1, // SEQ D1
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "BRA");
    assert_eq!(d.operands, "$001008");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "BSR");
    assert_eq!(d.operands, "$001008");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "BEQ");
    assert_eq!(d.operands, "$001008");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "BNE");
    assert_eq!(d.operands, "$001106");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x100A, read);
    assert_eq!(d.mnemonic, "DBRA");
    assert!(d.operands.contains("D3"));
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x100E, read);
    assert_eq!(d.mnemonic, "ST");
    assert_eq!(d.operands, "D0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1010, read);
    assert_eq!(d.mnemonic, "SEQ");
    assert_eq!(d.operands, "D1");
    assert_eq!(b, 2);
}

#[test]
fn test_disassemble_data_movement_and_lea() {
    let mem = [
        0x3200, // MOVE.W D0, D1
        0x2082, // MOVE.L D2, (A0)
        0x1011, // MOVE.B (A1), D0
        0x702A, // MOVEQ #$2A, D0 (42)
        0x307C, 0x1234, // MOVEA.W #$1234, A0
        0x41F9, 0x0000, 0x2000, // LEA ($00002000).L, A0
        0x4879, 0x0000, 0x3000, // PEA ($00003000).L
        0x48A7, 0xC000, // MOVEM.W D0-D1, -(A7) (bits 15,14 are D0,D1 in pre-decrement)
        0x4CDF, 0x0003, // MOVEM.L (A7)+, D0-D1
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "MOVE.W");
    assert_eq!(d.operands, "D0, D1");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "MOVE.L");
    assert_eq!(d.operands, "D2, (A0)");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "MOVE.B");
    assert_eq!(d.operands, "(A1), D0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "MOVEQ");
    assert_eq!(d.operands, "#$2A, D0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "MOVEA.W");
    assert_eq!(d.operands, "#$1234, A0");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x100C, read);
    assert_eq!(d.mnemonic, "LEA");
    assert_eq!(d.operands, "($00002000).L, A0");
    assert_eq!(b, 6);

    let (d, b) = disassemble(0x1012, read);
    assert_eq!(d.mnemonic, "PEA");
    assert_eq!(d.operands, "($00003000).L");
    assert_eq!(b, 6);

    let (d, b) = disassemble(0x1018, read);
    assert_eq!(d.mnemonic, "MOVEM.W");
    assert_eq!(d.operands, "D0-D1, -(A7)");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x101C, read);
    assert_eq!(d.mnemonic, "MOVEM.L");
    assert_eq!(d.operands, "(A7)+, D0-D1");
    assert_eq!(b, 4);
}

#[test]
fn test_disassemble_arithmetic_and_logic() {
    let mem: [u16; 16] = [
        0xD240, // ADD.W D0, D1
        0xD0C0, // ADDA.W D0, A0
        0x5240, // ADDQ.W #1, D0
        0xD100, // ADDX.B D0, D0
        0x9682, // SUB.L D2, D3
        0x90C0, // SUBA.W D0, A0
        0x5381, // SUBQ.L #1, D1
        0xC840, // AND.W D0, D4
        0x8A41, // OR.W D1, D5
        0xB142, // EOR.W D0, D2
        0x4240, // CLR.W D0
        0x4441, // NEG.W D1
        0x4042, // NEGX.W D2
        0x4643, // NOT.W D3
        0x4A44, // TST.W D4
        0x4180, // CHK.W D0, D0
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, _) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "ADD.W");
    assert_eq!(d.operands, "D0, D1");

    let (d, _) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "ADDA.W");
    assert_eq!(d.operands, "D0, A0");

    let (d, _) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "ADDQ.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "ADDX.B");
    assert_eq!(d.operands, "D0, D0");

    let (d, _) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "SUB.L");
    assert_eq!(d.operands, "D2, D3");

    let (d, _) = disassemble(0x100A, read);
    assert_eq!(d.mnemonic, "SUBA.W");
    assert_eq!(d.operands, "D0, A0");

    let (d, _) = disassemble(0x100C, read);
    assert_eq!(d.mnemonic, "SUBQ.L");
    assert_eq!(d.operands, "#1, D1");

    let (d, _) = disassemble(0x100E, read);
    assert_eq!(d.mnemonic, "AND.W");
    assert_eq!(d.operands, "D0, D4");

    let (d, _) = disassemble(0x1010, read);
    assert_eq!(d.mnemonic, "OR.W");
    assert_eq!(d.operands, "D1, D5");

    let (d, _) = disassemble(0x1012, read);
    assert_eq!(d.mnemonic, "EOR.W");
    assert_eq!(d.operands, "D0, D2");

    let (d, _) = disassemble(0x1014, read);
    assert_eq!(d.mnemonic, "CLR.W");
    assert_eq!(d.operands, "D0");

    let (d, _) = disassemble(0x1016, read);
    assert_eq!(d.mnemonic, "NEG.W");
    assert_eq!(d.operands, "D1");

    let (d, _) = disassemble(0x1018, read);
    assert_eq!(d.mnemonic, "NEGX.W");
    assert_eq!(d.operands, "D2");

    let (d, _) = disassemble(0x101A, read);
    assert_eq!(d.mnemonic, "NOT.W");
    assert_eq!(d.operands, "D3");

    let (d, _) = disassemble(0x101C, read);
    assert_eq!(d.mnemonic, "TST.W");
    assert_eq!(d.operands, "D4");

    let (d, _) = disassemble(0x101E, read);
    assert_eq!(d.mnemonic, "CHK.W");
    assert_eq!(d.operands, "D0, D0");
}

#[test]
fn test_disassemble_shifts_and_rotates() {
    let mem: [u16; 10] = [
        0xE348, // LSL.W #1, D0
        0xE248, // LSR.W #1, D0
        0xE340, // ASL.W #1, D0
        0xE240, // ASR.W #1, D0
        0xE358, // ROL.W #1, D0
        0xE258, // ROR.W #1, D0
        0xE350, // ROXL.W #1, D0
        0xE250, // ROXR.W #1, D0
        0xE3D0, // LSL.W (A0) (memory shift)
        0xE2D0, // LSR.W (A0)
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, _) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "LSL.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "LSR.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "ASL.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "ASR.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "ROL.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x100A, read);
    assert_eq!(d.mnemonic, "ROR.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x100C, read);
    assert_eq!(d.mnemonic, "ROXL.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x100E, read);
    assert_eq!(d.mnemonic, "ROXR.W");
    assert_eq!(d.operands, "#1, D0");

    let (d, _) = disassemble(0x1010, read);
    assert_eq!(d.mnemonic, "LSL.W");
    assert_eq!(d.operands, "(A0)");

    let (d, _) = disassemble(0x1012, read);
    assert_eq!(d.mnemonic, "LSR.W");
    assert_eq!(d.operands, "(A0)");
}

#[test]
fn test_disassemble_multiply_divide_and_bits() {
    let mem = [
        0xC0C0, // MULU.W D0, D0
        0xC1C1, // MULS.W D1, D0
        0x80C0, // DIVU.W D0, D0
        0x81C1, // DIVS.W D1, D0
        0x0100, // BTST D0, D0
        0x0101, // BTST D0, D1
        0x08C0, 0x0004, // BSET #4, D0
        0x0880, 0x0002, // BCLR #2, D0
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, _) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "MULU.W");
    assert_eq!(d.operands, "D0, D0");

    let (d, _) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "MULS.W");
    assert_eq!(d.operands, "D1, D0");

    let (d, _) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "DIVU.W");
    assert_eq!(d.operands, "D0, D0");

    let (d, _) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "DIVS.W");
    assert_eq!(d.operands, "D1, D0");

    let (d, _) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "BTST");
    assert_eq!(d.operands, "D0, D0");

    let (d, _) = disassemble(0x100C, read);
    assert_eq!(d.mnemonic, "BSET");
    assert_eq!(d.operands, "#4, D0");

    let (d, _) = disassemble(0x1010, read);
    assert_eq!(d.mnemonic, "BCLR");
    assert_eq!(d.operands, "#2, D0");
}

#[test]
fn test_disassemble_immediate_and_sr_ccr() {
    let mem = [
        0x007C, 0x2700, // ORI #$2700, SR
        0x027C, 0x0700, // ANDI #$0700, SR
        0x0A3C, 0x001F, // EORI #$1F, CCR
        0x0640, 0x0042, // ADDI.W #$0042, D0
        0x0440, 0x0010, // SUBI.W #$0010, D0
        0x0C40, 0x0020, // CMPI.W #$0020, D0
        0x40C0, // MOVE SR, D0
        0x46C0, // MOVE D0, SR
        0x44C0, // MOVE D0, CCR
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "ORI");
    assert_eq!(d.operands, "#$2700, SR");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "ANDI");
    assert_eq!(d.operands, "#$0700, SR");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "EORI");
    assert_eq!(d.operands, "#$1F, CCR");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x100C, read);
    assert_eq!(d.mnemonic, "ADDI.W");
    assert_eq!(d.operands, "#$0042, D0");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1010, read);
    assert_eq!(d.mnemonic, "SUBI.W");
    assert_eq!(d.operands, "#$0010, D0");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1014, read);
    assert_eq!(d.mnemonic, "CMPI.W");
    assert_eq!(d.operands, "#$0020, D0");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1018, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "SR, D0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x101A, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "D0, SR");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x101C, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "D0, CCR");
    assert_eq!(b, 2);
}

#[test]
fn test_disassemble_fallback_raw_data() {
    // Unmapped/unknown opcode $FFFF
    let (d, b) = disassemble(0x1000, |_| 0xFFFF);
    assert_eq!(d.mnemonic, "DATA.W");
    assert_eq!(d.operands, "$FFFF");
    assert_eq!(b, 2);
}

#[test]
fn test_find_aligned_disassembly_start_fibonacci() {
    use debugger::find_aligned_disassembly_start;

    // Memory layout:
    // $0FFE: 0000 (padding before entry)
    // $1000: 41F9 0000 2000 (LEA ($2000).L, A0 - 6 bytes)
    // $1006: 4240 (CLR.W D0 - 2 bytes)
    // $1008: 323C 0001 (MOVE.W #1, D1 - 4 bytes)
    // $100C: 4E71 (NOP)
    let read = |pc: u32| match pc {
        0x1000 => 0x41F9,
        0x1002 => 0x0000,
        0x1004 => 0x2000,
        0x1006 => 0x4240,
        0x1008 => 0x323C,
        0x100A => 0x0001,
        0x100C => 0x4E71,
        _ => 0x0000,
    };

    // 1. At entry point ($1000): should NOT back up into zeros ($0FFC..$0FFE)
    let start_at_entry = find_aligned_disassembly_start(0x1000, 3, read, &[]);
    assert_eq!(start_at_entry, 0x1000);

    // 2. At second instruction ($1006): with history [0x1000]
    let start_at_1006_with_hist = find_aligned_disassembly_start(0x1006, 3, read, &[0x1000]);
    assert_eq!(start_at_1006_with_hist, 0x1000);

    // 3. At second instruction ($1006): without history (pure heuristic code guessing)
    let start_at_1006_no_hist = find_aligned_disassembly_start(0x1006, 3, read, &[]);
    assert_eq!(start_at_1006_no_hist, 0x1000);

    // 4. At third instruction ($1008): should anchor at $1000
    let start_at_1008 = find_aligned_disassembly_start(0x1008, 3, read, &[0x1000, 0x1006]);
    assert_eq!(start_at_1008, 0x1000);
}
