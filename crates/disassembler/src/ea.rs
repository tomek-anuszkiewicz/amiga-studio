//! Motorola 68000 Effective Address and Register List Formatting Helpers
//!
//! Provides zero-panic formatting for addressing modes, immediate values,
//! condition codes, and MOVEM register masks.

use crate::encoding::{condition, extended_mode, field, mode};

const REGISTER_BANK_COUNT: usize = 8;
pub(crate) fn is_data_alterable_ea(mode: u8, reg: u8) -> bool {
    matches!(mode, mode::DATA_REGISTER | mode::INDIRECT..=mode::INDEXED)
        || (mode == mode::EXTENDED && reg <= extended_mode::ABSOLUTE_LONG)
}

pub(crate) fn is_data_ea(mode: u8, reg: u8) -> bool {
    matches!(mode, mode::DATA_REGISTER | mode::INDIRECT..=mode::INDEXED)
        || (mode == mode::EXTENDED && reg <= extended_mode::IMMEDIATE)
}

pub(crate) fn is_memory_alterable_ea(mode: u8, reg: u8) -> bool {
    matches!(mode, mode::INDIRECT..=mode::INDEXED)
        || (mode == mode::EXTENDED && reg <= extended_mode::ABSOLUTE_LONG)
}

pub(crate) fn is_control_ea(mode: u8, reg: u8) -> bool {
    matches!(mode, mode::INDIRECT | mode::DISPLACEMENT | mode::INDEXED)
        || (mode == mode::EXTENDED && reg <= extended_mode::PC_INDEXED)
}

/// Formats an effective address (EA) based on mode (0..7) and register (0..7)
pub fn format_ea(mode: u8, reg: u8, mut read_ext: impl FnMut() -> u16) -> String {
    match mode {
        mode::DATA_REGISTER => format!("D{}", reg),
        mode::ADDRESS_REGISTER => format!("A{}", reg),
        mode::INDIRECT => format!("(A{})", reg),
        mode::POSTINCREMENT => format!("(A{})+", reg),
        mode::PREDECREMENT => format!("-(A{})", reg),
        mode::DISPLACEMENT => {
            let d16 = read_ext() as i16;
            format!("(${:04X}, A{})", d16, reg)
        }
        mode::INDEXED => {
            let ext = read_ext();
            let idx_name = if (ext & field::BIT_INDEX_ADDRESS_REGISTER) != 0 {
                'A'
            } else {
                'D'
            };
            let idx_num = (ext >> field::BIT_INDEX_REGISTER_SHIFT) & field::MASK_REGISTER;
            let sz = if (ext & field::BIT_INDEX_LONG) != 0 {
                ".L"
            } else {
                ".W"
            };
            let d8 = (ext & 0xFF) as i8;
            format!("(${:02X}, A{}, {}{}{})", d8, reg, idx_name, idx_num, sz)
        }
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => {
                let w = read_ext();
                format!("(${:04X}).W", w)
            }
            extended_mode::ABSOLUTE_LONG => {
                let hi = read_ext() as u32;
                let lo = read_ext() as u32;
                format!("(${:08X}).L", (hi << 16) | lo)
            }
            extended_mode::PC_DISPLACEMENT => {
                let d16 = read_ext() as i16;
                format!("(${:04X}, PC)", d16)
            }
            extended_mode::PC_INDEXED => {
                let ext = read_ext();
                let idx_name = if (ext & field::BIT_INDEX_ADDRESS_REGISTER) != 0 {
                    'A'
                } else {
                    'D'
                };
                let idx_num = (ext >> field::BIT_INDEX_REGISTER_SHIFT) & field::MASK_REGISTER;
                let sz = if (ext & field::BIT_INDEX_LONG) != 0 {
                    ".L"
                } else {
                    ".W"
                };
                let d8 = (ext & 0xFF) as i8;
                format!("(${:02X}, PC, {}{}{})", d8, idx_name, idx_num, sz)
            }
            extended_mode::IMMEDIATE => {
                let val = read_ext();
                format!("#${:04X}", val)
            }
            _ => format!("UNKNOWN_EA(7, {})", reg),
        },
        _ => format!("UNKNOWN_EA({}, {})", mode, reg),
    }
}

/// Formats immediate operand with explicit size (1 = byte, 2 = word, 4 = long)
pub fn format_immediate(size_bytes: usize, mut read_ext: impl FnMut() -> u16) -> String {
    match size_bytes {
        1 => {
            let w = read_ext();
            format!("#${:02X}", (w & 0xFF) as u8)
        }
        2 => {
            let w = read_ext();
            format!("#${:04X}", w)
        }
        4 => {
            let hi = read_ext() as u32;
            let lo = read_ext() as u32;
            format!("#${:08X}", (hi << 16) | lo)
        }
        _ => "#$00".to_string(),
    }
}

/// Formats a 16-bit MOVEM register mask into standard assembly register list (e.g. "D0-D3/A0-A1")
pub fn format_movem_reg_list(mask: u16, is_predec: bool) -> String {
    if mask == 0 {
        return "<none>".to_string();
    }

    let mut regs = Vec::new();
    // Data registers D0..D7 in ascending order
    for k in 0..REGISTER_BANK_COUNT {
        let bit = if is_predec {
            2 * REGISTER_BANK_COUNT - 1 - k
        } else {
            k
        };
        if (mask & (1 << bit)) != 0 {
            regs.push(format!("D{}", k));
        }
    }
    // Address registers A0..A7 in ascending order
    for k in 0..REGISTER_BANK_COUNT {
        let bit = if is_predec {
            REGISTER_BANK_COUNT - 1 - k
        } else {
            REGISTER_BANK_COUNT + k
        };
        if (mask & (1 << bit)) != 0 {
            regs.push(format!("A{}", k));
        }
    }

    // Collapse consecutive ranges (e.g. D0, D1, D2 -> D0-D2)
    collapse_reg_ranges(&regs)
}

fn collapse_reg_ranges(regs: &[String]) -> String {
    if regs.is_empty() {
        return String::new();
    }

    let mut result = Vec::new();
    let mut i = 0;
    while i < regs.len() {
        let start = &regs[i];
        let prefix = start.chars().next().unwrap_or('D');
        let start_num: u8 = start[1..].parse().unwrap_or(0);

        let mut end_num = start_num;
        let mut j = i + 1;
        while j < regs.len() {
            let next = &regs[j];
            let next_prefix = next.chars().next().unwrap_or(' ');
            let next_num: u8 = next[1..].parse().unwrap_or(0);
            if next_prefix == prefix && next_num == end_num + 1 {
                end_num = next_num;
                j += 1;
            } else {
                break;
            }
        }

        if end_num >= start_num + 1 {
            result.push(format!("{}{}-{}{}", prefix, start_num, prefix, end_num));
            i = j;
        } else {
            result.push(format!("{}{}", prefix, start_num));
            i += 1;
        }
    }

    result.join("/")
}

/// Returns standard M68000 Bcc condition mnemonic (BRA, BSR, BNE, etc.)
pub fn bcc_condition_name(cond: u8) -> &'static str {
    match cond & condition::MASK_CONDITION {
        condition::TRUE => "BRA",
        condition::FALSE => "BSR",
        condition::HIGH => "BHI",
        condition::LOW_OR_SAME => "BLS",
        condition::CARRY_CLEAR => "BCC",
        condition::CARRY_SET => "BCS",
        condition::NOT_EQUAL => "BNE",
        condition::EQUAL => "BEQ",
        condition::OVERFLOW_CLEAR => "BVC",
        condition::OVERFLOW_SET => "BVS",
        condition::PLUS => "BPL",
        condition::MINUS => "BMI",
        condition::GREATER_OR_EQUAL => "BGE",
        condition::LESS_THAN => "BLT",
        condition::GREATER_THAN => "BGT",
        condition::LESS_OR_EQUAL => "BLE",
        _ => "Bcc",
    }
}

/// Returns standard M68000 DBcc condition mnemonic (DBT, DBRA, DBNE, etc.)
pub fn dbcc_condition_name(cond: u8) -> &'static str {
    match cond & condition::MASK_CONDITION {
        condition::TRUE => "DBT",
        condition::FALSE => "DBF",
        condition::HIGH => "DBHI",
        condition::LOW_OR_SAME => "DBLS",
        condition::CARRY_CLEAR => "DBCC",
        condition::CARRY_SET => "DBCS",
        condition::NOT_EQUAL => "DBNE",
        condition::EQUAL => "DBEQ",
        condition::OVERFLOW_CLEAR => "DBVC",
        condition::OVERFLOW_SET => "DBVS",
        condition::PLUS => "DBPL",
        condition::MINUS => "DBMI",
        condition::GREATER_OR_EQUAL => "DBGE",
        condition::LESS_THAN => "DBLT",
        condition::GREATER_THAN => "DBGT",
        condition::LESS_OR_EQUAL => "DBLE",
        _ => "DBcc",
    }
}

/// Returns standard M68000 Scc condition mnemonic (ST, SF, SNE, etc.)
pub fn scc_condition_name(cond: u8) -> &'static str {
    match cond & condition::MASK_CONDITION {
        condition::TRUE => "ST",
        condition::FALSE => "SF",
        condition::HIGH => "SHI",
        condition::LOW_OR_SAME => "SLS",
        condition::CARRY_CLEAR => "SCC",
        condition::CARRY_SET => "SCS",
        condition::NOT_EQUAL => "SNE",
        condition::EQUAL => "SEQ",
        condition::OVERFLOW_CLEAR => "SVC",
        condition::OVERFLOW_SET => "SVS",
        condition::PLUS => "SPL",
        condition::MINUS => "SMI",
        condition::GREATER_OR_EQUAL => "SGE",
        condition::LESS_THAN => "SLT",
        condition::GREATER_THAN => "SGT",
        condition::LESS_OR_EQUAL => "SLE",
        _ => "Scc",
    }
}
