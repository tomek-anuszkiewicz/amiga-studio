use disassembler::disassemble;

#[test]
fn test_disassemble_fallback_raw_data() {
    // Unmapped/unknown opcode $FFFF falls back to DATA.W
    let (d, b) = disassemble(0x1000, |_| 0xFFFF);
    assert_eq!(d.mnemonic, "DATA.W");
    assert_eq!(d.operands, "$FFFF");
    assert_eq!(b, 2);
    assert_eq!(d.word_count, 1);
    assert_eq!(d.words[0], 0xFFFF);
}

#[test]
fn test_disassemble_format_line() {
    let mem = [0x4E71, 0x4E75, 0x4E72, 0x2700]; // NOP, RTS, STOP #$2700
    let read = |pc: u32| mem[((pc - 0x1000) / 2) as usize];

    // Single word instruction line format
    let (d, b) = disassemble(0x1000, read);
    assert_eq!(b, 2);
    assert_eq!(d.format_line(), "00001000: 4E71             NOP");

    let (d, b) = disassemble(0x1002, read);
    assert_eq!(b, 2);
    assert_eq!(d.format_line(), "00001002: 4E75             RTS");

    // Multi-word instruction line format
    let (d, b) = disassemble(0x1004, read);
    assert_eq!(b, 4);
    assert_eq!(
        d.format_line(),
        "00001004: 4E72 2700        STOP     #$2700"
    );
}

#[test]
fn test_disassemble_pc_wrapping_and_offsets() {
    let read = |_| 0x4E71; // NOP everywhere
    let (d, b) = disassemble(0x00FF_FFFE, read);
    assert_eq!(d.pc, 0x00FF_FFFE);
    assert_eq!(d.mnemonic, "NOP");
    assert_eq!(b, 2);
}

#[test]
fn test_disassemble_clr_l() {
    let (d, b) = disassemble(0x2000, |_| 0x4280);
    assert_eq!(d.mnemonic, "CLR.L");
    assert_eq!(d.operands, "D0");
    assert_eq!(b, 2);
}
