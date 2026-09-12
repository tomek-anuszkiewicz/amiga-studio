//! Branch, jump, call, return, trap, and control flow disassembler module

use crate::ea::*;

/// Attempts to disassemble branch, jump, trap, return, and loop instructions.
///
/// Returns `Some((mnemonic, operands))` if decoded, consuming extension words via `next_word`.
pub fn try_disassemble_branch(
    pc: u32,
    op: u16,
    mut next_word: impl FnMut() -> u16,
) -> Option<(&'static str, String)> {
    // 1. Inherent & Control Instructions ($4E70..$4E77, $4AFC)
    match op {
        0x4E70 => return Some(("RESET", String::new())),
        0x4E71 => return Some(("NOP", String::new())),
        0x4E72 => {
            let imm = next_word();
            return Some(("STOP", format!("#${:04X}", imm)));
        }
        0x4E73 => return Some(("RTE", String::new())),
        0x4E75 => return Some(("RTS", String::new())),
        0x4E76 => return Some(("TRAPV", String::new())),
        0x4E77 => return Some(("RTR", String::new())),
        0x4AFC => return Some(("ILLEGAL", String::new())),
        _ => {}
    }

    // 2. TRAP #vector ($4E40..$4E4F)
    if (op & 0xFFF0) == 0x4E40 {
        let vec = op & 0x0F;
        return Some(("TRAP", format!("#{}", vec)));
    }

    // 3. LINK / UNLK ($4E50..$4E5F)
    if (op & 0xFFF8) == 0x4E50 {
        let an = (op & 0x07) as u8;
        let d16 = next_word() as i16;
        return Some(("LINK", format!("A{}, #${:04X}", an, d16)));
    }
    if (op & 0xFFF8) == 0x4E58 {
        let an = (op & 0x07) as u8;
        return Some(("UNLK", format!("A{}", an)));
    }

    // 4. JMP / JSR ($4EC0 / $4E80)
    if (op & 0xFFC0) == 0x4EC0 || (op & 0xFFC0) == 0x4E80 {
        let is_jsr = (op & 0xFFC0) == 0x4E80;
        let mnem = if is_jsr { "JSR" } else { "JMP" };
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return Some((mnem, ea_str));
    }

    // 5. BRA / BSR / Bcc ($6000..$6FFF)
    if (op & 0xF000) == 0x6000 {
        let cond = ((op >> 8) & 0x0F) as u8;
        let d8 = (op & 0xFF) as i8;
        let disp = if d8 == 0 {
            next_word() as i16 as i32
        } else {
            d8 as i32
        };
        let target = (pc.wrapping_add(2)).wrapping_add(disp as u32) & 0x00FF_FFFF;
        let mnem = bcc_condition_name(cond);
        return Some((mnem, format!("${:06X}", target)));
    }

    // 6. DBcc / DBRA ($50C8..$5FC8)
    if (op & 0xF0F8) == 0x50C8 {
        let cond = ((op >> 8) & 0x0F) as u8;
        let dn = (op & 0x07) as u8;
        let d16 = next_word() as i16;
        let target_pc = pc.wrapping_add(2).wrapping_add(d16 as u32) & 0x00FF_FFFF;
        let mnem = dbcc_condition_name(cond);
        return Some((mnem, format!("D{}, ${:06X}", dn, target_pc)));
    }

    // 7. Scc <ea> ($50C0..$5FC0, where size bits 7..6 == 3)
    if (op & 0xF0C0) == 0x50C0 {
        let cond = ((op >> 8) & 0x0F) as u8;
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        let mnem = scc_condition_name(cond);
        return Some((mnem, ea_str));
    }

    None
}
