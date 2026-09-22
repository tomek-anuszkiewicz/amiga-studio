#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use disassembler::disassemble;

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
        0x48A7, 0xC000, // MOVEM.W D0-D1, -(A7) (bits 15,14 in predec)
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
    assert_eq!(d.operands, "#42, D0");
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
    assert_eq!(d.operands, "A6, #-16");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "UNLK");
    assert_eq!(d.operands, "A6");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "A0, USP");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "USP, A1");
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
fn test_disassemble_sr_and_ccr() {
    let mem = [
        0x40C0, // MOVE.W SR, D0
        0x46C0, // MOVE.W D0, SR
        0x44C0, // MOVE.W D0, CCR
        0x4E60, // MOVE USP, A0
        0x4E68, // MOVE A0, USP
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "MOVE.W");
    assert_eq!(d.operands, "SR, D0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "MOVE.W");
    assert_eq!(d.operands, "D0, SR");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "MOVE.W");
    assert_eq!(d.operands, "D0, CCR");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "USP, A0");
    assert_eq!(b, 2);

    let (d, b) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "MOVE");
    assert_eq!(d.operands, "A0, USP");
    assert_eq!(b, 2);
}
