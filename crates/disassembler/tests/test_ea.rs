use disassembler::{
    bcc_condition_name, dbcc_condition_name, format_ea, format_immediate, format_movem_reg_list,
    scc_condition_name,
};

#[test]
fn test_format_ea_register_direct_and_indirect() {
    let dummy_ext = || -> u16 { panic!("read_ext called unexpectedly") };

    // Mode 0: Data Register Direct
    assert_eq!(format_ea(0, 0, dummy_ext), "D0");
    assert_eq!(format_ea(0, 7, dummy_ext), "D7");

    // Mode 1: Address Register Direct
    assert_eq!(format_ea(1, 0, dummy_ext), "A0");
    assert_eq!(format_ea(1, 7, dummy_ext), "A7");

    // Mode 2: Address Register Indirect
    assert_eq!(format_ea(2, 2, dummy_ext), "(A2)");

    // Mode 3: Post-increment
    assert_eq!(format_ea(3, 3, dummy_ext), "(A3)+");

    // Mode 4: Pre-decrement
    assert_eq!(format_ea(4, 4, dummy_ext), "-(A4)");
}

#[test]
fn test_format_ea_displacement_and_index() {
    // Mode 5: Address Register Indirect with Displacement (d16, An)
    let d16 = 0x0100u16;
    assert_eq!(format_ea(5, 1, || d16), "($0100, A1)");

    // Mode 5 negative displacement
    let neg_d16 = 0xFFF0u16; // -16
    assert_eq!(format_ea(5, 2, || neg_d16), "($FFF0, A2)");

    // Mode 6: Address Register Indirect with Index (d8, An, Xn.SIZE)
    // ext: bit 15 = 0 (Data reg D), reg 3 (bits 14..12 = 011), size W (bit 11 = 0), d8 = 0x20
    let ext_dw = 0x3020u16;
    assert_eq!(format_ea(6, 0, || ext_dw), "($20, A0, D3.W)");

    // ext: bit 15 = 1 (Address reg A), reg 5, size L (bit 11 = 1), d8 = 0xFE (-2)
    let ext_al = 0xD8FEu16;
    assert_eq!(format_ea(6, 4, || ext_al), "($FE, A4, A5.L)");
}

#[test]
fn test_format_ea_mode7_special() {
    // 7.0: Absolute Short ($xxxx).W
    assert_eq!(format_ea(7, 0, || 0x1234), "($1234).W");

    // 7.1: Absolute Long ($xxxxxxxx).L
    let mut words = [0x0012u16, 0x3456u16].into_iter();
    assert_eq!(format_ea(7, 1, || words.next().unwrap()), "($00123456).L");

    // 7.2: Program Counter with Displacement (d16, PC)
    assert_eq!(format_ea(7, 2, || 0x0040), "($0040, PC)");

    // 7.3: Program Counter with Index (d8, PC, Xn.SIZE)
    let ext_pc_idx = 0x1808u16; // D1.L, d8 = 0x08
    assert_eq!(format_ea(7, 3, || ext_pc_idx), "($08, PC, D1.L)");

    // 7.4: Immediate
    assert_eq!(format_ea(7, 4, || 0xABCD), "#$ABCD");

    // 7.5..7.7: Unknown
    assert_eq!(format_ea(7, 5, || 0), "UNKNOWN_EA(7, 5)");
    assert_eq!(format_ea(8, 0, || 0), "UNKNOWN_EA(8, 0)");
}

#[test]
fn test_format_immediate() {
    // Byte immediate
    assert_eq!(format_immediate(1, || 0x00FF), "#$FF");

    // Word immediate
    assert_eq!(format_immediate(2, || 0x1234), "#$1234");

    // Long immediate
    let mut long_words = [0xDEADu16, 0xBEEFu16].into_iter();
    assert_eq!(
        format_immediate(4, || long_words.next().unwrap()),
        "#$DEADBEEF"
    );

    // Invalid size fallback
    assert_eq!(format_immediate(0, || 0), "#$00");
}

#[test]
fn test_format_movem_reg_list() {
    // Zero mask
    assert_eq!(format_movem_reg_list(0x0000, false), "<none>");

    // Standard mode (control / post-increment modes)
    // Bit 0 = D0, bit 1 = D1, bit 2 = D2 -> "D0-D2"
    assert_eq!(format_movem_reg_list(0x0007, false), "D0-D2");

    // Mixed non-consecutive: D0, D2, A0, A1, A3
    // D0 = bit 0, D2 = bit 2, A0 = bit 8, A1 = bit 9, A3 = bit 11
    let mask = (1 << 0) | (1 << 2) | (1 << 8) | (1 << 9) | (1 << 11);
    assert_eq!(format_movem_reg_list(mask, false), "D0/D2/A0-A1/A3");

    // All data registers D0-D7
    assert_eq!(format_movem_reg_list(0x00FF, false), "D0-D7");

    // All address registers A0-A7
    assert_eq!(format_movem_reg_list(0xFF00, false), "A0-A7");

    // All registers D0-D7/A0-A7
    assert_eq!(format_movem_reg_list(0xFFFF, false), "D0-D7/A0-A7");

    // Pre-decrement mode (is_predec = true): reverse bit order
    // In predec mode: bit 15 = D0, bit 14 = D1, ..., bit 8 = D7, bit 7 = A0, bit 0 = A7
    let predec_mask = (1 << 15) | (1 << 14) | (1 << 7); // D0, D1, A0
    assert_eq!(format_movem_reg_list(predec_mask, true), "D0-D1/A0");
}

#[test]
fn test_condition_names() {
    assert_eq!(bcc_condition_name(0), "BRA");
    assert_eq!(bcc_condition_name(1), "BSR");
    assert_eq!(bcc_condition_name(6), "BNE");
    assert_eq!(bcc_condition_name(7), "BEQ");

    assert_eq!(dbcc_condition_name(0), "DBT");
    assert_eq!(dbcc_condition_name(1), "DBF");
    assert_eq!(dbcc_condition_name(6), "DBNE");
    assert_eq!(dbcc_condition_name(7), "DBEQ");

    assert_eq!(scc_condition_name(0), "ST");
    assert_eq!(scc_condition_name(1), "SF");
    assert_eq!(scc_condition_name(6), "SNE");
    assert_eq!(scc_condition_name(7), "SEQ");
}
