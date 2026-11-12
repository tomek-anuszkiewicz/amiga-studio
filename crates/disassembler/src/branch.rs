//! Branch, jump, call, return, trap, and control flow disassembler module

use crate::encoding::opcode;
const RESET_OPCODE: u16 = 0x4E70;
const NOP_OPCODE: u16 = 0x4E71;
const STOP_OPCODE: u16 = 0x4E72;
const RTE_OPCODE: u16 = 0x4E73;
const RTS_OPCODE: u16 = 0x4E75;
const TRAPV_OPCODE: u16 = 0x4E76;
const RTR_OPCODE: u16 = 0x4E77;
const ILLEGAL_OPCODE: u16 = 0x4AFC;
const MASK_TRAP: u16 = 0xFFF0;
const TRAP_PATTERN: u16 = 0x4E40;
const LINK_PATTERN: u16 = 0x4E50;
const UNLK_PATTERN: u16 = 0x4E58;
const JMP_PATTERN: u16 = 0x4EC0;
const JSR_PATTERN: u16 = 0x4E80;
const BRANCH_LINE: u16 = 0x6000;
const MASK_DBCC: u16 = 0xF0F8;
const DBCC_PATTERN: u16 = 0x50C8;
const MASK_SCC: u16 = 0xF0C0;
const SCC_PATTERN: u16 = 0x50C0;
use crate::ea::{
    bcc_condition_name, dbcc_condition_name, format_ea, is_control_ea, is_data_alterable_ea,
    scc_condition_name,
};
use crate::encoding::{condition, field};

/// Attempts to disassemble branch, jump, trap, return, and loop instructions.
///
/// Returns `Some((mnemonic, operands))` if decoded, consuming extension words via `next_word`.
pub(crate) fn try_disassemble_branch(
    pc: u32,
    op: u16,
    mut next_word: impl FnMut() -> u16,
) -> Option<(&'static str, String)> {
    // 1. Inherent & Control Instructions ($4E70..$4E77, $4AFC)
    match op {
        RESET_OPCODE => return Some(("RESET", String::new())),
        NOP_OPCODE => return Some(("NOP", String::new())),
        STOP_OPCODE => {
            let imm = next_word();
            return Some(("STOP", format!("#${:04X}", imm)));
        }
        RTE_OPCODE => return Some(("RTE", String::new())),
        RTS_OPCODE => return Some(("RTS", String::new())),
        TRAPV_OPCODE => return Some(("TRAPV", String::new())),
        RTR_OPCODE => return Some(("RTR", String::new())),
        ILLEGAL_OPCODE => return Some(("ILLEGAL", String::new())),
        _ => {}
    }

    // 2. TRAP #vector ($4E40..$4E4F)
    if (op & MASK_TRAP) == TRAP_PATTERN {
        let vec = op & field::MASK_TRAP_NUMBER;
        return Some(("TRAP", format!("#{}", vec)));
    }

    // 3. LINK / UNLK ($4E50..$4E5F)
    if (op & opcode::MASK_REGISTER_OPCODE) == LINK_PATTERN {
        let an = (op & field::MASK_REGISTER) as u8;
        let d16 = next_word() as i16;
        return Some(("LINK", format!("A{}, #{}", an, d16)));
    }
    if (op & opcode::MASK_REGISTER_OPCODE) == UNLK_PATTERN {
        let an = (op & field::MASK_REGISTER) as u8;
        return Some(("UNLK", format!("A{}", an)));
    }

    // 4. JMP / JSR ($4EC0 / $4E80)
    if (op & opcode::MASK_EA_OPCODE) == JMP_PATTERN || (op & opcode::MASK_EA_OPCODE) == JSR_PATTERN
    {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_control_ea(mode, reg) {
            return None;
        }
        let is_jsr = (op & opcode::MASK_EA_OPCODE) == JSR_PATTERN;
        let mnem = if is_jsr { "JSR" } else { "JMP" };
        let ea_str = format_ea(mode, reg, &mut next_word);
        return Some((mnem, ea_str));
    }

    // 5. BRA / BSR / Bcc ($6000..$6FFF)
    if (op & opcode::MASK_OPCODE_LINE) == BRANCH_LINE {
        let cond = ((op >> condition::BIT_SHIFT) & condition::MASK_CONDITION as u16) as u8;
        let d8 = (op & 0xFF) as i8;
        let disp = if d8 == 0 {
            next_word() as i16 as i32
        } else {
            d8 as i32
        };
        let target =
            (pc.wrapping_add(2)).wrapping_add(disp as u32) & crate::encoding::MASK_PHYSICAL_ADDRESS;
        let mnem = bcc_condition_name(cond);
        return Some((mnem, format!("${:06X}", target)));
    }

    // 6. DBcc / DBRA ($50C8..$5FC8)
    if (op & MASK_DBCC) == DBCC_PATTERN {
        let cond = ((op >> condition::BIT_SHIFT) & condition::MASK_CONDITION as u16) as u8;
        let dn = (op & field::MASK_REGISTER) as u8;
        let d16 = next_word() as i16;
        let target_pc =
            pc.wrapping_add(2).wrapping_add(d16 as u32) & crate::encoding::MASK_PHYSICAL_ADDRESS;
        let mnem = dbcc_condition_name(cond);
        return Some((mnem, format!("D{}, ${:06X}", dn, target_pc)));
    }

    // 7. Scc <ea> ($50C0..$5FC0, where size bits 7..6 == 3)
    if (op & MASK_SCC) == SCC_PATTERN {
        let mode = ((op >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE) as u8;
        let reg = (op & field::MASK_REGISTER) as u8;
        if !is_data_alterable_ea(mode, reg) {
            return None;
        }
        let cond = ((op >> condition::BIT_SHIFT) & condition::MASK_CONDITION as u16) as u8;
        let ea_str = format_ea(mode, reg, &mut next_word);
        let mnem = scc_condition_name(cond);
        return Some((mnem, ea_str));
    }

    None
}
