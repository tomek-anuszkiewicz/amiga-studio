#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use disassembler::disassemble;

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
fn test_disassemble_immediate_arithmetic() {
    let mem = [
        0x007C, 0x2700, // ORI #$2700, SR
        0x027C, 0x0700, // ANDI #$0700, SR
        0x0A3C, 0x001F, // EORI #$1F, CCR
        0x0640, 0x0042, // ADDI.W #$0042, D0
        0x0440, 0x0010, // SUBI.W #$0010, D0
        0x0C40, 0x0020, // CMPI.W #$0020, D0
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    let (d, b) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "ORI.W");
    assert_eq!(d.operands, "#$2700, SR");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "ANDI.W");
    assert_eq!(d.operands, "#$0700, SR");
    assert_eq!(b, 4);

    let (d, b) = disassemble(0x1008, read);
    assert_eq!(d.mnemonic, "EORI.B");
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
}

#[test]
fn test_disassemble_standard_alu_ops() {
    let mem = [
        0xD041, // ADD.W D1, D0
        0x9041, // SUB.W D1, D0
        0xC041, // AND.W D1, D0
        0x8041, // OR.W  D1, D0
    ];
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];
    let (d, _) = disassemble(0x1000, read);
    assert_eq!(d.mnemonic, "ADD.W");
    let (d, _) = disassemble(0x1002, read);
    assert_eq!(d.mnemonic, "SUB.W");
    let (d, _) = disassemble(0x1004, read);
    assert_eq!(d.mnemonic, "AND.W");
    let (d, _) = disassemble(0x1006, read);
    assert_eq!(d.mnemonic, "OR.W");
}
