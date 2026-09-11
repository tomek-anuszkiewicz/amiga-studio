//! Built-in zero-dependency Motorola 68000 opcode disassembler
//!
//! Provides single-instruction disassembly with side-effect-free memory reading.

/// Disassembled instruction representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disassembly {
    /// Program Counter address
    pub pc: u32,
    /// Instruction words (up to 5 words on MC68000)
    pub words: [u16; 5],
    pub word_count: usize,
    /// Instruction mnemonic (e.g. "MOVE.W", "ADD.L", "NOP")
    pub mnemonic: &'static str,
    /// Operands string (e.g. "D0, D1", "#$0042, (A0)")
    pub operands: String,
}

impl Disassembly {
    /// Formats the disassembly into standard debugger view string (e.g. "00FC0004: 4E71            NOP")
    pub fn format_line(&self) -> String {
        let mut hex = String::with_capacity(16);
        for i in 0..self.word_count {
            hex.push_str(&format!("{:04X} ", self.words[i]));
        }
        if self.operands.is_empty() {
            format!("{:08X}: {:<16} {}", self.pc, hex, self.mnemonic)
        } else {
            format!(
                "{:08X}: {:<16} {:<8} {}",
                self.pc, hex, self.mnemonic, self.operands
            )
        }
    }
}

/// Formats an Effective Address (EA) specified by mode and reg
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

/// Attempts to decode ALU instructions for op groups 8 (OR/DIV), 9 (SUB), B (CMP/EOR), C (AND/MUL), D (ADD).
/// Returns `(mnemonic, operands)` if matched.
pub fn decode_alu_group<F>(op: u16, mut next_word: F) -> Option<(&'static str, String)>
where
    F: FnMut() -> u16,
{
    let op_group = (op >> 12) & 0x0F;
    let reg_d = ((op >> 9) & 0x07) as u8;
    let opmode = ((op >> 6) & 0x07) as u8;
    let ea_mode = ((op >> 3) & 0x07) as u8;
    let ea_reg = (op & 0x07) as u8;

    // Group B: CMP / CMPA / EOR / CMPM
    if op_group == 0xB {
        match opmode {
            3 => {
                let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
                return Some(("CMPA.W", format!("{}, A{}", ea_str, reg_d)));
            }
            7 => {
                let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
                return Some(("CMPA.L", format!("{}, A{}", ea_str, reg_d)));
            }
            4 | 5 | 6 => {
                let (mnem_eor, mnem_cmpm) = match opmode {
                    4 => ("EOR.B", "CMPM.B"),
                    5 => ("EOR.W", "CMPM.W"),
                    _ => ("EOR.L", "CMPM.L"),
                };
                if ea_mode == 1 {
                    return Some((mnem_cmpm, format!("(A{})+, (A{})+", ea_reg, reg_d)));
                } else {
                    let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
                    return Some((mnem_eor, format!("D{}, {}", reg_d, ea_str)));
                }
            }
            0 | 1 | 2 => {
                let mnem = match opmode {
                    0 => "CMP.B",
                    1 => "CMP.W",
                    _ => "CMP.L",
                };
                let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
                return Some((mnem, format!("{}, D{}", ea_str, reg_d)));
            }
            _ => {}
        }
    }

    // Group 8: OR / DIVU / DIVS / SBCD
    if op_group == 0x8 {
        if opmode == 3 {
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return Some(("DIVU.W", format!("{}, D{}", ea_str, reg_d)));
        }
        if opmode == 7 {
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return Some(("DIVS.W", format!("{}, D{}", ea_str, reg_d)));
        }
        if opmode == 4 {
            if ea_mode == 0 {
                return Some(("SBCD", format!("D{}, D{}", ea_reg, reg_d)));
            } else if ea_mode == 1 {
                return Some(("SBCD", format!("-(A{}), -(A{})", ea_reg, reg_d)));
            }
        }
    }

    // Group C: AND / MULU / MULS / ABCD / EXG
    if op_group == 0xC {
        if opmode == 3 {
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return Some(("MULU.W", format!("{}, D{}", ea_str, reg_d)));
        }
        if opmode == 7 {
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return Some(("MULS.W", format!("{}, D{}", ea_str, reg_d)));
        }
        if opmode == 4 {
            if ea_mode == 0 {
                return Some(("ABCD", format!("D{}, D{}", ea_reg, reg_d)));
            } else if ea_mode == 1 {
                return Some(("ABCD", format!("-(A{}), -(A{})", ea_reg, reg_d)));
            }
        }
        if opmode == 5 && ea_mode == 0 {
            return Some(("EXG", format!("D{}, D{}", reg_d, ea_reg)));
        }
        if opmode == 5 && ea_mode == 1 {
            return Some(("EXG", format!("A{}, A{}", reg_d, ea_reg)));
        }
        if opmode == 6 && ea_mode == 1 {
            return Some(("EXG", format!("D{}, A{}", reg_d, ea_reg)));
        }
    }

    // Group D / 9: ADD / SUB / ADDA / SUBA / ADDX / SUBX
    if op_group == 0xD || op_group == 0x9 {
        let is_sub = op_group == 0x9;
        if opmode == 3 {
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            let mnem = if is_sub { "SUBA.W" } else { "ADDA.W" };
            return Some((mnem, format!("{}, A{}", ea_str, reg_d)));
        }
        if opmode == 7 {
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            let mnem = if is_sub { "SUBA.L" } else { "ADDA.L" };
            return Some((mnem, format!("{}, A{}", ea_str, reg_d)));
        }
        if opmode == 4 || opmode == 5 || opmode == 6 {
            if ea_mode == 0 || ea_mode == 1 {
                let sz_str = match opmode {
                    4 => ".B",
                    5 => ".W",
                    _ => ".L",
                };
                let mnem = match (is_sub, sz_str) {
                    (false, ".B") => "ADDX.B",
                    (false, ".W") => "ADDX.W",
                    (false, ".L") => "ADDX.L",
                    (true, ".B") => "SUBX.B",
                    (true, ".W") => "SUBX.W",
                    _ => "SUBX.L",
                };
                let ops = if ea_mode == 0 {
                    format!("D{}, D{}", ea_reg, reg_d)
                } else {
                    format!("-(A{}), -(A{})", ea_reg, reg_d)
                };
                return Some((mnem, ops));
            }
        }
    }

    // Standard ADD / SUB / AND / OR (<ea>, Dn or Dn, <ea>)
    if op_group == 0xD || op_group == 0x9 || op_group == 0xC || op_group == 0x8 {
        let base_mnem = match op_group {
            0xD => "ADD",
            0x9 => "SUB",
            0xC => "AND",
            0x8 => "OR",
            _ => unreachable!(),
        };

        let (sz_str, ea_is_source) = match opmode {
            0 => (".B", true),
            1 => (".W", true),
            2 => (".L", true),
            4 => (".B", false),
            5 => (".W", false),
            6 => (".L", false),
            _ => (".W", true),
        };

        let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
        let ops = if ea_is_source {
            format!("{}, D{}", ea_str, reg_d)
        } else {
            format!("D{}, {}", reg_d, ea_str)
        };

        let full_mnem = match base_mnem {
            "ADD" => match sz_str {
                ".B" => "ADD.B",
                ".W" => "ADD.W",
                _ => "ADD.L",
            },
            "SUB" => match sz_str {
                ".B" => "SUB.B",
                ".W" => "SUB.W",
                _ => "SUB.L",
            },
            "AND" => match sz_str {
                ".B" => "AND.B",
                ".W" => "AND.W",
                _ => "AND.L",
            },
            "OR" => match sz_str {
                ".B" => "OR.B",
                ".W" => "OR.W",
                _ => "OR.L",
            },
            _ => "OP",
        };

        return Some((full_mnem, ops));
    }

    None
}

/// Disassembles one instruction starting at `pc` using a side-effect-free word reader function
pub fn disassemble(pc: u32, read_word: impl Fn(u32) -> u16) -> (Disassembly, u32) {
    let mut words = [0u16; 5];
    let mut offset = 0u32;

    let op = read_word(pc);
    words[0] = op;
    let mut word_count = 1;
    offset += 2;

    let mut next_word = || {
        let w = read_word(pc.wrapping_add(offset));
        if word_count < 5 {
            words[word_count] = w;
            word_count += 1;
        }
        offset += 2;
        w
    };

    // 1. NOP / RTS
    if op == 0x4E71 {
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "NOP",
                operands: String::new(),
            },
            offset,
        );
    }
    if op == 0x4E75 {
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "RTS",
                operands: String::new(),
            },
            offset,
        );
    }

    // 2. TRAP
    if (op & 0xFFF0) == 0x4E40 {
        let vec = op & 0x0F;
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "TRAP",
                operands: format!("#{}", vec),
            },
            offset,
        );
    }

    // 3. BRA / Bcc
    if (op & 0xF000) == 0x6000 {
        let cond = ((op >> 8) & 0x0F) as u8;
        let d8 = (op & 0xFF) as i8;
        let disp = if d8 == 0 {
            next_word() as i16 as i32
        } else {
            d8 as i32
        };
        let target = (pc.wrapping_add(2)).wrapping_add(disp as u32) & 0x00FF_FFFF;
        let mnem = match cond {
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
            _ => unreachable!(),
        };
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: mnem,
                operands: format!("${:06X}", target),
            },
            offset,
        );
    }

    // 4. JMP / JSR / Miscellaneous 4Exx
    if (op & 0xFFC0) == 0x4EC0 || (op & 0xFFC0) == 0x4E80 {
        let is_jsr = (op & 0xFFC0) == 0x4E80;
        let mnem = if is_jsr { "JSR" } else { "JMP" };
        let mode = ((op >> 3) & 0x07) as u8;
        let reg = (op & 0x07) as u8;
        let ea_str = format_ea(mode, reg, &mut next_word);
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: mnem,
                operands: ea_str,
            },
            offset,
        );
    }
    if op == 0x4E70 {
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "RESET",
                operands: String::new(),
            },
            offset,
        );
    }
    if op == 0x4E72 {
        let imm = next_word();
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "STOP",
                operands: format!("#${:04X}", imm),
            },
            offset,
        );
    }
    if op == 0x4E73 {
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "RTE",
                operands: String::new(),
            },
            offset,
        );
    }
    if op == 0x4E76 {
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "TRAPV",
                operands: String::new(),
            },
            offset,
        );
    }
    if op == 0x4E77 {
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "RTR",
                operands: String::new(),
            },
            offset,
        );
    }
    if (op & 0xFFF8) == 0x4E50 {
        let reg = (op & 0x07) as u8;
        let disp = next_word() as i16;
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "LINK",
                operands: format!("A{}, #{}", reg, disp),
            },
            offset,
        );
    }
    if (op & 0xFFF8) == 0x4E58 {
        let reg = (op & 0x07) as u8;
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "UNLK",
                operands: format!("A{}", reg),
            },
            offset,
        );
    }
    if (op & 0xFFF8) == 0x4E60 {
        let reg = (op & 0x07) as u8;
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "MOVE",
                operands: format!("USP, A{}", reg),
            },
            offset,
        );
    }
    if (op & 0xFFF8) == 0x4E68 {
        let reg = (op & 0x07) as u8;
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "MOVE",
                operands: format!("A{}, USP", reg),
            },
            offset,
        );
    }

    // 5. MOVE / MOVEA
    let top2 = (op >> 14) & 0x03;
    if top2 == 0 {
        let size_bits = (op >> 12) & 0x03;
        if size_bits != 0 {
            let mnem = match size_bits {
                1 => "MOVE.B",
                3 => "MOVE.W",
                2 => "MOVE.L",
                _ => unreachable!(),
            };
            let dst_reg = ((op >> 9) & 0x07) as u8;
            let dst_mode = ((op >> 6) & 0x07) as u8;
            let src_mode = ((op >> 3) & 0x07) as u8;
            let src_reg = (op & 0x07) as u8;

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
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: mnem,
                    operands: format!("{}, {}", src_str, dst_str),
                },
                offset,
            );
        }
    }

    let op_group = (op >> 12) & 0x0F;

    // 6. Group 0: Immediate ALU & Bit Manipulation
    if op_group == 0 {
        let is_dynamic_bit = (op & 0x0100) != 0;
        let is_static_bit = (op & 0x0F00) == 0x0800;
        if is_dynamic_bit || is_static_bit {
            let opmode = (op >> 6) & 0x03;
            let mnem = match opmode {
                0 => "BTST",
                1 => "BCHG",
                2 => "BCLR",
                3 => "BSET",
                _ => unreachable!(),
            };
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let (src_str, ea_str) = if is_dynamic_bit {
                let reg_d = ((op >> 9) & 0x07) as u8;
                (
                    format!("D{}", reg_d),
                    format_ea(ea_mode, ea_reg, &mut next_word),
                )
            } else {
                let bit_num = next_word() & 0xFF;
                (
                    format!("#{}", bit_num),
                    format_ea(ea_mode, ea_reg, &mut next_word),
                )
            };
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: mnem,
                    operands: format!("{}, {}", src_str, ea_str),
                },
                offset,
            );
        }

        // Immediate arithmetic / logic (ORI, ANDI, SUBI, ADDI, EORI, CMPI)
        let imm_type = (op >> 9) & 0x07;
        let base_mnem = match imm_type {
            0 => "ORI",
            1 => "ANDI",
            2 => "SUBI",
            3 => "ADDI",
            5 => "EORI",
            6 => "CMPI",
            _ => "",
        };
        if !base_mnem.is_empty() {
            let sz_bits = (op >> 6) & 0x03;
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            if ea_mode == 7 && ea_reg == 4 {
                if sz_bits == 0 {
                    let imm = next_word() & 0xFF;
                    let mnem = match base_mnem {
                        "ORI" => "ORI.B",
                        "ANDI" => "ANDI.B",
                        "EORI" => "EORI.B",
                        _ => base_mnem,
                    };
                    return (
                        Disassembly {
                            pc,
                            words,
                            word_count,
                            mnemonic: mnem,
                            operands: format!("#${:02X}, CCR", imm),
                        },
                        offset,
                    );
                } else if sz_bits == 1 {
                    let imm = next_word();
                    let mnem = match base_mnem {
                        "ORI" => "ORI.W",
                        "ANDI" => "ANDI.W",
                        "EORI" => "EORI.W",
                        _ => base_mnem,
                    };
                    return (
                        Disassembly {
                            pc,
                            words,
                            word_count,
                            mnemonic: mnem,
                            operands: format!("#${:04X}, SR", imm),
                        },
                        offset,
                    );
                }
            }
            let (sz_str, imm_str) = match sz_bits {
                0 => (".B", format!("#${:02X}", next_word() & 0xFF)),
                1 => (".W", format!("#${:04X}", next_word())),
                2 => {
                    let hi = next_word() as u32;
                    let lo = next_word() as u32;
                    (".L", format!("#${:08X}", (hi << 16) | lo))
                }
                _ => (".W", format!("#${:04X}", next_word())),
            };
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
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
                _ => "OP",
            };
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: full_mnem,
                    operands: format!("{}, {}", imm_str, ea_str),
                },
                offset,
            );
        }
    }

    // 7. Group 4: Miscellaneous (CLR, NEG, NEGX, NOT, EXT, SWAP, TST, TAS, LEA, CHK)
    if op_group == 4 {
        if (op & 0xFFF8) == 0x4840 {
            let reg = (op & 0x07) as u8;
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "SWAP",
                    operands: format!("D{}", reg),
                },
                offset,
            );
        }
        if (op & 0xFFF8) == 0x4880 {
            let reg = (op & 0x07) as u8;
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "EXT.W",
                    operands: format!("D{}", reg),
                },
                offset,
            );
        }
        if (op & 0xFFF8) == 0x48C0 {
            let reg = (op & 0x07) as u8;
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "EXT.L",
                    operands: format!("D{}", reg),
                },
                offset,
            );
        }
        if (op & 0xFFC0) == 0x4AC0 {
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "TAS",
                    operands: ea_str,
                },
                offset,
            );
        }
        if (op & 0xFFC0) == 0x40C0 {
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "MOVE.W",
                    operands: format!("SR, {}", ea_str),
                },
                offset,
            );
        }
        if (op & 0xFFC0) == 0x44C0 {
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "MOVE.W",
                    operands: format!("{}, CCR", ea_str),
                },
                offset,
            );
        }
        if (op & 0xFFC0) == 0x46C0 {
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "MOVE.W",
                    operands: format!("{}, SR", ea_str),
                },
                offset,
            );
        }
        if (op & 0x01C0) == 0x01C0 {
            let reg = ((op >> 9) & 0x07) as u8;
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "LEA",
                    operands: format!("{}, A{}", ea_str, reg),
                },
                offset,
            );
        }
        if (op & 0x01C0) == 0x0180 {
            let reg = ((op >> 9) & 0x07) as u8;
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: "CHK.W",
                    operands: format!("{}, D{}", ea_str, reg),
                },
                offset,
            );
        }
        let kind = (op >> 8) & 0x0F;
        let base_mnem = match kind {
            0 => "NEGX",
            2 => "CLR",
            4 => "NEG",
            6 => "NOT",
            10 => "TST",
            _ => "",
        };
        if !base_mnem.is_empty() {
            let sz_bits = (op >> 6) & 0x03;
            let sz_str = match sz_bits {
                0 => ".B",
                1 => ".W",
                2 => ".L",
                _ => ".W",
            };
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            let full_mnem = match (base_mnem, sz_str) {
                ("NEGX", ".B") => "NEGX.B",
                ("NEGX", ".W") => "NEGX.W",
                ("NEGX", ".L") => "NEGX.L",
                ("CLR", ".B") => "CLR.B",
                ("CLR", ".W") => "CLR.W",
                ("CLR", ".L") => "CLR.L",
                ("NEG", ".B") => "NEG.B",
                ("NEG", ".W") => "NEG.W",
                ("NEG", ".L") => "NEG.L",
                ("NOT", ".B") => "NOT.B",
                ("NOT", ".W") => "NOT.W",
                ("NOT", ".L") => "NOT.L",
                ("TST", ".B") => "TST.B",
                ("TST", ".W") => "TST.W",
                ("TST", ".L") => "TST.L",
                _ => "OP",
            };
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: full_mnem,
                    operands: ea_str,
                },
                offset,
            );
        }
    }

    // 8. Group 5: ADDQ / SUBQ / Scc / DBcc
    if op_group == 5 {
        let data = ((op >> 9) & 0x07) as u8;
        let val = if data == 0 { 8 } else { data };
        let is_sub = (op & 0x0100) != 0;
        let sz_bits = (op >> 6) & 0x03;
        let ea_mode = ((op >> 3) & 0x07) as u8;
        let ea_reg = (op & 0x07) as u8;
        if sz_bits == 3 {
            if ea_mode == 1 {
                let cond = ((op >> 8) & 0x0F) as u8;
                let disp = next_word() as i16;
                let target = (pc.wrapping_add(2)).wrapping_add(disp as u32) & 0x00FF_FFFF;
                let mnem = if cond == 1 { "DBF" } else { "DBcc" };
                return (
                    Disassembly {
                        pc,
                        words,
                        word_count,
                        mnemonic: mnem,
                        operands: format!("D{}, ${:06X}", ea_reg, target),
                    },
                    offset,
                );
            }
        } else {
            let mnem = match (is_sub, sz_bits) {
                (false, 0) => "ADDQ.B",
                (false, 1) => "ADDQ.W",
                (false, 2) => "ADDQ.L",
                (true, 0) => "SUBQ.B",
                (true, 1) => "SUBQ.W",
                (true, 2) => "SUBQ.L",
                _ => "ADDQ",
            };
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: mnem,
                    operands: format!("#{}, {}", val, ea_str),
                },
                offset,
            );
        }
    }

    // 9. Group 7: MOVEQ
    if op_group == 7 && (op & 0x0100) == 0 {
        let reg = ((op >> 9) & 0x07) as u8;
        let imm = (op & 0xFF) as i8;
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: "MOVEQ",
                operands: format!("#{}, D{}", imm, reg),
            },
            offset,
        );
    }

    // 10. ALU & Comparison Groups (8, 9, B, C, D)
    if let Some((mnem, ops)) = decode_alu_group(op, &mut next_word) {
        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: mnem,
                operands: ops,
            },
            offset,
        );
    }

    // 11. Group E: Shifts & Rotates
    if op_group == 0xE {
        let is_mem_shift = (op & 0x00C0) == 0x00C0;
        let shift_type = (op >> 9) & 0x03;
        let is_left = (op & 0x0100) != 0;
        let base_name = match (shift_type, is_left) {
            (0, false) => "ASR",
            (0, true) => "ASL",
            (1, false) => "LSR",
            (1, true) => "LSL",
            (2, false) => "ROXR",
            (2, true) => "ROXL",
            (3, false) => "ROR",
            (3, true) => "ROL",
            _ => unreachable!(),
        };
        if is_mem_shift {
            let ea_mode = ((op >> 3) & 0x07) as u8;
            let ea_reg = (op & 0x07) as u8;
            let ea_str = format_ea(ea_mode, ea_reg, &mut next_word);
            let mnem = match base_name {
                "ASR" => "ASR.W",
                "ASL" => "ASL.W",
                "LSR" => "LSR.W",
                "LSL" => "LSL.W",
                "ROXR" => "ROXR.W",
                "ROXL" => "ROXL.W",
                "ROR" => "ROR.W",
                "ROL" => "ROL.W",
                _ => "SHIFT",
            };
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: mnem,
                    operands: ea_str,
                },
                offset,
            );
        } else {
            let sz_bits = (op >> 6) & 0x03;
            let sz_str = match sz_bits {
                0 => ".B",
                1 => ".W",
                2 => ".L",
                _ => ".W",
            };
            let reg_d = (op & 0x07) as u8;
            let is_reg_count = (op & 0x0020) != 0;
            let count_or_reg = ((op >> 9) & 0x07) as u8;
            let count_str = if is_reg_count {
                format!("D{}", count_or_reg)
            } else {
                let imm = if count_or_reg == 0 { 8 } else { count_or_reg };
                format!("#{}", imm)
            };
            let mnem = match (base_name, sz_str) {
                ("ASL", ".B") => "ASL.B",
                ("ASL", ".W") => "ASL.W",
                ("ASL", ".L") => "ASL.L",
                ("ASR", ".B") => "ASR.B",
                ("ASR", ".W") => "ASR.W",
                ("ASR", ".L") => "ASR.L",
                ("LSL", ".B") => "LSL.B",
                ("LSL", ".W") => "LSL.W",
                ("LSL", ".L") => "LSL.L",
                ("LSR", ".B") => "LSR.B",
                ("LSR", ".W") => "LSR.W",
                ("LSR", ".L") => "LSR.L",
                ("ROL", ".B") => "ROL.B",
                ("ROL", ".W") => "ROL.W",
                ("ROL", ".L") => "ROL.L",
                ("ROR", ".B") => "ROR.B",
                ("ROR", ".W") => "ROR.W",
                ("ROR", ".L") => "ROR.L",
                ("ROXL", ".B") => "ROXL.B",
                ("ROXL", ".W") => "ROXL.W",
                ("ROXL", ".L") => "ROXL.L",
                ("ROXR", ".B") => "ROXR.B",
                ("ROXR", ".W") => "ROXR.W",
                ("ROXR", ".L") => "ROXR.L",
                _ => "SHIFT",
            };
            return (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: mnem,
                    operands: format!("{}, D{}", count_str, reg_d),
                },
                offset,
            );
        }
    }

    // Default raw instruction word
    (
        Disassembly {
            pc,
            words,
            word_count: 1,
            mnemonic: "DATA.W",
            operands: format!("${:04X}", op),
        },
        2,
    )
}
