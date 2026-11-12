//! Data movement, stack operations, unaries, and single-operand disassembler module

use crate::encoding::opcode;
const PEA_PATTERN: u16 = 0x4840;
const EXT_WORD_PATTERN: u16 = 0x4880;
const QUICK_LINE: u16 = 0x5000;
const BIT_QUICK_SUBTRACT: u16 = 0x0100;
const MOVEQ_PATTERN: u16 = 0x7000;
const MASK_MOVEM: u16 = 0xFB80;
const MOVEM_PATTERN: u16 = 0x4880;
const BIT_MOVEM_LOAD: u16 = 0x0400;
const BIT_MOVEM_LONG: u16 = 0x0040;
const NEGX_GROUP: u16 = 0x0040;
const LEA_PATTERN: u16 = 0x41C0;
const CHK_PATTERN: u16 = 0x4180;
const NBCD_PATTERN: u16 = 0x4800;
const SWAP_PATTERN: u16 = 0x4840;
const EXT_LONG_PATTERN: u16 = 0x48C0;
const MOVE_TO_USP_PATTERN: u16 = 0x4E60;
const MOVE_FROM_USP_PATTERN: u16 = 0x4E68;
const MOVE_FROM_SR_PATTERN: u16 = 0x40C0;
const MOVE_TO_CCR_PATTERN: u16 = 0x44C0;
const MOVE_TO_SR_PATTERN: u16 = 0x46C0;
const CLR_GROUP: u16 = 0x0042;
const NEG_GROUP: u16 = 0x0044;
const NOT_GROUP: u16 = 0x0046;
const TST_GROUP: u16 = 0x004A;
use crate::ea::{
    format_ea, format_movem_reg_list, is_control_ea, is_data_alterable_ea, is_data_ea,
};
use crate::encoding::{extended_mode, field, mode, move_size, size};

/// Attempts to disassemble data movement, register transfer, and unary arithmetic instructions.
///
/// Returns `Some((mnemonic, operands))` if decoded, consuming extension words via `next_word`.
pub(crate) fn try_disassemble_data(
    op: u16,
    mut next_word: impl FnMut() -> u16,
) -> Option<(&'static str, String)> {
    // 1. ADDQ / SUBQ ($5000..$5FFF)
    if (op & opcode::MASK_OPCODE_LINE) == QUICK_LINE {
        let size_bits = (op >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE;
        if size_bits < size::SPECIAL {
            let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
            let reg = (op & field::MASK_REGISTER) as u8;
            if !(is_data_alterable_ea(mode, reg)
                || (mode == mode::ADDRESS_REGISTER && size_bits != size::BYTE))
            {
                return None;
            }
            let is_subq = (op & BIT_QUICK_SUBTRACT) != 0;
            let raw = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
            let data = if raw == 0 {
                field::QUICK_ZERO_VALUE
            } else {
                raw
            };
            let sz = match size_bits {
                size::BYTE => ".B",
                size::WORD => ".W",
                size::LONG => ".L",
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
            let ea_str = format_ea(mode, reg, &mut next_word);
            return Some((mnem, format!("#{}, {}", data, ea_str)));
        }
    }

    // 2. MOVE / MOVEA ($1000..$3FFF)
    let top2 = (op >> field::BIT_MOVE_CLASS_SHIFT) & field::MASK_MOVE_CLASS;
    if top2 == field::MOVE_CLASS {
        let move_size_bits = (op >> field::BIT_MOVE_SIZE_SHIFT) & field::MASK_SIZE;
        if move_size_bits != move_size::INVALID {
            let mnem = match move_size_bits {
                move_size::BYTE => "MOVE.B",
                move_size::WORD => "MOVE.W",
                move_size::LONG => "MOVE.L",
                _ => "MOVE",
            };
            let dst_reg = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
            let dst_mode = ((op >> field::BIT_MOVE_DEST_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
            let src_mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
            let src_reg = (op & field::MASK_REGISTER) as u8;
            if !(is_data_ea(src_mode, src_reg)
                || (move_size_bits != move_size::BYTE && src_mode == mode::ADDRESS_REGISTER))
                || (move_size_bits == move_size::BYTE && dst_mode == mode::ADDRESS_REGISTER)
                || (dst_mode != mode::ADDRESS_REGISTER && !is_data_alterable_ea(dst_mode, dst_reg))
            {
                return None;
            }
            let mnem = if dst_mode == mode::ADDRESS_REGISTER {
                if move_size_bits == move_size::WORD {
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
    if (op & opcode::MASK_REGISTER_LINE) == MOVEQ_PATTERN {
        let reg = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
        let data = (op & 0xFF) as i8;
        return Some(("MOVEQ", format!("#{}, D{}", data, reg)));
    }

    // 4. MOVEM ($4880..$48BF, $48C0..$48FF, $4C80..$4CBF, $4CC0..$4CFF)
    let movem_ea_mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
    if (op & MASK_MOVEM) == MOVEM_PATTERN && movem_ea_mode >= mode::INDIRECT {
        let is_load = (op & BIT_MOVEM_LOAD) != 0;
        let ea_reg = (op & field::MASK_REGISTER) as u8;
        let valid_ea = if is_load {
            matches!(
                movem_ea_mode,
                mode::INDIRECT | mode::POSTINCREMENT | mode::DISPLACEMENT | mode::INDEXED
            ) || (movem_ea_mode == mode::EXTENDED && ea_reg <= extended_mode::PC_INDEXED)
        } else {
            matches!(
                movem_ea_mode,
                mode::INDIRECT | mode::PREDECREMENT | mode::DISPLACEMENT | mode::INDEXED
            ) || (movem_ea_mode == mode::EXTENDED && ea_reg <= extended_mode::ABSOLUTE_LONG)
        };
        if !valid_ea {
            return None;
        }
        let sz = if (op & BIT_MOVEM_LONG) != 0 {
            "MOVEM.L"
        } else {
            "MOVEM.W"
        };
        let mask = next_word();
        let ea_str = format_ea(movem_ea_mode, ea_reg, &mut next_word);
        let reg_list = format_movem_reg_list(mask, movem_ea_mode == mode::PREDECREMENT);
        let ops = if is_load {
            format!("{}, {}", ea_str, reg_list)
        } else {
            format!("{}, {}", reg_list, ea_str)
        };
        return Some((sz, ops));
    }

    // 5. LEA & CHK ($41C0, $4180)
    if (op & opcode::MASK_REGISTER_EA_OPCODE) == LEA_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_control_ea(mode, reg) {
            return None;
        }
        let an = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
        let ea_str = format_ea(mode, reg, &mut next_word);
        return Some(("LEA", format!("{}, A{}", ea_str, an)));
    }
    if (op & opcode::MASK_REGISTER_EA_OPCODE) == CHK_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_data_ea(mode, reg) {
            return None;
        }
        let dn = ((op >> field::BIT_REGISTER_SHIFT) & field::MASK_REGISTER) as u8;
        let ea_str = format_ea(mode, reg, &mut next_word);
        return Some(("CHK.W", format!("{}, D{}", ea_str, dn)));
    }

    // 6. PEA, SWAP, EXT ($4840, $4880, $48C0)
    if (op & opcode::MASK_EA_OPCODE) == NBCD_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_data_alterable_ea(mode, reg) {
            return None;
        }
        return Some(("NBCD", format_ea(mode, reg, &mut next_word)));
    }
    if (op & opcode::MASK_REGISTER_OPCODE) == SWAP_PATTERN {
        let dn = (op & field::MASK_REGISTER) as u8;
        return Some(("SWAP", format!("D{}", dn)));
    }
    if (op & opcode::MASK_EA_OPCODE) == PEA_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_control_ea(mode, reg) {
            return None;
        }
        let ea_str = format_ea(mode, reg, &mut next_word);
        return Some(("PEA", ea_str));
    }
    if (op & opcode::MASK_REGISTER_OPCODE) == EXT_WORD_PATTERN {
        let dn = (op & field::MASK_REGISTER) as u8;
        return Some(("EXT.W", format!("D{}", dn)));
    }
    if (op & opcode::MASK_REGISTER_OPCODE) == EXT_LONG_PATTERN {
        let dn = (op & field::MASK_REGISTER) as u8;
        return Some(("EXT.L", format!("D{}", dn)));
    }

    // 7. MOVE to/from USP ($4E60, $4E68)
    if (op & opcode::MASK_REGISTER_OPCODE) == MOVE_TO_USP_PATTERN {
        let an = (op & field::MASK_REGISTER) as u8;
        return Some(("MOVE", format!("A{}, USP", an)));
    }
    if (op & opcode::MASK_REGISTER_OPCODE) == MOVE_FROM_USP_PATTERN {
        let an = (op & field::MASK_REGISTER) as u8;
        return Some(("MOVE", format!("USP, A{}", an)));
    }

    // 8. SR & CCR transfers ($40C0, $44C0, $46C0)
    if (op & opcode::MASK_EA_OPCODE) == MOVE_FROM_SR_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_data_alterable_ea(mode, reg) {
            return None;
        }
        let ea_str = format_ea(
            ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
            (op & field::MASK_REGISTER) as u8,
            &mut next_word,
        );
        return Some(("MOVE.W", format!("SR, {}", ea_str)));
    }
    if (op & opcode::MASK_EA_OPCODE) == MOVE_TO_CCR_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_data_ea(mode, reg) {
            return None;
        }
        let ea_str = format_ea(
            ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
            (op & field::MASK_REGISTER) as u8,
            &mut next_word,
        );
        return Some(("MOVE.W", format!("{}, CCR", ea_str)));
    }
    if (op & opcode::MASK_EA_OPCODE) == MOVE_TO_SR_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_data_ea(mode, reg) {
            return None;
        }
        let ea_str = format_ea(
            ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
            (op & field::MASK_REGISTER) as u8,
            &mut next_word,
        );
        return Some(("MOVE.W", format!("{}, SR", ea_str)));
    }

    // 9. CLR, NEG, NEGX, NOT, TST ($4000..$4AFF)
    let group_4 = (op >> field::BIT_OPCODE_HIGH_BYTE_SHIFT) & 0xFF;
    if matches!(
        group_4,
        NEGX_GROUP | CLR_GROUP | NEG_GROUP | NOT_GROUP | TST_GROUP
    ) {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_data_alterable_ea(mode, reg) {
            return None;
        }
        let size_bits = (op >> field::BIT_SIZE_SHIFT) & field::MASK_SIZE;
        if size_bits < size::SPECIAL {
            let mnem = match (group_4, size_bits) {
                (NEGX_GROUP, size::BYTE) => "NEGX.B",
                (NEGX_GROUP, size::WORD) => "NEGX.W",
                (NEGX_GROUP, size::LONG) => "NEGX.L",
                (CLR_GROUP, size::BYTE) => "CLR.B",
                (CLR_GROUP, size::WORD) => "CLR.W",
                (CLR_GROUP, size::LONG) => "CLR.L",
                (NEG_GROUP, size::BYTE) => "NEG.B",
                (NEG_GROUP, size::WORD) => "NEG.W",
                (NEG_GROUP, size::LONG) => "NEG.L",
                (NOT_GROUP, size::BYTE) => "NOT.B",
                (NOT_GROUP, size::WORD) => "NOT.W",
                (NOT_GROUP, size::LONG) => "NOT.L",
                (TST_GROUP, size::BYTE) => "TST.B",
                (TST_GROUP, size::WORD) => "TST.W",
                (TST_GROUP, size::LONG) => "TST.L",
                _ => "OP",
            };
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some((mnem, ea_str));
        } else if group_4 == TST_GROUP && size_bits == size::SPECIAL {
            let ea_str = format_ea(
                ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8,
                (op & field::MASK_REGISTER) as u8,
                &mut next_word,
            );
            return Some(("TAS", ea_str));
        }
    }

    None
}
