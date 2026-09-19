//! ALU, Compare, Bitwise, Immediate, Multiply/Divide and Shift/Rotate disassembler module

use crate::ea::{format_ea, format_immediate};

/// Attempts to disassemble arithmetic, logic, compare, bit, multiply/divide, and shift/rotate instructions.
///
/// Returns `Some((mnemonic, operands))` if decoded, consuming extension words via `next_word`.
pub(crate) fn try_disassemble_alu(
    op: u16,
    mut next_word: impl FnMut() -> u16,
) -> Option<(&'static str, String)> {
    // 1. Immediate operations ($0000..$0FFF: ORI, ANDI, SUBI, ADDI, EORI, CMPI, BitOps)
    if (op & 0xF000) == 0x0000 {
        // Bit operations with immediate bit number ($0800..$08FF)
        if (op & 0xFF00) == 0x0800 {
            let bit_type = (op >> 6) & 0x03;
            let mnem = match bit_type {
                0 => "BTST",
                1 => "BCHG",
                2 => "BCLR",
                3 => "BSET",
                _ => "BIT",
            };
            let bit_val = (next_word() & 0xFF) as u8;
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, format!("#{}, {}", bit_val, ea_str)));
        }

        // Bit operations with dynamic bit in Dn ($0100..$01FF)
        if (op & 0xF100) == 0x0100 {
            let dn = ((op >> 9) & 7) as u8;
            let bit_type = (op >> 6) & 0x03;
            let mnem = match bit_type {
                0 => "BTST",
                1 => "BCHG",
                2 => "BCLR",
                3 => "BSET",
                _ => "BIT",
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, format!("D{}, {}", dn, ea_str)));
        }

        // Special immediate to CCR/SR
        match op {
            0x003C => {
                let imm = format_immediate(1, &mut next_word);
                return Some(("ORI.B", format!("{}, CCR", imm)));
            }
            0x007C => {
                let imm = format_immediate(2, &mut next_word);
                return Some(("ORI.W", format!("{}, SR", imm)));
            }
            0x023C => {
                let imm = format_immediate(1, &mut next_word);
                return Some(("ANDI.B", format!("{}, CCR", imm)));
            }
            0x027C => {
                let imm = format_immediate(2, &mut next_word);
                return Some(("ANDI.W", format!("{}, SR", imm)));
            }
            0x0A3C => {
                let imm = format_immediate(1, &mut next_word);
                return Some(("EORI.B", format!("{}, CCR", imm)));
            }
            0x0A7C => {
                let imm = format_immediate(2, &mut next_word);
                return Some(("EORI.W", format!("{}, SR", imm)));
            }
            _ => {}
        }

        let op_type = (op >> 9) & 7;
        let size_bits = (op >> 6) & 3;
        if size_bits < 3 {
            let (sz_bytes, sz_str) = match size_bits {
                0 => (1, ".B"),
                1 => (2, ".W"),
                2 => (4, ".L"),
                _ => (2, ".W"),
            };
            let base_mnem = match op_type {
                0 => "ORI",
                1 => "ANDI",
                2 => "SUBI",
                3 => "ADDI",
                5 => "EORI",
                6 => "CMPI",
                _ => "",
            };
            if !base_mnem.is_empty() {
                let full_mnem = match (base_mnem, sz_str) {
                    ("ORI", ".B") => "ORI.B",
                    ("ORI", ".W") => "ORI.W",
                    ("ORI", ".L") => "ORI.L",
                    ("ANDI", ".B") => "ANDI.B",
                    ("ANDI", ".W") => "ANDI.W",
                    ("ANDI", ".L") => "ANDI.L",
                    ("SUBI", ".B") => "SUBI.B",
                    ("SUBI", ".W") => "SUBI.W",
                    ("SUBI", ".L") => "SUBI.L",
                    ("ADDI", ".B") => "ADDI.B",
                    ("ADDI", ".W") => "ADDI.W",
                    ("ADDI", ".L") => "ADDI.L",
                    ("EORI", ".B") => "EORI.B",
                    ("EORI", ".W") => "EORI.W",
                    ("EORI", ".L") => "EORI.L",
                    ("CMPI", ".B") => "CMPI.B",
                    ("CMPI", ".W") => "CMPI.W",
                    ("CMPI", ".L") => "CMPI.L",
                    _ => base_mnem,
                };
                let imm = format_immediate(sz_bytes, &mut next_word);
                let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
                return Some((full_mnem, format!("{}, {}", imm, ea_str)));
            }
        }
    }

    // 2. Compare, EOR, CMPA, CMPM ($B000..$BFFF)
    if (op & 0xF000) == 0xB000 {
        let reg_d = ((op >> 9) & 7) as u8;
        let opmode = ((op >> 6) & 7) as u8;

        // CMPA.W / CMPA.L
        if opmode == 3 || opmode == 7 {
            let mnem = if opmode == 3 { "CMPA.W" } else { "CMPA.L" };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, format!("{}, A{}", ea_str, reg_d)));
        }

        // CMPM (Ay)+, (Ax)+
        if (op & 0xF138) == 0xB108 {
            let mnem = match opmode & 3 {
                0 => "CMPM.B",
                1 => "CMPM.W",
                2 => "CMPM.L",
                _ => "CMPM",
            };
            let ax = reg_d;
            let ay = (op & 7) as u8;
            return Some((mnem, format!("(A{})+, (A{})+", ay, ax)));
        }

        // EOR Dn, <ea>
        if opmode == 4 || opmode == 5 || opmode == 6 {
            let mnem = match opmode {
                4 => "EOR.B",
                5 => "EOR.W",
                6 => "EOR.L",
                _ => "EOR",
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, format!("D{}, {}", reg_d, ea_str)));
        }

        // CMP <ea>, Dn
        if opmode <= 2 {
            let mnem = match opmode {
                0 => "CMP.B",
                1 => "CMP.W",
                2 => "CMP.L",
                _ => "CMP",
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, format!("{}, D{}", ea_str, reg_d)));
        }
    }

    // 3. Multiply, Divide & EXG ($C000..$CFFF, $8000..$8FFF)
    if (op & 0xF000) == 0xC000 || (op & 0xF000) == 0x8000 {
        let is_c = (op & 0xF000) == 0xC000;
        let reg = ((op >> 9) & 7) as u8;

        // EXG
        if is_c && (op & 0xF130) == 0xC100 {
            let mode = (op >> 3) & 0x1F;
            let ry = (op & 7) as u8;
            let rx = reg;
            match mode {
                0x08 => return Some(("EXG", format!("D{}, D{}", rx, ry))),
                0x09 => return Some(("EXG", format!("A{}, A{}", rx, ry))),
                0x11 => return Some(("EXG", format!("D{}, A{}", rx, ry))),
                _ => {}
            }
        }

        // MULU / MULS / DIVU / DIVS
        if (op & 0xF1C0) == 0xC0C0 {
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some(("MULU.W", format!("{}, D{}", ea_str, reg)));
        }
        if (op & 0xF1C0) == 0xC1C0 {
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some(("MULS.W", format!("{}, D{}", ea_str, reg)));
        }
        if (op & 0xF1C0) == 0x80C0 {
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some(("DIVU.W", format!("{}, D{}", ea_str, reg)));
        }
        if (op & 0xF1C0) == 0x81C0 {
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some(("DIVS.W", format!("{}, D{}", ea_str, reg)));
        }
    }

    // ABCD & SBCD ($C100..$C10F, $8100..$810F)
    if (op & 0xF1F0) == 0xC100 || (op & 0xF1F0) == 0x8100 {
        let is_abcd = (op & 0xF000) == 0xC000;
        let mnem = if is_abcd { "ABCD" } else { "SBCD" };
        let rx = ((op >> 9) & 7) as u8;
        let ry = (op & 7) as u8;
        let is_mem = (op & 0x0008) != 0;
        let ops = if is_mem {
            format!("-(A{}), -(A{})", ry, rx)
        } else {
            format!("D{}, D{}", ry, rx)
        };
        return Some((mnem, ops));
    }

    // 4. ADD, ADDA, ADDX, SUB, SUBA, SUBX, AND, OR
    let op_group = (op >> 12) & 0x0F;
    if op_group == 0xD || op_group == 0x9 || op_group == 0xC || op_group == 0x8 {
        let base_name = match op_group {
            0xD => "ADD",
            0x9 => "SUB",
            0xC => "AND",
            _ => "OR",
        };
        let reg_d = ((op >> 9) & 7) as u8;
        let opmode = ((op >> 6) & 7) as u8;

        // ADDA / SUBA
        if (op_group == 0xD || op_group == 0x9) && (opmode == 3 || opmode == 7) {
            let mnem = match (op_group, opmode) {
                (0xD, 3) => "ADDA.W",
                (0xD, 7) => "ADDA.L",
                (0x9, 3) => "SUBA.W",
                (0x9, 7) => "SUBA.L",
                _ => "ADDA",
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, format!("{}, A{}", ea_str, reg_d)));
        }

        // ADDX / SUBX
        if (op_group == 0xD || op_group == 0x9) && (op & 0x0130) == 0x0100 {
            let sz = match opmode & 3 {
                0 => ".B",
                1 => ".W",
                2 => ".L",
                _ => "",
            };
            let is_mem = (op & 0x0008) != 0;
            let rx = reg_d;
            let ry = (op & 7) as u8;
            let mnem = if op_group == 0xD {
                match sz {
                    ".B" => "ADDX.B",
                    ".W" => "ADDX.W",
                    ".L" => "ADDX.L",
                    _ => "ADDX",
                }
            } else {
                match sz {
                    ".B" => "SUBX.B",
                    ".W" => "SUBX.W",
                    ".L" => "SUBX.L",
                    _ => "SUBX",
                }
            };
            let ops = if is_mem {
                format!("-(A{}), -(A{})", ry, rx)
            } else {
                format!("D{}, D{}", ry, rx)
            };
            return Some((mnem, ops));
        }

        let (sz_str, ea_is_source) = match opmode {
            0 => (".B", true),
            1 => (".W", true),
            2 => (".L", true),
            4 => (".B", false),
            5 => (".W", false),
            6 => (".L", false),
            _ => (".W", true),
        };

        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        let ops = if ea_is_source {
            format!("{}, D{}", ea_str, reg_d)
        } else {
            format!("D{}, {}", reg_d, ea_str)
        };

        let full_mnem = match (base_name, sz_str) {
            ("ADD", ".B") => "ADD.B",
            ("ADD", ".W") => "ADD.W",
            ("ADD", ".L") => "ADD.L",
            ("SUB", ".B") => "SUB.B",
            ("SUB", ".W") => "SUB.W",
            ("SUB", ".L") => "SUB.L",
            ("AND", ".B") => "AND.B",
            ("AND", ".W") => "AND.W",
            ("AND", ".L") => "AND.L",
            ("OR", ".B") => "OR.B",
            ("OR", ".W") => "OR.W",
            ("OR", ".L") => "OR.L",
            _ => "OP",
        };

        return Some((full_mnem, ops));
    }

    // 5. Shifts & Rotates ($E000..$EFFF)
    if (op & 0xF000) == 0xE000 {
        // Memory shifts: op & 0xF8C0 == 0xE0C0
        if (op & 0xF8C0) == 0xE0C0 {
            let dir_left = (op & 0x0100) != 0;
            let shift_type = (op >> 9) & 3;
            let mnem = match (shift_type, dir_left) {
                (0, false) => "ASR.W",
                (0, true) => "ASL.W",
                (1, false) => "LSR.W",
                (1, true) => "LSL.W",
                (2, false) => "ROXR.W",
                (2, true) => "ROXL.W",
                (3, false) => "ROR.W",
                (3, true) => "ROL.W",
                _ => "SHIFT",
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return Some((mnem, ea_str));
        }

        // Register shifts
        let size_bits = (op >> 6) & 3;
        if size_bits < 3 {
            let dir_left = (op & 0x0100) != 0;
            let shift_type = (op >> 3) & 3;
            let is_reg = (op & 0x0020) != 0;
            let count_raw = ((op >> 9) & 7) as u8;
            let target_reg = (op & 7) as u8;

            let sz = match size_bits {
                0 => ".B",
                1 => ".W",
                2 => ".L",
                _ => "",
            };

            let mnem = match (shift_type, dir_left, sz) {
                (0, false, ".B") => "ASR.B",
                (0, false, ".W") => "ASR.W",
                (0, false, ".L") => "ASR.L",
                (0, true, ".B") => "ASL.B",
                (0, true, ".W") => "ASL.W",
                (0, true, ".L") => "ASL.L",
                (1, false, ".B") => "LSR.B",
                (1, false, ".W") => "LSR.W",
                (1, false, ".L") => "LSR.L",
                (1, true, ".B") => "LSL.B",
                (1, true, ".W") => "LSL.W",
                (1, true, ".L") => "LSL.L",
                (2, false, ".B") => "ROXR.B",
                (2, false, ".W") => "ROXR.W",
                (2, false, ".L") => "ROXR.L",
                (2, true, ".B") => "ROXL.B",
                (2, true, ".W") => "ROXL.W",
                (2, true, ".L") => "ROXL.L",
                (3, false, ".B") => "ROR.B",
                (3, false, ".W") => "ROR.W",
                (3, false, ".L") => "ROR.L",
                (3, true, ".B") => "ROL.B",
                (3, true, ".W") => "ROL.W",
                (3, true, ".L") => "ROL.L",
                _ => "SHIFT",
            };

            let count_str = if is_reg {
                format!("D{}", count_raw)
            } else {
                let cnt = if count_raw == 0 { 8 } else { count_raw };
                format!("#{}", cnt)
            };

            return Some((mnem, format!("{}, D{}", count_str, target_reg)));
        }
    }

    None
}
