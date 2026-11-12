//! Built-in Motorola 68000 opcode assembler and hex parser
//!
//! Converts assembly mnemonic strings (e.g. "NOP", "MOVE.W D0, D1", "ADD.W #$12, D0")
//! or raw hexadecimal word sequences (e.g. "4E71", "3200", "33FC 0042 0007 0000")
//! into machine code words (`Vec<u16>`).

use cpu::micro::encoding::{condition, extended_mode, field, mode, opcode, operand_size};
const QUICK_MIN: u32 = 1;
const QUICK_MAX: u32 = 8;
const EXT_WORD_OPMODE: u16 = 2;
const EXT_LONG_OPMODE: u16 = 3;
const BIT_MEMORY_DESTINATION: u16 = 4;
const BIT_OPCODE_LINE_SHIFT: u16 = 12;
const MOVE_BYTE_SIZE: u16 = 1;
const MOVE_WORD_SIZE: u16 = 3;
const MOVE_LONG_SIZE: u16 = 2;
const ILLEGAL_OPCODE: u16 = 0x4AFC;
const EXT_BASE: u16 = 0x4800;
const BIT_QUICK_SUBTRACT: u16 = 0x0100;
const ADDA_BASE: u16 = 0xD0C0;
const SUBA_BASE: u16 = 0x90C0;
const ADDRESS_LONG_OPMODE: u16 = 0x01C0;
const ADDRESS_WORD_OPMODE: u16 = 0x00C0;
const EOR_BASE: u16 = 0xB100;
const MULU_BASE: u16 = 0xC0C0;
const MULS_BASE: u16 = 0xC1C0;
const DIVU_BASE: u16 = 0x80C0;
const DIVS_BASE: u16 = 0x81C0;
const DBCC_BASE: u16 = 0x50C8;
const JSR_BASE: u16 = 0x4E80;
const JMP_BASE: u16 = 0x4EC0;
const OR_GROUP: u16 = 0x0008;
const SUB_GROUP: u16 = 0x0009;
const AND_GROUP: u16 = 0x000C;
const ADD_GROUP: u16 = 0x000D;
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
                    words.push(((prev as u16) << condition::BIT_SHIFT) | (b as u16));
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
        "NOP" => return Ok(vec![(opcode::NOP_OPCODE as u16)]),
        "RTS" => return Ok(vec![(opcode::RTS_OPCODE as u16)]),
        "RTE" => return Ok(vec![(opcode::RTE_OPCODE as u16)]),
        "RTR" => return Ok(vec![(opcode::RTR_OPCODE as u16)]),
        "RESET" => return Ok(vec![(opcode::RESET_OPCODE as u16)]),
        "TRAPV" => return Ok(vec![(opcode::TRAPV_OPCODE as u16)]),
        "ILLEGAL" => return Ok(vec![ILLEGAL_OPCODE]),
        _ => {}
    }

    // B. TRAP #vector (0..15)
    if base_mnem == "TRAP" {
        let clean = op_part.trim().trim_start_matches('#');
        let vec_num = clean
            .parse::<u16>()
            .map_err(|_| format!("Invalid TRAP vector: '{}'", op_part))?;
        if vec_num > field::MASK_TRAP_NUMBER {
            return Err("TRAP vector must be between 0 and 15".to_string());
        }
        return Ok(vec![(opcode::TRAP_FIRST as u16) | vec_num]);
    }

    // C. SWAP Dn
    if base_mnem == "SWAP" {
        let reg = parse_dn(op_part)?;
        return Ok(vec![(opcode::SWAP_OPCODE as u16) | (reg as u16)]);
    }

    // D. EXT.W Dn / EXT.L Dn
    if base_mnem == "EXT" {
        let reg = parse_dn(op_part)?;
        let opmode = match size_opt {
            Some("W") => EXT_WORD_OPMODE,
            Some("L") => EXT_LONG_OPMODE,
            _ => return Err("EXT requires .W or .L size specifier".to_string()),
        };
        return Ok(vec![
            EXT_BASE | (opmode << field::BIT_SIZE_SHIFT) | (reg as u16),
        ]);
    }

    // E. MOVEQ #imm, Dn
    if base_mnem == "MOVEQ" {
        let (imm_str, reg_str) = split_two_operands(op_part)?;
        let reg = parse_dn(reg_str)?;
        let imm_val = parse_imm(imm_str)?;
        let d8 = (imm_val as i64) as i8; // 8-bit sign-extended
        return Ok(vec![
            (opcode::MOVEQ_FIRST as u16)
                | ((reg as u16) << field::BIT_DEST_REGISTER_SHIFT)
                | ((d8 as u8) as u16),
        ]);
    }

    // F. CLR / TST / NOT / NEG / NEGX
    if matches!(base_mnem, "CLR" | "TST" | "NOT" | "NEG" | "NEGX") {
        let size_bits = parse_size_bits(size_opt)?;
        let (ea_mode, ea_reg, mut ext_words) = parse_ea(op_part)?;
        let base_code = match base_mnem {
            "NEGX" => opcode::NEGX_FIRST as u16,
            "CLR" => opcode::CLR_FIRST as u16,
            "NEG" => opcode::NEG_FIRST as u16,
            "NOT" => opcode::NOT_FIRST as u16,
            "TST" => opcode::TST_FIRST as u16,
            _ => return Err(format!("Unsupported unary operation: {base_mnem}")),
        };
        let mut words = vec![
            base_code
                | (size_bits << field::BIT_SIZE_SHIFT)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
        words.append(&mut ext_words);
        return Ok(words);
    }

    // G. LEA <ea>, An
    if base_mnem == "LEA" {
        let (ea_str, an_str) = split_two_operands(op_part)?;
        let an = parse_an(an_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(ea_str, false, true)?;
        let mut words = vec![
            (opcode::LEA_PATTERN as u16)
                | ((an as u16) << field::BIT_DEST_REGISTER_SHIFT)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }

    // H. PEA <ea>
    if base_mnem == "PEA" {
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(op_part, false, true)?;
        let mut words = vec![
            (opcode::PEA_FIRST as u16)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }

    // I. LINK An, #disp / UNLK An
    if base_mnem == "LINK" {
        let (an_str, disp_str) = split_two_operands(op_part)?;
        let an = parse_an(an_str)?;
        let disp = parse_imm(disp_str)? as i16 as u16;
        return Ok(vec![(opcode::LINK_OPCODE as u16) | (an as u16), disp]);
    }
    if base_mnem == "UNLK" {
        let an = parse_an(op_part)?;
        return Ok(vec![(opcode::UNLK_OPCODE as u16) | (an as u16)]);
    }

    // J. ADDQ / SUBQ #imm, <ea>
    if base_mnem == "ADDQ" || base_mnem == "SUBQ" {
        let size_bits = parse_size_bits(size_opt)?;
        let (imm_str, ea_str) = split_two_operands(op_part)?;
        let raw_val = parse_imm(imm_str)?;
        if raw_val < QUICK_MIN || raw_val > QUICK_MAX {
            return Err("ADDQ/SUBQ immediate data must be in range 1..8".to_string());
        }
        let data = (raw_val & field::MASK_REGISTER as u32) as u16;
        let (ea_mode, ea_reg, mut ext) = parse_ea(ea_str)?;
        let is_subq = base_mnem == "SUBQ";
        let mut words = vec![
            (opcode::QUICK_SCC_DBCC_FIRST as u16)
                | (data << field::BIT_DEST_REGISTER_SHIFT)
                | (if is_subq { BIT_QUICK_SUBTRACT } else { 0 })
                | (size_bits << field::BIT_SIZE_SHIFT)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
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
        let op_base = if base_mnem == "ADDA" {
            ADDA_BASE
        } else {
            SUBA_BASE
        };
        let opmode = if is_long {
            ADDRESS_LONG_OPMODE
        } else {
            ADDRESS_WORD_OPMODE
        };
        let mut words = vec![
            op_base
                | ((an as u16) << field::BIT_DEST_REGISTER_SHIFT)
                | opmode
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
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
        let opmode = if is_long {
            ADDRESS_LONG_OPMODE
        } else {
            ADDRESS_WORD_OPMODE
        };
        let mut words = vec![
            (opcode::CMP_EOR_FIRST as u16)
                | ((an as u16) << field::BIT_DEST_REGISTER_SHIFT)
                | opmode
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }
    if base_mnem == "CMPI" {
        let size_bits = parse_size_bits(size_opt)?;
        let (imm_str, ea_str) = split_two_operands(op_part)?;
        let imm_val = parse_imm(imm_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea(ea_str)?;
        let mut words = vec![
            (opcode::CMPI_FIRST as u16)
                | (size_bits << field::BIT_SIZE_SHIFT)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
        if size_bits == operand_size::LONG as u16 {
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
            (opcode::CMP_EOR_FIRST as u16)
                | ((dn as u16) << field::BIT_DEST_REGISTER_SHIFT)
                | (size_bits << field::BIT_SIZE_SHIFT)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
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
        let opmode = size_bits | BIT_MEMORY_DESTINATION;
        let mut words = vec![
            EOR_BASE
                | ((dn as u16) << field::BIT_DEST_REGISTER_SHIFT)
                | (opmode << field::BIT_SIZE_SHIFT)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }

    // N. MULU / MULS / DIVU / DIVS
    if matches!(base_mnem, "MULU" | "MULS" | "DIVU" | "DIVS") {
        let (src_str, dn_str) = split_two_operands(op_part)?;
        let dn = parse_dn(dn_str)?;
        let (ea_mode, ea_reg, mut ext) = parse_ea_sized(src_str, true, false)?;
        let base_code = match base_mnem {
            "MULU" => MULU_BASE,
            "MULS" => MULS_BASE,
            "DIVU" => DIVU_BASE,
            "DIVS" => DIVS_BASE,
            _ => {
                return Err(format!(
                    "Unsupported multiply/divide operation: {base_mnem}"
                ))
            }
        };
        let mut words = vec![
            base_code
                | ((dn as u16) << field::BIT_DEST_REGISTER_SHIFT)
                | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
                | (ea_reg as u16),
        ];
        words.append(&mut ext);
        return Ok(words);
    }

    // O. DBcc / DBRA Dn, <target>
    if base_mnem == "DBRA" || base_mnem.starts_with("DB") {
        let cond = match base_mnem {
            "DBT" => condition::TRUE,
            "DBRA" | "DBF" => condition::FALSE,
            "DBHI" => condition::HIGH,
            "DBLS" => condition::LOW_OR_SAME,
            "DBCC" => condition::CARRY_CLEAR,
            "DBCS" => condition::CARRY_SET,
            "DBNE" => condition::NOT_EQUAL,
            "DBEQ" => condition::EQUAL,
            "DBVC" => condition::OVERFLOW_CLEAR,
            "DBVS" => condition::OVERFLOW_SET,
            "DBPL" => condition::PLUS,
            "DBMI" => condition::MINUS,
            "DBGE" => condition::GREATER_OR_EQUAL,
            "DBLT" => condition::LESS_THAN,
            "DBGT" => condition::GREATER_THAN,
            "DBLE" => condition::LESS_OR_EQUAL,
            _ => condition::FALSE,
        };
        let (dn_str, target_str) = split_two_operands(op_part)?;
        let dn = parse_dn(dn_str)?;
        let target_clean = target_str.trim().trim_start_matches('$');
        let target = u32::from_str_radix(target_clean, 16)
            .map_err(|_| format!("Invalid branch target address: '{}'", target_str))?;
        let pc_base = pc.wrapping_add(2);
        let disp = (target as i32) - (pc_base as i32);
        let d16 = disp as i16 as u16;
        return Ok(vec![
            DBCC_BASE | ((cond as u16) << condition::BIT_SHIFT) | (dn as u16),
            d16,
        ]);
    }

    // P. BRA / BSR / Bcc
    if let Some(cond) = parse_bcc_cond(base_mnem) {
        return assemble_branch(cond, op_part, pc);
    }

    // Q. JMP / JSR
    if base_mnem == "JMP" || base_mnem == "JSR" {
        let (ea_mode, ea_reg, mut ext_words) = parse_ea(op_part)?;
        let base_code = if base_mnem == "JSR" {
            JSR_BASE
        } else {
            JMP_BASE
        };
        let mut words =
            vec![base_code | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT) | (ea_reg as u16)];
        words.append(&mut ext_words);
        return Ok(words);
    }

    // R. MOVE / MOVEA
    if base_mnem == "MOVE" || base_mnem == "MOVEA" {
        return assemble_move(size_opt, op_part);
    }

    // S. ADD / SUB / AND / OR
    if matches!(base_mnem, "ADD" | "SUB" | "AND" | "OR") {
        return assemble_alu(base_mnem, size_opt, op_part);
    }

    Err(format!(
        "Unsupported mnemonic '{}'. Try entering hex opcode words (e.g. '4E71').",
        mnem_part
    ))
}

fn parse_bcc_cond(mnem: &str) -> Option<u8> {
    match mnem {
        "BRA" => Some(condition::TRUE),
        "BSR" => Some(condition::FALSE),
        "BHI" => Some(condition::HIGH),
        "BLS" => Some(condition::LOW_OR_SAME),
        "BCC" => Some(condition::CARRY_CLEAR),
        "BCS" => Some(condition::CARRY_SET),
        "BNE" => Some(condition::NOT_EQUAL),
        "BEQ" => Some(condition::EQUAL),
        "BVC" => Some(condition::OVERFLOW_CLEAR),
        "BVS" => Some(condition::OVERFLOW_SET),
        "BPL" => Some(condition::PLUS),
        "BMI" => Some(condition::MINUS),
        "BGE" => Some(condition::GREATER_OR_EQUAL),
        "BLT" => Some(condition::LESS_THAN),
        "BGT" => Some(condition::GREATER_THAN),
        "BLE" => Some(condition::LESS_OR_EQUAL),
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

    if (i32::from(i8::MIN)..=i32::from(i8::MAX)).contains(&disp) && disp != 0 {
        // 8-bit short displacement
        let d8 = (disp as i8) as u8;
        Ok(vec![
            (opcode::BRANCH_FIRST as u16) | ((cond as u16) << condition::BIT_SHIFT) | (d8 as u16),
        ])
    } else if (i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&disp) {
        // 16-bit word displacement
        let d16 = disp as i16 as u16;
        Ok(vec![
            (opcode::BRANCH_FIRST as u16) | ((cond as u16) << condition::BIT_SHIFT),
            d16,
        ])
    } else {
        Err(format!("Branch displacement out of range: {:+}", disp))
    }
}

fn assemble_move(size_opt: Option<&str>, op_part: &str) -> Result<Vec<u16>, String> {
    let (size_bits, is_word, is_long) = match size_opt {
        Some("B") => (MOVE_BYTE_SIZE, false, false),
        Some("W") | None => (MOVE_WORD_SIZE, true, false),
        Some("L") => (MOVE_LONG_SIZE, false, true),
        _ => return Err("Invalid size specifier for MOVE".to_string()),
    };

    let (src_str, dst_str) = split_two_operands(op_part)?;
    let (src_mode, src_reg, mut src_ext) = parse_ea_sized(src_str, is_word, is_long)?;
    let (dst_mode, dst_reg, mut dst_ext) = parse_ea_sized(dst_str, is_word, is_long)?;

    let base = (size_bits << field::BIT_INDEX_REGISTER_SHIFT)
        | ((dst_reg as u16) << field::BIT_DEST_REGISTER_SHIFT)
        | ((dst_mode as u16) << field::BIT_MOVE_DEST_MODE_SHIFT)
        | ((src_mode as u16) << field::BIT_EA_MODE_SHIFT)
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
        "OR" => OR_GROUP,
        "SUB" => SUB_GROUP,
        "AND" => AND_GROUP,
        "ADD" => ADD_GROUP,
        _ => return Err(format!("Unsupported ALU operation: {base_mnem}")),
    };

    let (sz_code, is_word, is_long) = match size_opt {
        Some("B") => (operand_size::BYTE as u16, false, false),
        Some("W") | None => (operand_size::WORD as u16, true, false),
        Some("L") => (operand_size::LONG as u16, false, true),
        _ => return Err("Invalid size specifier for ALU operation".to_string()),
    };

    let (op1, op2) = split_two_operands(op_part)?;

    // Case 1: <ea>, Dn -> opmode 0, 1, 2
    if let Ok(reg_d) = parse_dn(op2) {
        let (ea_mode, ea_reg, mut ext_words) = parse_ea_sized(op1, is_word, is_long)?;
        let opmode = sz_code;
        let base = (op_group << BIT_OPCODE_LINE_SHIFT)
            | ((reg_d as u16) << field::BIT_DEST_REGISTER_SHIFT)
            | (opmode << field::BIT_SIZE_SHIFT)
            | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
            | (ea_reg as u16);
        let mut words = vec![base];
        words.append(&mut ext_words);
        return Ok(words);
    }

    // Case 2: Dn, <ea> -> opmode 4, 5, 6
    if let Ok(reg_d) = parse_dn(op1) {
        let (ea_mode, ea_reg, mut ext_words) = parse_ea_sized(op2, is_word, is_long)?;
        let opmode = sz_code | BIT_MEMORY_DESTINATION;
        let base = (op_group << BIT_OPCODE_LINE_SHIFT)
            | ((reg_d as u16) << field::BIT_DEST_REGISTER_SHIFT)
            | (opmode << field::BIT_SIZE_SHIFT)
            | ((ea_mode as u16) << field::BIT_EA_MODE_SHIFT)
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
        Some("B") => Ok(operand_size::BYTE as u16),
        Some("W") | None => Ok(operand_size::WORD as u16),
        Some("L") => Ok(operand_size::LONG as u16),
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
        if let Some(d) = digit.to_digit(cpu::state::REGISTER_COUNT as u32) {
            return Ok(d as u8);
        }
    }
    Err(format!("Expected data register D0-D7, found: '{}'", s))
}

fn parse_an(s: &str) -> Result<u8, String> {
    let clean = s.trim().to_ascii_uppercase();
    if clean == "SP" {
        return Ok(cpu::state::REGISTER_STACK_POINTER as u8);
    }
    if clean.starts_with('A') && clean.len() == 2 {
        let digit = clean.chars().nth(1).unwrap_or(' ');
        if let Some(d) = digit.to_digit(cpu::state::REGISTER_COUNT as u32) {
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
        return Ok((mode::DATA_REGISTER, reg, Vec::new()));
    }

    // 2. Address Register Direct: An
    if let Ok(reg) = parse_an(&clean) {
        return Ok((mode::ADDRESS_REGISTER, reg, Vec::new()));
    }

    // 3. Address Register Indirect Postincrement: (An)+
    if clean.ends_with(")+") && clean.starts_with('(') {
        let inner = &clean[1..clean.len() - 2];
        let reg = parse_an(inner)?;
        return Ok((mode::POSTINCREMENT, reg, Vec::new()));
    }

    // 4. Address Register Indirect Predecrement: -(An)
    if clean.starts_with("-(") && clean.ends_with(')') {
        let inner = &clean[2..clean.len() - 1];
        let reg = parse_an(inner)?;
        return Ok((mode::PREDECREMENT, reg, Vec::new()));
    }

    // 5. Address Register Indirect: (An)
    if clean.starts_with('(') && clean.ends_with(')') {
        let inner = &clean[1..clean.len() - 1];
        if let Ok(reg) = parse_an(inner) {
            return Ok((mode::INDIRECT, reg, Vec::new()));
        }
    }

    // 6. Immediate Data: #$val or #val
    if clean.starts_with('#') {
        let val = parse_imm(&clean)?;
        if is_long {
            return Ok((
                mode::EXTENDED,
                extended_mode::IMMEDIATE,
                vec![(val >> 16) as u16, val as u16],
            ));
        } else if is_word {
            return Ok((mode::EXTENDED, extended_mode::IMMEDIATE, vec![val as u16]));
        } else {
            // Byte immediate occupies full 16-bit extension word (low byte)
            return Ok((
                mode::EXTENDED,
                extended_mode::IMMEDIATE,
                vec![val as u8 as u16],
            ));
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
            return Ok((
                mode::EXTENDED,
                extended_mode::ABSOLUTE_LONG,
                vec![(addr >> 16) as u16, addr as u16],
            ));
        } else {
            return Ok((
                mode::EXTENDED,
                extended_mode::ABSOLUTE_WORD,
                vec![addr as u16],
            ));
        }
    }

    Err(format!("Unrecognized effective address operand: '{}'", s))
}
