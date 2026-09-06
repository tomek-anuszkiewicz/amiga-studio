//! Built-in zero-dependency Motorola 68000 opcode disassembler

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

    // 4. JMP / JSR
    if (op & 0xFFC0) == 0x4EC0 || (op & 0xFFC0) == 0x4E80 {
        let is_jsr = (op & 0xFFC0) == 0x4E80;
        let mnem = if is_jsr { "JSR" } else { "JMP" };
        let mode = ((op >> 3) & 0x07) as u8;
        let reg = (op & 0x07) as u8;
        let ea_str = format_ea(mode, reg, &mut next_word, pc.wrapping_add(offset));
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

    // 5. MOVE / MOVEA
    let top2 = (op >> 14) & 0x03;
    if top2 == 0 {
        let size_bits = (op >> 12) & 0x03;
        if size_bits != 0 {
            let (sz_str, mnem) = match size_bits {
                1 => (".B", "MOVE.B"),
                3 => (".W", "MOVE.W"),
                2 => (".L", "MOVE.L"),
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

            let src_str = format_ea(src_mode, src_reg, &mut next_word, pc.wrapping_add(offset));
            let dst_str = format_ea(dst_mode, dst_reg, &mut next_word, pc.wrapping_add(offset));
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

    // 6. ADD / SUB / AND / OR
    let op_group = (op >> 12) & 0x0F;
    if op_group == 0xD || op_group == 0x9 || op_group == 0xC || op_group == 0x8 {
        let base_mnem = match op_group {
            0xD => "ADD",
            0x9 => "SUB",
            0xC => "AND",
            0x8 => "OR",
            _ => unreachable!(),
        };
        let reg_d = ((op >> 9) & 0x07) as u8;
        let opmode = ((op >> 6) & 0x07) as u8;
        let ea_mode = ((op >> 3) & 0x07) as u8;
        let ea_reg = (op & 0x07) as u8;

        let (sz_str, ea_is_source) = match opmode {
            0 => (".B", true),
            1 => (".W", true),
            2 => (".L", true),
            4 => (".B", false),
            5 => (".W", false),
            6 => (".L", false),
            _ => (".W", true),
        };

        let ea_str = format_ea(ea_mode, ea_reg, &mut next_word, pc.wrapping_add(offset));
        let ops = if ea_is_source {
            format!("{}, D{}", ea_str, reg_d)
        } else {
            format!("D{}, {}", reg_d, ea_str)
        };

        let full_mnem = match (base_mnem, sz_str) {
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

        return (
            Disassembly {
                pc,
                words,
                word_count,
                mnemonic: full_mnem,
                operands: ops,
            },
            offset,
        );
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

fn format_ea(mode: u8, reg: u8, mut read_ext: impl FnMut() -> u16, _current_pc: u32) -> String {
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
