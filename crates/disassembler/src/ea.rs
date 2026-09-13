//! Motorola 68000 Effective Address and Register List Formatting Helpers
//!
//! Provides zero-panic formatting for addressing modes, immediate values,
//! condition codes, and MOVEM register masks.

/// Formats an effective address (EA) based on mode (0..7) and register (0..7)
pub fn format_ea(mode: u8, reg: u8, mut read_ext: impl FnMut() -> u16) -> String {
    match mode {
        0 => format!("D{}", reg),
        1 => format!("A{}", reg),
        2 => format!("(A{})", reg),
        3 => format!("(A{})+", reg),
        4 => format!("-(A{})", reg),
        5 => {
            let d16 = read_ext() as i16;
            format!("(${:04X}, A{})", d16, reg)
        }
        6 => {
            let ext = read_ext();
            let idx_name = if (ext & 0x8000) != 0 { 'A' } else { 'D' };
            let idx_num = (ext >> 12) & 0x07;
            let sz = if (ext & 0x0800) != 0 { ".L" } else { ".W" };
            let d8 = (ext & 0xFF) as i8;
            format!("(${:02X}, A{}, {}{}{})", d8, reg, idx_name, idx_num, sz)
        }
        7 => match reg {
            0 => {
                let w = read_ext();
                format!("(${:04X}).W", w)
            }
            1 => {
                let hi = read_ext() as u32;
                let lo = read_ext() as u32;
                format!("(${:08X}).L", (hi << 16) | lo)
            }
            2 => {
                let d16 = read_ext() as i16;
                format!("(${:04X}, PC)", d16)
            }
            3 => {
                let ext = read_ext();
                let idx_name = if (ext & 0x8000) != 0 { 'A' } else { 'D' };
                let idx_num = (ext >> 12) & 0x07;
                let sz = if (ext & 0x0800) != 0 { ".L" } else { ".W" };
                let d8 = (ext & 0xFF) as i8;
                format!("(${:02X}, PC, {}{}{})", d8, idx_name, idx_num, sz)
            }
            4 => {
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
    for k in 0..8 {
        let bit = if is_predec { 15 - k } else { k };
        if (mask & (1 << bit)) != 0 {
            regs.push(format!("D{}", k));
        }
    }
    // Address registers A0..A7 in ascending order
    for k in 0..8 {
        let bit = if is_predec { 7 - k } else { 8 + k };
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
    match cond & 0x0F {
        0 => "BRA",
        1 => "BSR",
        2 => "BHI",
        3 => "BLS",
        4 => "BCC",
        5 => "BCS",
        6 => "BNE",
        7 => "BEQ",
        8 => "BVC",
        9 => "BVS",
        10 => "BPL",
        11 => "BMI",
        12 => "BGE",
        13 => "BLT",
        14 => "BGT",
        15 => "BLE",
        _ => "Bcc",
    }
}

/// Returns standard M68000 DBcc condition mnemonic (DBT, DBRA, DBNE, etc.)
pub fn dbcc_condition_name(cond: u8) -> &'static str {
    match cond & 0x0F {
        0 => "DBT",
        1 => "DBF",
        2 => "DBHI",
        3 => "DBLS",
        4 => "DBCC",
        5 => "DBCS",
        6 => "DBNE",
        7 => "DBEQ",
        8 => "DBVC",
        9 => "DBVS",
        10 => "DBPL",
        11 => "DBMI",
        12 => "DBGE",
        13 => "DBLT",
        14 => "DBGT",
        15 => "DBLE",
        _ => "DBcc",
    }
}

/// Returns standard M68000 Scc condition mnemonic (ST, SF, SNE, etc.)
pub fn scc_condition_name(cond: u8) -> &'static str {
    match cond & 0x0F {
        0 => "ST",
        1 => "SF",
        2 => "SHI",
        3 => "SLS",
        4 => "SCC",
        5 => "SCS",
        6 => "SNE",
        7 => "SEQ",
        8 => "SVC",
        9 => "SVS",
        10 => "SPL",
        11 => "SMI",
        12 => "SGE",
        13 => "SLT",
        14 => "SGT",
        15 => "SLE",
        _ => "Scc",
    }
}
