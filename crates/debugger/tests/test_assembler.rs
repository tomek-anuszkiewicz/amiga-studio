use debugger::assemble_instruction;

#[test]
fn test_assemble_raw_hex_strings() {
    // 1. Single word
    assert_eq!(assemble_instruction("4E71", 0x1000).unwrap(), vec![0x4E71]);
    assert_eq!(assemble_instruction("$4E71", 0x1000).unwrap(), vec![0x4E71]);
    assert_eq!(
        assemble_instruction(" 4e71 ", 0x1000).unwrap(),
        vec![0x4E71]
    );

    // 2. Multiple words separated by spaces or commas
    assert_eq!(
        assemble_instruction("33FC 0042 0007 0000", 0x1000).unwrap(),
        vec![0x33FC, 0x0042, 0x0007, 0x0000]
    );
    assert_eq!(
        assemble_instruction("$33FC, $0042, $0007, $0000", 0x1000).unwrap(),
        vec![0x33FC, 0x0042, 0x0007, 0x0000]
    );

    // 3. Byte pairs and 32-bit values
    assert_eq!(
        assemble_instruction("4E 71 4E 75", 0x1000).unwrap(),
        vec![0x4E71, 0x4E75]
    );
    assert_eq!(
        assemble_instruction("00070000", 0x1000).unwrap(),
        vec![0x0007, 0x0000]
    );

    // 4. Invalid hex formats
    assert!(assemble_instruction("", 0x1000).is_err());
    assert!(assemble_instruction("   ", 0x1000).is_err());
    assert!(assemble_instruction("4E7", 0x1000).is_err()); // Odd nibble count
    assert!(assemble_instruction("4E 71 4E", 0x1000).is_err()); // Odd byte count
    assert!(assemble_instruction("GHIJ", 0x1000).is_err()); // Non-hex characters
}

#[test]
fn test_assemble_inherent_instructions() {
    assert_eq!(assemble_instruction("NOP", 0x1000).unwrap(), vec![0x4E71]);
    assert_eq!(assemble_instruction("RTS", 0x1000).unwrap(), vec![0x4E75]);
    assert_eq!(assemble_instruction("RTE", 0x1000).unwrap(), vec![0x4E73]);
    assert_eq!(assemble_instruction("RTR", 0x1000).unwrap(), vec![0x4E77]);
    assert_eq!(assemble_instruction("RESET", 0x1000).unwrap(), vec![0x4E70]);
    assert_eq!(assemble_instruction("TRAPV", 0x1000).unwrap(), vec![0x4E76]);
    assert_eq!(
        assemble_instruction("ILLEGAL", 0x1000).unwrap(),
        vec![0x4AFC]
    );
}

#[test]
fn test_assemble_trap_vectors() {
    assert_eq!(
        assemble_instruction("TRAP #0", 0x1000).unwrap(),
        vec![0x4E40]
    );
    assert_eq!(
        assemble_instruction("TRAP #1", 0x1000).unwrap(),
        vec![0x4E41]
    );
    assert_eq!(
        assemble_instruction("TRAP #15", 0x1000).unwrap(),
        vec![0x4E4F]
    );

    // Vector out of range (>15)
    assert!(assemble_instruction("TRAP #16", 0x1000).is_err());
    assert!(assemble_instruction("TRAP #XYZ", 0x1000).is_err());
}

#[test]
fn test_assemble_swap_and_ext() {
    assert_eq!(
        assemble_instruction("SWAP D0", 0x1000).unwrap(),
        vec![0x4840]
    );
    assert_eq!(
        assemble_instruction("SWAP D7", 0x1000).unwrap(),
        vec![0x4847]
    );

    assert_eq!(
        assemble_instruction("EXT.W D0", 0x1000).unwrap(),
        vec![0x4880]
    );
    assert_eq!(
        assemble_instruction("EXT.L D3", 0x1000).unwrap(),
        vec![0x48C3]
    );

    // Invalid register or missing size
    assert!(assemble_instruction("SWAP A0", 0x1000).is_err());
    assert!(assemble_instruction("EXT D0", 0x1000).is_err());
}

#[test]
fn test_assemble_moveq() {
    assert_eq!(
        assemble_instruction("MOVEQ #0, D0", 0x1000).unwrap(),
        vec![0x7000]
    );
    assert_eq!(
        assemble_instruction("MOVEQ #$42, D1", 0x1000).unwrap(),
        vec![0x7242]
    );
    assert_eq!(
        assemble_instruction("MOVEQ #-1, D7", 0x1000).unwrap(),
        vec![0x7EFF]
    );
}

#[test]
fn test_assemble_clr_and_tst() {
    assert_eq!(
        assemble_instruction("CLR.B D0", 0x1000).unwrap(),
        vec![0x4200]
    );
    assert_eq!(
        assemble_instruction("CLR.W D1", 0x1000).unwrap(),
        vec![0x4241]
    );
    assert_eq!(
        assemble_instruction("CLR.L (A0)", 0x1000).unwrap(),
        vec![0x4290]
    );
    assert_eq!(
        assemble_instruction("CLR.W (A1)+", 0x1000).unwrap(),
        vec![0x4259]
    );
    assert_eq!(
        assemble_instruction("CLR.B -(A2)", 0x1000).unwrap(),
        vec![0x4222]
    );

    assert_eq!(
        assemble_instruction("TST.W D2", 0x1000).unwrap(),
        vec![0x4A42]
    );
    assert_eq!(
        assemble_instruction("TST.L (A3)", 0x1000).unwrap(),
        vec![0x4A93]
    );
}

#[test]
fn test_assemble_branches() {
    // BRA forward short ($1000 -> $100A, disp = +8)
    assert_eq!(
        assemble_instruction("BRA $100A", 0x1000).unwrap(),
        vec![0x6008]
    );
    // BRA backward short ($100A -> $1000, disp = -12 => 0xF4)
    assert_eq!(
        assemble_instruction("BRA $1000", 0x100A).unwrap(),
        vec![0x60F4]
    );

    // BSR, BEQ, BNE
    assert_eq!(
        assemble_instruction("BSR $1010", 0x1000).unwrap(),
        vec![0x610E]
    );
    assert_eq!(
        assemble_instruction("BEQ $1008", 0x1000).unwrap(),
        vec![0x6706]
    );
    assert_eq!(
        assemble_instruction("BNE $1008", 0x1000).unwrap(),
        vec![0x6606]
    );

    // 16-bit word branch displacement (> 126 bytes)
    let words = assemble_instruction("BRA $1200", 0x1000).unwrap();
    assert_eq!(words[0], 0x6000);
    assert_eq!(words[1], 0x01FE);

    // Target must be 16-bit aligned (even address)
    assert!(assemble_instruction("BRA $1001", 0x1000).is_err());
}

#[test]
fn test_assemble_jmp_and_jsr() {
    assert_eq!(
        assemble_instruction("JMP (A0)", 0x1000).unwrap(),
        vec![0x4ED0]
    );
    assert_eq!(
        assemble_instruction("JSR (A1)", 0x1000).unwrap(),
        vec![0x4E91]
    );
}

#[test]
fn test_assemble_move_and_alu() {
    assert_eq!(
        assemble_instruction("MOVE.W D0, D1", 0x1000).unwrap(),
        vec![0x3200]
    );
    assert_eq!(
        assemble_instruction("MOVE.L D2, (A0)", 0x1000).unwrap(),
        vec![0x2082]
    );

    assert_eq!(
        assemble_instruction("ADD.W D0, D1", 0x1000).unwrap(),
        vec![0xD240]
    );
    assert_eq!(
        assemble_instruction("SUB.L D2, D3", 0x1000).unwrap(),
        vec![0x9682]
    );
    assert_eq!(
        assemble_instruction("AND.W D0, D4", 0x1000).unwrap(),
        vec![0xC840]
    );
    assert_eq!(
        assemble_instruction("OR.W D1, D5", 0x1000).unwrap(),
        vec![0x8A41]
    );
}

#[test]
fn test_assemble_lea_and_pea() {
    let words = assemble_instruction("LEA ($002000).L, A0", 0x1000).unwrap();
    assert_eq!(words[0], 0x41F9);
    assert_eq!(words[1], 0x0000);
    assert_eq!(words[2], 0x2000);

    let words = assemble_instruction("LEA (A1), A2", 0x1000).unwrap();
    assert_eq!(words, vec![0x45D1]);

    let words = assemble_instruction("PEA ($003000).L", 0x1000).unwrap();
    assert_eq!(words[0], 0x4879);
    assert_eq!(words[1], 0x0000);
    assert_eq!(words[2], 0x3000);
}

#[test]
fn test_assemble_link_and_unlk() {
    assert_eq!(
        assemble_instruction("LINK A6, #-16", 0x1000).unwrap(),
        vec![0x4E56, 0xFFF0]
    );
    assert_eq!(
        assemble_instruction("UNLK A6", 0x1000).unwrap(),
        vec![0x4E5E]
    );
}

#[test]
fn test_assemble_quick_and_adda() {
    assert_eq!(
        assemble_instruction("ADDQ.W #1, D0", 0x1000).unwrap(),
        vec![0x5240]
    );
    assert_eq!(
        assemble_instruction("SUBQ.L #8, (A0)", 0x1000).unwrap(),
        vec![0x5190]
    );

    assert_eq!(
        assemble_instruction("ADDA.W D0, A0", 0x1000).unwrap(),
        vec![0xD0C0]
    );
    assert_eq!(
        assemble_instruction("SUBA.L (A0), A1", 0x1000).unwrap(),
        vec![0x93D0]
    );
}

#[test]
fn test_assemble_cmp_and_eor() {
    assert_eq!(
        assemble_instruction("CMP.W D0, D1", 0x1000).unwrap(),
        vec![0xB240]
    );
    assert_eq!(
        assemble_instruction("CMPA.W (A0), A1", 0x1000).unwrap(),
        vec![0xB2D0]
    );
    assert_eq!(
        assemble_instruction("CMPI.W #$42, D0", 0x1000).unwrap(),
        vec![0x0C40, 0x0042]
    );
    assert_eq!(
        assemble_instruction("EOR.W D0, D1", 0x1000).unwrap(),
        vec![0xB141]
    );
}

#[test]
fn test_assemble_mulu_and_divu() {
    assert_eq!(
        assemble_instruction("MULU.W D0, D1", 0x1000).unwrap(),
        vec![0xC2C0]
    );
    assert_eq!(
        assemble_instruction("MULS.W (A0), D2", 0x1000).unwrap(),
        vec![0xC5D0]
    );
    assert_eq!(
        assemble_instruction("DIVU.W D0, D1", 0x1000).unwrap(),
        vec![0x82C0]
    );
    assert_eq!(
        assemble_instruction("DIVS.W D2, D3", 0x1000).unwrap(),
        vec![0x87C2]
    );
}

#[test]
fn test_assemble_dbra() {
    // DBRA D3, $1004 from PC $1000 -> disp = $1004 - ($1000 + 2) = +2
    let words = assemble_instruction("DBRA D3, $1004", 0x1000).unwrap();
    assert_eq!(words[0], 0x51CB);
    assert_eq!(words[1], 0x0002);
}

#[test]
fn test_roundtrip_assemble_and_disassemble() {
    let cases = [
        ("NOP", "NOP", ""),
        ("RTS", "RTS", ""),
        ("MOVE.W D0, D1", "MOVE.W", "D0, D1"),
        ("MOVEQ #42, D0", "MOVEQ", "#42, D0"),
        ("ADD.W D0, D1", "ADD.W", "D0, D1"),
        ("SUB.L D2, D3", "SUB.L", "D2, D3"),
        ("AND.W D0, D4", "AND.W", "D0, D4"),
        ("OR.W D1, D5", "OR.W", "D1, D5"),
        ("ADDQ.W #1, D0", "ADDQ.W", "#1, D0"),
        ("SWAP D0", "SWAP", "D0"),
        ("EXT.W D1", "EXT.W", "D1"),
        ("EXT.L D2", "EXT.L", "D2"),
        ("UNLK A6", "UNLK", "A6"),
        ("MULU.W D0, D1", "MULU.W", "D0, D1"),
        ("DIVU.W D0, D1", "DIVU.W", "D0, D1"),
    ];

    for (asm_str, exp_mnem, exp_ops) in cases {
        let words = assemble_instruction(asm_str, 0x1000)
            .unwrap_or_else(|e| panic!("Failed to assemble '{}': {}", asm_str, e));
        let (dis, _) = debugger::disassemble(0x1000, |addr| {
            let idx = ((addr - 0x1000) / 2) as usize;
            if idx < words.len() {
                words[idx]
            } else {
                0
            }
        });
        assert_eq!(
            dis.mnemonic, exp_mnem,
            "Mnemonic mismatch for '{}'",
            asm_str
        );
        if !exp_ops.is_empty() {
            assert_eq!(dis.operands, exp_ops, "Operands mismatch for '{}'", asm_str);
        }
    }
}
