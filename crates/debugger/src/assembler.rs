//! Built-in zero-dependency Motorola 68000 opcode assembler and hex parser
//!
//! Converts assembly mnemonic strings (e.g. "NOP", "MOVE.W D0, D1", "ADD.W #$12, D0")
//! or raw hexadecimal word sequences (e.g. "4E71", "3200", "33FC 0042 0007 0000")
//! into machine code words (`Vec<u16>`).

/// Parses either a raw hex word string or an assembly mnemonic into 16-bit instruction words
pub fn assemble_instruction(input: &str, pc: u32) -> Result<Vec<u16>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Instruction input cannot be empty".to_string());
    }

    // 1. Try parsing as raw hex words first (e.g. "4E71", "4E 71", "33FC 0042 0007 0000", "$4E71")
    if let Ok(words) = try_parse_hex_words(trimmed) {
        if !words.is_empty() {
            return Ok(words);
        }
    }

    // 2. Parse as assembly mnemonic and operands
    assemble_mnemonic(trimmed, pc)
}

/// Attempts to parse a string as space- or comma-separated hex words
fn try_parse_hex_words(input: &str) -> Result<Vec<u16>, ()> {
    let clean = input.replace('$', "");
    let tokens: Vec<&str> = clean
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.is_empty() {
        return Err(());
    }

    let mut words = Vec::new();
    let mut pending_byte: Option<u8> = None;

    for token in tokens {
        if token.len() == 4 {
            if let Ok(w) = u16::from_str_radix(token, 16) {
                words.push(w);
                continue;
            } else {
                return Err(());
            }
        } else if token.len() == 2 {
            if let Ok(b) = u8::from_str_radix(token, 16) {
                if let Some(prev) = pending_byte.take() {
                    words.push(((prev as u16) << 8) | (b as u16));
                } else {
                    pending_byte = Some(b);
                }
                continue;
            } else {
                return Err(());
            }
        } else if token.len() == 8 {
            if let Ok(val) = u32::from_str_radix(token, 16) {
                words.push((val >> 16) as u16);
                words.push(val as u16);
                continue;
            } else {
                return Err(());
            }
        } else {
            return Err(());
        }
    }

    if pending_byte.is_some() {
        return Err(()); // Odd byte count cannot form complete 16-bit words
    }

    Ok(words)
}

/// Mini-assembler for standard M68000 mnemonics
fn assemble_mnemonic(input: &str, pc: u32) -> Result<Vec<u16>, String> {
    let (mnem_part, op_part) = match input.split_once(|c: char| c.is_whitespace()) {
        Some((m, o)) => (m.trim().to_ascii_uppercase(), o.trim()),
        None => (input.trim().to_ascii_uppercase(), ""),
    };

    // Split mnemonic into base and size (e.g. "MOVE.W" -> ("MOVE", Some("W")))
    let (base_mnem, size_opt) = match mnem_part.split_once('.') {
        Some((b, s)) => (b, Some(s)),
        None => (mnem_part.as_str(), None),
    };

    // A. Inherent opcodes (no operands)
    match base_mnem {
        "NOP" => return Ok(vec![0x4E71]),
        "RTS" => return Ok(vec![0x4E75]),
        "RTE" => return Ok(vec![0x4E73]),
        "RTR" => return Ok(vec![0x4E77]),
        "RESET" => return Ok(vec![0x4E70]),
        "TRAPV" => return Ok(vec![0x4E76]),
        "ILLEGAL" => return Ok(vec![0x4AFC]),
        _ => {}
    }

    // B. TRAP #vector (0..15)
    if base_mnem == "TRAP" {
        let clean = op_part.trim().trim_start_matches('#');
        let vec_num = clean
            .parse::<u16>()
            .map_err(|_| format!("Invalid TRAP vector: '{}'", op_part))?;
        if vec_num > 15 {
            return Err("TRAP vector must be between 0 and 15".to_string());
        }
        return Ok(vec![0x4E40 | vec_num]);
    }

    // C. SWAP Dn
    if base_mnem == "SWAP" {
        let reg = parse_dn(op_part)?;
        return Ok(vec![0x4840 | (reg as u16)]);
    }

    // D. EXT.W Dn / EXT.L Dn
    if base_mnem == "EXT" {
        let reg = parse_dn(op_part)?;
        let opmode = match size_opt {
            Some("W") => 2,
            Some("L") => 3,
            _ => return Err("EXT requires .W or .L size specifier".to_string()),
        };
        return Ok(vec![0x4800 | (opmode << 6) | (reg as u16)]);
    }

    // E. MOVEQ #imm, Dn
    if base_mnem == "MOVEQ" {
        let (imm_str, reg_str) = split_two_operands(op_part)?;
        let reg = parse_dn(reg_str)?;
        let imm_val = parse_imm(imm_str)?;
        let d8 = (imm_val as i64) as i8; // 8-bit sign-extended
        return Ok(vec![0x7000 | ((reg as u16) << 9) | ((d8 as u8) as u16)]);
    }

    // F. CLR / TST / NOT / NEG / NEGX
    if base_mnem == "CLR"
        || base_mnem == "TST"
        || base_mnem == "NOT"
        || base_mnem == "NEG"
        || base_mnem == "NEGX"
    {
        let size_bits = parse_size_bits(size_opt)?;
        let (ea_mode, ea_reg, mut ext_words) = parse_ea(op_part)?;
        let base_code = match base_mnem {
            "NEGX" => 0x4000,
            "CLR" => 0x4200,
            "NEG" => 0x4400,
            "NOT" => 0x4600,
            "TST" => 0x4A00,
            _ => unreachable!(),
        };
        let mut words =
            vec![base_code | (size_bits << 6) | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        words.append(&mut ext_words);
        return Ok(words);
    }

    // G. LEA <ea>, An
    if base_mnem == "LEA" {
        let (ea_str, an_str) = split_two_operands(op_part)?;
        let an = parse_an(an_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(ea_str, false, true)?;
        let mut words =
            vec![0x41C0 | ((an as u16) << 9) | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        words.append(&mut ext);
        return Ok(words);
    }

    // H. PEA <ea>
    if base_mnem == "PEA" {
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(op_part, false, true)?;
        let mut words = vec![0x4840 | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        words.append(&mut ext);
        return Ok(words);
    }

    // I. LINK An, #disp / UNLK An
    if base_mnem == "LINK" {
        let (an_str, disp_str) = split_two_operands(op_part)?;
        let an = parse_an(an_str)?;
        let disp = parse_imm(disp_str)? as i16 as u16;
        return Ok(vec![0x4E50 | (an as u16), disp]);
    }
    if base_mnem == "UNLK" {
        let an = parse_an(op_part)?;
        return Ok(vec![0x4E58 | (an as u16)]);
    }

    // J. ADDQ / SUBQ #imm, <ea>
    if base_mnem == "ADDQ" || base_mnem == "SUBQ" {
        let size_bits = parse_size_bits(size_opt)?;
        let (imm_str, ea_str) = split_two_operands(op_part)?;
        let raw_val = parse_imm(imm_str)?;
        if raw_val < 1 || raw_val > 8 {
            return Err("ADDQ/SUBQ immediate data must be in range 1..8".to_string());
        }
        let data = (raw_val & 7) as u16;
        let (ea_mode, ea_reg, mut ext) = parse_ea(ea_str)?;
        let is_subq = base_mnem == "SUBQ";
        let mut words = vec![
            0x5000
                | (data << 9)
                | (if is_subq { 0x0100 } else { 0 })
                | (size_bits << 6)
                | ((ea_mode as u16) << 3)
                | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }

    // K. ADDA / SUBA <ea>, An
    if base_mnem == "ADDA" || base_mnem == "SUBA" {
        let is_long = match size_opt {
            Some("L") => true,
            Some("W") | None => false,
            _ => return Err(format!("{} requires .W or .L size", base_mnem)),
        };
        let (src_str, an_str) = split_two_operands(op_part)?;
        let an = parse_an(an_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(src_str, !is_long, is_long)?;
        let op_base = if base_mnem == "ADDA" { 0xD0C0 } else { 0x90C0 };
        let opmode = if is_long { 0x01C0 } else { 0x00C0 };
        let mut words =
            vec![op_base | ((an as u16) << 9) | opmode | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        words.append(&mut ext);
        return Ok(words);
    }

    // L. CMP / CMPA / CMPI
    if base_mnem == "CMPA" {
        let is_long = match size_opt {
            Some("L") => true,
            Some("W") | None => false,
            _ => return Err("CMPA requires .W or .L size".to_string()),
        };
        let (src_str, an_str) = split_two_operands(op_part)?;
        let an = parse_an(an_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(src_str, !is_long, is_long)?;
        let opmode = if is_long { 0x01C0 } else { 0x00C0 };
        let mut words =
            vec![0xB000 | ((an as u16) << 9) | opmode | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        words.append(&mut ext);
        return Ok(words);
    }
    if base_mnem == "CMPI" {
        let size_bits = parse_size_bits(size_opt)?;
        let (imm_str, ea_str) = split_two_operands(op_part)?;
        let imm_val = parse_imm(imm_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea(ea_str)?;
        let mut words = vec![0x0C00 | (size_bits << 6) | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        if size_bits == 2 {
            words.push((imm_val >> 16) as u16);
            words.push(imm_val as u16);
        } else {
            words.push(imm_val as u16);
        }
        words.append(&mut ext);
        return Ok(words);
    }
    if base_mnem == "CMP" {
        let size_bits = parse_size_bits(size_opt)?;
        let (src_str, dn_str) = split_two_operands(op_part)?;
        let dn = parse_dn(dn_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea(src_str)?;
        let mut words = vec![
            0xB000
                | ((dn as u16) << 9)
                | (size_bits << 6)
                | ((ea_mode as u16) << 3)
                | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }

    // M. EOR Dn, <ea>
    if base_mnem == "EOR" {
        let size_bits = parse_size_bits(size_opt)?;
        let (dn_str, dst_str) = split_two_operands(op_part)?;
        let dn = parse_dn(dn_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea(dst_str)?;
        let opmode = size_bits | 4;
        let mut words = vec![
            0xB100 | ((dn as u16) << 9) | (opmode << 6) | ((ea_mode as u16) << 3) | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }

    // N. MULU / MULS / DIVU / DIVS
    if base_mnem == "MULU" || base_mnem == "MULS" || base_mnem == "DIVU" || base_mnem == "DIVS" {
        let (src_str, dn_str) = split_two_operands(op_part)?;
        let dn = parse_dn(dn_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(src_str, true, false)?;
        let base_code = match base_mnem {
            "MULU" => 0xC0C0,
            "MULS" => 0xC1C0,
            "DIVU" => 0x80C0,
            "DIVS" => 0x81C0,
            _ => unreachable!(),
        };
        let mut words =
            vec![base_code | ((dn as u16) << 9) | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        words.append(&mut ext);
        return Ok(words);
    }

    // O. DBcc / DBRA Dn, <target>
    if base_mnem == "DBRA" || base_mnem.starts_with("DB") {
        let cond = match base_mnem {
            "DBT" => 0,
            "DBRA" | "DBF" => 1,
            "DBHI" => 2,
            "DBLS" => 3,
            "DBCC" => 4,
            "DBCS" => 5,
            "DBNE" => 6,
            "DBEQ" => 7,
            "DBVC" => 8,
            "DBVS" => 9,
            "DBPL" => 10,
            "DBMI" => 11,
            "DBGE" => 12,
            "DBLT" => 13,
            "DBGT" => 14,
            "DBLE" => 15,
            _ => 1,
        };
        let (dn_str, target_str) = split_two_operands(op_part)?;
        let dn = parse_dn(dn_str)?;
        let target_clean = target_str.trim().trim_start_matches('$');
        let target = u32::from_str_radix(target_clean, 16)
            .map_err(|_| format!("Invalid branch target address: '{}'", target_str))?;
        let pc_base = pc.wrapping_add(2);
        let disp = (target as i32) - (pc_base as i32);
        let d16 = disp as i16 as u16;
        return Ok(vec![0x50C8 | ((cond as u16) << 8) | (dn as u16), d16]);
    }

    // P. BRA / BSR / Bcc
    if let Some(cond) = parse_bcc_cond(base_mnem) {
        return assemble_branch(cond, op_part, pc);
    }

    // Q. JMP / JSR
    if base_mnem == "JMP" || base_mnem == "JSR" {
        let (ea_mode, ea_reg, mut ext_words) = parse_ea(op_part)?;
        let base_code = if base_mnem == "JSR" { 0x4E80 } else { 0x4EC0 };
        let mut words = vec![base_code | ((ea_mode as u16) << 3) | (ea_reg as u16)];
        words.append(&mut ext_words);
        return Ok(words);
    }

    // R. MOVE / MOVEA
    if base_mnem == "MOVE" || base_mnem == "MOVEA" {
        return assemble_move(size_opt, op_part);
    }

    // S. ADD / SUB / AND / OR
    if base_mnem == "ADD" || base_mnem == "SUB" || base_mnem == "AND" || base_mnem == "OR" {
        return assemble_alu(base_mnem, size_opt, op_part);
    }

    Err(format!(
        "Unsupported mnemonic '{}'. Try entering hex opcode words (e.g. '4E71').",
        mnem_part
    ))
}

fn parse_bcc_cond(mnem: &str) -> Option<u8> {
    match mnem {
        "BRA" => Some(0),
        "BSR" => Some(1),
        "BHI" => Some(2),
        "BLS" => Some(3),
        "BCC" => Some(4),
        "BCS" => Some(5),
        "BNE" => Some(6),
        "BEQ" => Some(7),
        "BVC" => Some(8),
        "BVS" => Some(9),
        "BPL" => Some(10),
        "BMI" => Some(11),
        "BGE" => Some(12),
        "BLT" => Some(13),
        "BGT" => Some(14),
        "BLE" => Some(15),
        _ => None,
    }
}

fn assemble_branch(cond: u8, op_part: &str, pc: u32) -> Result<Vec<u16>, String> {
    let clean = op_part.trim().trim_start_matches('$');
    let target = u32::from_str_radix(clean, 16)
        .map_err(|_| format!("Invalid branch target address: '{}'", op_part))?;

    if target & 1 != 0 {
        return Err(format!(
            "Branch target address must be 16-bit aligned (even): ${:06X}",
            target
        ));
    }

    let pc_base = pc.wrapping_add(2);
    let disp = (target as i32) - (pc_base as i32);

    if (-128..=127).contains(&disp) && disp != 0 {
        // 8-bit short displacement
        let d8 = (disp as i8) as u8;
        Ok(vec![0x6000 | ((cond as u16) << 8) | (d8 as u16)])
    } else if (-32768..=32767).contains(&disp) {
        // 16-bit word displacement
        let d16 = disp as i16 as u16;
        Ok(vec![0x6000 | ((cond as u16) << 8), d16])
    } else {
        Err(format!("Branch displacement out of range: {:+}", disp))
    }
}

fn assemble_move(size_opt: Option<&str>, op_part: &str) -> Result<Vec<u16>, String> {
    let (size_bits, is_word, is_long) = match size_opt {
        Some("B") => (1u16, false, false),
        Some("W") | None => (3u16, true, false),
        Some("L") => (2u16, false, true),
        _ => return Err("Invalid size specifier for MOVE".to_string()),
    };

    let (src_str, dst_str) = split_two_operands(op_part)?;
    let (src_mode, src_reg, mut src_ext) = parse_ea_sized(src_str, is_word, is_long)?;
    let (dst_mode, dst_reg, mut dst_ext) = parse_ea_sized(dst_str, is_word, is_long)?;

    let base = (size_bits << 12)
        | ((dst_reg as u16) << 9)
        | ((dst_mode as u16) << 6)
        | ((src_mode as u16) << 3)
        | (src_reg as u16);

    let mut words = vec![base];
    words.append(&mut src_ext);
    words.append(&mut dst_ext);
    Ok(words)
}

fn assemble_alu(
    base_mnem: &str,
    size_opt: Option<&str>,
    op_part: &str,
) -> Result<Vec<u16>, String> {
    let op_group = match base_mnem {
        "OR" => 0x8u16,
        "SUB" => 0x9u16,
        "AND" => 0xCu16,
        "ADD" => 0xDu16,
        _ => unreachable!(),
    };

    let (sz_code, is_word, is_long) = match size_opt {
        Some("B") => (0u16, false, false),
        Some("W") | None => (1u16, true, false),
        Some("L") => (2u16, false, true),
        _ => return Err("Invalid size specifier for ALU operation".to_string()),
    };

    let (op1, op2) = split_two_operands(op_part)?;

    // Case 1: <ea>, Dn -> opmode 0, 1, 2
    if let Ok(reg_d) = parse_dn(op2) {
        let (ea_mode, ea_reg, mut ext_words) = parse_ea_sized(op1, is_word, is_long)?;
        let opmode = sz_code;
        let base = (op_group << 12)
            | ((reg_d as u16) << 9)
            | (opmode << 6)
            | ((ea_mode as u16) << 3)
            | (ea_reg as u16);
        let mut words = vec![base];
        words.append(&mut ext_words);
        return Ok(words);
    }

    // Case 2: Dn, <ea> -> opmode 4, 5, 6
    if let Ok(reg_d) = parse_dn(op1) {
        let (ea_mode, ea_reg, mut ext_words) = parse_ea_sized(op2, is_word, is_long)?;
        let opmode = sz_code | 4;
        let base = (op_group << 12)
            | ((reg_d as u16) << 9)
            | (opmode << 6)
            | ((ea_mode as u16) << 3)
            | (ea_reg as u16);
        let mut words = vec![base];
        words.append(&mut ext_words);
        return Ok(words);
    }

    Err(format!(
        "{} requires at least one data register operand (e.g. 'D0, D1' or '#$10, D0')",
        base_mnem
    ))
}

fn parse_size_bits(size_opt: Option<&str>) -> Result<u16, String> {
    match size_opt {
        Some("B") => Ok(0),
        Some("W") | None => Ok(1),
        Some("L") => Ok(2),
        _ => Err("Invalid size specifier: expected .B, .W, or .L".to_string()),
    }
}

fn split_two_operands(ops: &str) -> Result<(&str, &str), String> {
    match ops.split_once(',') {
        Some((a, b)) => Ok((a.trim(), b.trim())),
        None => Err(format!(
            "Expected 2 comma-separated operands, found: '{}'",
            ops
        )),
    }
}

fn parse_dn(s: &str) -> Result<u8, String> {
    let clean = s.trim().to_ascii_uppercase();
    if clean.starts_with('D') && clean.len() == 2 {
        let digit = clean.chars().nth(1).unwrap_or(' ');
        if let Some(d) = digit.to_digit(8) {
            return Ok(d as u8);
        }
    }
    Err(format!("Expected data register D0-D7, found: '{}'", s))
}

fn parse_an(s: &str) -> Result<u8, String> {
    let clean = s.trim().to_ascii_uppercase();
    if clean == "SP" {
        return Ok(7);
    }
    if clean.starts_with('A') && clean.len() == 2 {
        let digit = clean.chars().nth(1).unwrap_or(' ');
        if let Some(d) = digit.to_digit(8) {
            return Ok(d as u8);
        }
    }
    Err(format!("Expected address register A0-A7, found: '{}'", s))
}

fn parse_imm(s: &str) -> Result<u32, String> {
    let clean = s.trim().trim_start_matches('#');
    let (is_neg, rest) = if let Some(stripped) = clean.strip_prefix('-') {
        (true, stripped)
    } else {
        (false, clean)
    };

    let val = if let Some(hex_str) = rest
        .strip_prefix('$')
        .or_else(|| rest.strip_prefix("0x"))
        .or_else(|| rest.strip_prefix("0X"))
    {
        u32::from_str_radix(hex_str, 16)
            .map_err(|_| format!("Invalid immediate hex value: '{}'", s))?
    } else if let Ok(dec) = rest.parse::<u32>() {
        dec
    } else {
        u32::from_str_radix(rest, 16).map_err(|_| format!("Invalid immediate value: '{}'", s))?
    };

    if is_neg {
        Ok((-(val as i32)) as u32)
    } else {
        Ok(val)
    }
}

fn parse_ea(s: &str) -> Result<(u8, u8, Vec<u16>), String> {
    parse_ea_sized(s, true, false)
}

fn parse_ea_sized(s: &str, is_word: bool, is_long: bool) -> Result<(u8, u8, Vec<u16>), String> {
    let clean = s.trim().to_ascii_uppercase();

    // 1. Data Register Direct: Dn
    if let Ok(reg) = parse_dn(&clean) {
        return Ok((0, reg, Vec::new()));
    }

    // 2. Address Register Direct: An
    if let Ok(reg) = parse_an(&clean) {
        return Ok((1, reg, Vec::new()));
    }

    // 3. Address Register Indirect Postincrement: (An)+
    if clean.ends_with(")+") && clean.starts_with('(') {
        let inner = &clean[1..clean.len() - 2];
        let reg = parse_an(inner)?;
        return Ok((3, reg, Vec::new()));
    }

    // 4. Address Register Indirect Predecrement: -(An)
    if clean.starts_with("-(") && clean.ends_with(')') {
        let inner = &clean[2..clean.len() - 1];
        let reg = parse_an(inner)?;
        return Ok((4, reg, Vec::new()));
    }

    // 5. Address Register Indirect: (An)
    if clean.starts_with('(') && clean.ends_with(')') {
        let inner = &clean[1..clean.len() - 1];
        if let Ok(reg) = parse_an(inner) {
            return Ok((2, reg, Vec::new()));
        }
    }

    // 6. Immediate Data: #$val or #val
    if clean.starts_with('#') {
        let val = parse_imm(&clean)?;
        if is_long {
            return Ok((7, 4, vec![(val >> 16) as u16, val as u16]));
        } else if is_word {
            return Ok((7, 4, vec![val as u16]));
        } else {
            // Byte immediate occupies full 16-bit extension word (low byte)
            return Ok((7, 4, vec![val as u8 as u16]));
        }
    }

    // 7. Absolute Long or Word: $123456, ($123456).L, ($1234).W
    let is_explicit_long = clean.ends_with(".L");
    let is_explicit_word = clean.ends_with(".W");
    let mut inner = clean.as_str();
    if is_explicit_long {
        inner = &inner[..inner.len() - 2];
    } else if is_explicit_word {
        inner = &inner[..inner.len() - 2];
    }
    let inner = inner.trim();
    let inner = if inner.starts_with('(') && inner.ends_with(')') {
        inner[1..inner.len() - 1].trim()
    } else {
        inner
    };
    let hex_clean = inner.trim_start_matches('$');
    if let Ok(addr) = u32::from_str_radix(hex_clean, 16) {
        if addr > 0xFFFF || is_explicit_long || is_long {
            return Ok((7, 1, vec![(addr >> 16) as u16, addr as u16]));
        } else {
            return Ok((7, 0, vec![addr as u16]));
        }
    }

    Err(format!("Unrecognized effective address operand: '{}'", s))
}
