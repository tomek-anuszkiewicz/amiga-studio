//! Data movement, stack operations, unaries, and single-operand disassembler module

use crate::ea::*;

/// Attempts to disassemble data movement, register transfer, and unary arithmetic instructions.
///
/// Returns `Some((mnemonic, operands))` if decoded, consuming extension words via `next_word`.
pub fn try_disassemble_data(
    op: u16,
    mut next_word: impl FnMut() -> u16,
) -> Option<(&'static str, String)> {
    // 1. ADDQ / SUBQ ($5000..$5FFF)
    if (op & 0xF000) == 0x5000 {
        let size_bits = (op >> 6) & 0x03;
        if size_bits < 3 {
            let is_subq = (op & 0x0100) != 0;
            let raw = ((op >> 9) & 7) as u8;
            let data = if raw == 0 { 8 } else { raw };
            let sz = match size_bits {
                0 => ".B",
                1 => ".W",
                2 => ".L",
                _ => "",
            };
            let mnem = if is_subq {
                match sz {
                    ".B" => "SUBQ.B",
                    ".W" => "SUBQ.W",
                    ".L" => "SUBQ.L",
                    _ => "SUBQ",
                }
            } else {
                match sz {
                    ".B" => "ADDQ.B",
                    ".W" => "ADDQ.W",
                    ".L" => "ADDQ.L",
                    _ => "ADDQ",
                }
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, format!("#{}, {}", data, ea_str)));
        }
    }

    // 2. MOVE / MOVEA ($1000..$3FFF)
    let top2 = (op >> 14) & 0x03;
    if top2 == 0 {
        let size_bits = (op >> 12) & 0x03;
        if size_bits != 0 {
            let mnem = match size_bits {
                1 => "MOVE.B",
                3 => "MOVE.W",
                2 => "MOVE.L",
                _ => "MOVE",
            };
            let dst_reg = ((op >> 9) & 7) as u8;
            let dst_mode = ((op >> 6) & 7) as u8;
            let src_mode = ((op >> 3) & 7) as u8;
            let src_reg = (op & 7) as u8;
            let mnem = if dst_mode == 1 {
                if size_bits == 3 {
                    "MOVEA.W"
                } else {
                    "MOVEA.L"
                }
            } else {
                mnem
            };
            let src_str = format_ea(src_mode, src_reg, &mut next_word);
            let dst_str = format_ea(dst_mode, dst_reg, &mut next_word);
            return Some((mnem, format!("{}, {}", src_str, dst_str)));
        }
    }

    // 3. MOVEQ ($7000..$7FFF)
    if (op & 0xF100) == 0x7000 {
        let reg = ((op >> 9) & 7) as u8;
        let data = (op & 0xFF) as i8;
        return Some(("MOVEQ", format!("#{}, D{}", data, reg)));
    }

    // 4. MOVEM ($4880..$48BF, $48C0..$48FF, $4C80..$4CBF, $4CC0..$4CFF)
    let movem_ea_mode = ((op >> 3) & 7) as u8;
    if (op & 0xFB80) == 0x4880 && movem_ea_mode >= 2 {
        let is_load = (op & 0x0400) != 0;
        let sz = if (op & 0x0040) != 0 {
            "MOVEM.L"
        } else {
            "MOVEM.W"
        };
        let mask = next_word();
        let ea_reg = (op & 7) as u8;
        let ea_str = format_ea(movem_ea_mode, ea_reg, &mut next_word);
        let reg_list = format_movem_reg_list(mask, movem_ea_mode == 4);
        let ops = if is_load {
            format!("{}, {}", ea_str, reg_list)
        } else {
            format!("{}, {}", reg_list, ea_str)
        };
        return Some((sz, ops));
    }

    // 5. LEA & CHK ($41C0, $4180)
    if (op & 0xF1C0) == 0x41C0 {
        let an = ((op >> 9) & 7) as u8;
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return Some(("LEA", format!("{}, A{}", ea_str, an)));
    }
    if (op & 0xF1C0) == 0x4180 {
        let dn = ((op >> 9) & 7) as u8;
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return Some(("CHK.W", format!("{}, D{}", ea_str, dn)));
    }

    // 6. PEA, SWAP, EXT ($4840, $4880, $48C0)
    if (op & 0xFFF8) == 0x4840 {
        let dn = (op & 7) as u8;
        return Some(("SWAP", format!("D{}", dn)));
    }
    if (op & 0xFFC0) == 0x4840 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return Some(("PEA", ea_str));
    }
    if (op & 0xFFF8) == 0x4880 {
        let dn = (op & 7) as u8;
        return Some(("EXT.W", format!("D{}", dn)));
    }
    if (op & 0xFFF8) == 0x48C0 {
        let dn = (op & 7) as u8;
        return Some(("EXT.L", format!("D{}", dn)));
    }

    // 7. MOVE to/from USP ($4E60, $4E68)
    if (op & 0xFFF8) == 0x4E60 {
        let an = (op & 0x07) as u8;
        return Some(("MOVE", format!("USP, A{}", an)));
    }
    if (op & 0xFFF8) == 0x4E68 {
        let an = (op & 0x07) as u8;
        return Some(("MOVE", format!("A{}, USP", an)));
    }

    // 8. SR & CCR transfers ($40C0, $44C0, $46C0)
    if (op & 0xFFC0) == 0x40C0 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return Some(("MOVE.W", format!("SR, {}", ea_str)));
    }
    if (op & 0xFFC0) == 0x44C0 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return Some(("MOVE.W", format!("{}, CCR", ea_str)));
    }
    if (op & 0xFFC0) == 0x46C0 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return Some(("MOVE.W", format!("{}, SR", ea_str)));
    }

    // 9. CLR, NEG, NEGX, NOT, TST ($4000..$4AFF)
    let group_4 = (op >> 8) & 0xFF;
    if group_4 == 0x40 || group_4 == 0x42 || group_4 == 0x44 || group_4 == 0x46 || group_4 == 0x4A {
        let size_bits = (op >> 6) & 0x03;
        if size_bits < 3 {
            let mnem = match (group_4, size_bits) {
                (0x40, 0) => "NEGX.B",
                (0x40, 1) => "NEGX.W",
                (0x40, 2) => "NEGX.L",
                (0x42, 0) => "CLR.B",
                (0x42, 1) => "CLR.W",
                (0x42, 2) => "CLR.L",
                (0x44, 0) => "NEG.B",
                (0x44, 1) => "NEG.W",
                (0x44, 2) => "NEG.L",
                (0x46, 0) => "NOT.B",
                (0x46, 1) => "NOT.W",
                (0x46, 2) => "NOT.L",
                (0x4A, 0) => "TST.B",
                (0x4A, 1) => "TST.W",
                (0x4A, 2) => "TST.L",
                _ => "OP",
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, ea_str));
        } else if group_4 == 0x4A && size_bits == 3 {
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some(("TAS", ea_str));
        }
    }

    None
}
