use disassembler::disassemble;

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
fn test_disassemble_jumps() {
    let mem = [
        0x4ED0, // JMP (A0)
        0x4EB9, 0x0002, 0x0000, // JSR ($00020000).L
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "JMP");
    assert_eq!(d.operands, "(A0)");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "JSR");
    assert_eq!(d.operands, "($00020000).L");
    assert_eq!(b, 6);
}
