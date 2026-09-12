//! Built-in zero-dependency Motorola 68000 opcode disassembler

use crate::ea_format::*;

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

    // Helper macro replacement: closure to build Disassembly
    let build =
        |words: [u16; 5], word_count: usize, offset: u32, mnem: &'static str, ops: String| {
            (
                Disassembly {
                    pc,
                    words,
                    word_count,
                    mnemonic: mnem,
                    operands: ops,
                },
                offset,
            )
        };

    // 1. Inherent & Control Instructions ($4E70..$4E77, $4AFC)
    match op {
        0x4E70 => return build(words, word_count, offset, "RESET", String::new()),
        0x4E71 => return build(words, word_count, offset, "NOP", String::new()),
        0x4E72 => {
            let imm = next_word();
            return build(words, word_count, offset, "STOP", format!("#${:04X}", imm));
        }
        0x4E73 => return build(words, word_count, offset, "RTE", String::new()),
        0x4E75 => return build(words, word_count, offset, "RTS", String::new()),
        0x4E76 => return build(words, word_count, offset, "TRAPV", String::new()),
        0x4E77 => return build(words, word_count, offset, "RTR", String::new()),
        0x4AFC => return build(words, word_count, offset, "ILLEGAL", String::new()),
        _ => {}
    }

    // 2. TRAP #vector
    if (op & 0xFFF0) == 0x4E40 {
        let vec = op & 0x0F;
        return build(words, word_count, offset, "TRAP", format!("#{}", vec));
    }

    // 3. LINK / UNLK
    if (op & 0xFFF8) == 0x4E50 {
        let an = (op & 0x07) as u8;
        let d16 = next_word() as i16;
        return build(
            words,
            word_count,
            offset,
            "LINK",
            format!("A{}, #${:04X}", an, d16),
        );
    }
    if (op & 0xFFF8) == 0x4E58 {
        let an = (op & 0x07) as u8;
        return build(words, word_count, offset, "UNLK", format!("A{}", an));
    }

    // 4. MOVE to/from USP
    if (op & 0xFFF8) == 0x4E60 {
        let an = (op & 0x07) as u8;
        return build(words, word_count, offset, "MOVE", format!("A{}, USP", an));
    }
    if (op & 0xFFF8) == 0x4E68 {
        let an = (op & 0x07) as u8;
        return build(words, word_count, offset, "MOVE", format!("USP, A{}", an));
    }

    // 5. JMP / JSR
    if (op & 0xFFC0) == 0x4EC0 || (op & 0xFFC0) == 0x4E80 {
        let is_jsr = (op & 0xFFC0) == 0x4E80;
        let mnem = if is_jsr { "JSR" } else { "JMP" };
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return build(words, word_count, offset, mnem, ea_str);
    }

    // 6. BRA / BSR / Bcc
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
        return build(words, word_count, offset, mnem, format!("${:06X}", target));
    }

    // 7. DBcc / DBRA
    if (op & 0xF0F8) == 0x50C8 {
        let cond = ((op >> 8) & 0x0F) as u8;
        let dn = (op & 0x07) as u8;
        let d16 = next_word() as i16;
        let target_pc = pc.wrapping_add(2).wrapping_add(d16 as u32) & 0x00FF_FFFF;
        let mnem = dbcc_condition_name(cond);
        return build(
            words,
            word_count,
            offset,
            mnem,
            format!("D{}, ${:06X}", dn, target_pc),
        );
    }

    // 8. Scc <ea> ($50C0..$5FC0, where size bits 7..6 == 3)
    if (op & 0xF0C0) == 0x50C0 {
        let cond = ((op >> 8) & 0x0F) as u8;
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        let mnem = scc_condition_name(cond);
        return build(words, word_count, offset, mnem, ea_str);
    }

    // 9. ADDQ / SUBQ
    if (op & 0xF000) == 0x5000 {
        let size_bits = (op >> 6) & 0x03;
        if size_bits < 3 {
            let is_subq = (op & 0x0100) != 0;
            let raw = ((op >> 9) & 7) as u8;
            let data = if raw == 0 { 8 } else { raw };
            let sz = match size_bits {
                0 => ".B",
                1 => ".W",
                2 => ".L",
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
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return build(
                words,
                word_count,
                offset,
                mnem,
                format!("#{}, {}", data, ea_str),
            );
        }
    }

    // 10. MOVE / MOVEA
    let top2 = (op >> 14) & 0x03;
    if top2 == 0 {
        let size_bits = (op >> 12) & 0x03;
        if size_bits != 0 {
            let mnem = match size_bits {
                1 => "MOVE.B",
                3 => "MOVE.W",
                2 => "MOVE.L",
                _ => "MOVE",
            };
            let dst_reg = ((op >> 9) & 7) as u8;
            let dst_mode = ((op >> 6) & 7) as u8;
            let src_mode = ((op >> 3) & 7) as u8;
            let src_reg = (op & 7) as u8;
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
            return build(
                words,
                word_count,
                offset,
                mnem,
                format!("{}, {}", src_str, dst_str),
            );
        }
    }

    // 11. MOVEQ
    if (op & 0xF100) == 0x7000 {
        let reg = ((op >> 9) & 7) as u8;
        let data = (op & 0xFF) as u8;
        return build(
            words,
            word_count,
            offset,
            "MOVEQ",
            format!("#${:02X}, D{}", data, reg),
        );
    }

    // 12. MOVEM ($4880..$48BF, $48C0..$48FF, $4C80..$4CBF, $4CC0..$4CFF)
    let movem_ea_mode = ((op >> 3) & 7) as u8;
    if (op & 0xFB80) == 0x4880 && movem_ea_mode >= 2 {
        let is_load = (op & 0x0400) != 0;
        let sz = if (op & 0x0040) != 0 {
            "MOVEM.L"
        } else {
            "MOVEM.W"
        };
        let mask = next_word();
        let ea_reg = (op & 7) as u8;
        let ea_str = format_ea(movem_ea_mode, ea_reg, &mut next_word);
        let reg_list = format_movem_reg_list(mask, movem_ea_mode == 4);
        let ops = if is_load {
            format!("{}, {}", ea_str, reg_list)
        } else {
            format!("{}, {}", reg_list, ea_str)
        };
        return build(words, word_count, offset, sz, ops);
    }

    // 13. LEA & CHK
    if (op & 0xF1C0) == 0x41C0 {
        let an = ((op >> 9) & 7) as u8;
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return build(
            words,
            word_count,
            offset,
            "LEA",
            format!("{}, A{}", ea_str, an),
        );
    }
    if (op & 0xF1C0) == 0x4180 {
        let dn = ((op >> 9) & 7) as u8;
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return build(
            words,
            word_count,
            offset,
            "CHK.W",
            format!("{}, D{}", ea_str, dn),
        );
    }

    // 14. PEA, SWAP, EXT
    if (op & 0xFFF8) == 0x4840 {
        let dn = (op & 7) as u8;
        return build(words, word_count, offset, "SWAP", format!("D{}", dn));
    }
    if (op & 0xFFC0) == 0x4840 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return build(words, word_count, offset, "PEA", ea_str);
    }
    if (op & 0xFFF8) == 0x4880 {
        let dn = (op & 7) as u8;
        return build(words, word_count, offset, "EXT.W", format!("D{}", dn));
    }
    if (op & 0xFFF8) == 0x48C0 {
        let dn = (op & 7) as u8;
        return build(words, word_count, offset, "EXT.L", format!("D{}", dn));
    }

    // 15. SR & CCR transfers
    if (op & 0xFFC0) == 0x40C0 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return build(words, word_count, offset, "MOVE", format!("SR, {}", ea_str));
    }
    if (op & 0xFFC0) == 0x44C0 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return build(
            words,
            word_count,
            offset,
            "MOVE",
            format!("{}, CCR", ea_str),
        );
    }
    if (op & 0xFFC0) == 0x46C0 {
        let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
        return build(words, word_count, offset, "MOVE", format!("{}, SR", ea_str));
    }

    // 16. CLR, NEG, NEGX, NOT, TST
    let group_4 = (op >> 8) & 0xFF;
    if group_4 == 0x40 || group_4 == 0x42 || group_4 == 0x44 || group_4 == 0x46 || group_4 == 0x4A {
        let size_bits = (op >> 6) & 0x03;
        if size_bits < 3 {
            let mnem = match (group_4, size_bits) {
                (0x40, 0) => "NEGX.B",
                (0x40, 1) => "NEGX.W",
                (0x40, 2) => "NEGX.L",
                (0x42, 0) => "CLR.B",
                (0x42, 1) => "CLR.W",
                (0x42, 2) => "CLR.L",
                (0x44, 0) => "NEG.B",
                (0x44, 1) => "NEG.W",
                (0x44, 2) => "NEG.L",
                (0x46, 0) => "NOT.B",
                (0x46, 1) => "NOT.W",
                (0x46, 2) => "NOT.L",
                (0x4A, 0) => "TST.B",
                (0x4A, 1) => "TST.W",
                (0x4A, 2) => "TST.L",
                _ => "OP",
            };
            let ea_str = format_ea(((op >> 3) & 7) as u8, (op & 7) as u8, &mut next_word);
            return build(words, word_count, offset, mnem, ea_str);
        }
    }

    // 17. Arithmetic, Logic, Compare, Multiply/Divide, Shifts & Rotates (disassembler_alu module)
    if let Some((mnem, ops)) = crate::disassembler_alu::try_disassemble_alu(op, &mut next_word) {
        return build(words, word_count, offset, mnem, ops);
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

/// Heuristic anchor finder for backward disassembly alignment.
///
/// In variable-length CISC architectures like Motorola 68000 (instruction lengths 2, 4, 6, 8, 10 bytes),
/// disassembling backwards by an arbitrary byte offset frequently lands inside instruction extension words
/// or uninitialized padding, producing phantom instructions (e.g. `00 00` -> `ORI.B #$00, D0`) and
/// desynchronizing the instruction boundary at `target_pc`.
///
/// This function finds an optimal start address `start_pc <= target_pc` such that:
/// 1. Disassembling forward from `start_pc` lands EXACTLY on `target_pc`.
/// 2. Prioritizes known executed instruction boundaries from temporal/trace history.
/// 3. Scores candidate start addresses by maximizing valid code and minimizing non-code
///    (penalizing invalid/unknown opcodes `DATA.W`, excessive `ORI.B #...` from zero memory, etc.).
/// 4. Targets displaying approximately `desired_prior_instructions` (e.g. 2 to 3) before `target_pc`.
pub fn find_aligned_disassembly_start<F>(
    target_pc: u32,
    desired_prior_instructions: usize,
    read_word: F,
    known_boundaries: &[u32],
) -> u32
where
    F: Fn(u32) -> u16,
{
    let target_pc = target_pc & 0x00FF_FFFE;
    if desired_prior_instructions == 0 || target_pc == 0 {
        return target_pc;
    }

    // Step 1: Check known boundaries from execution history (temporal/trace).
    // If we find an anchor in history that cleanly sweeps forward to `target_pc`,
    // that is verified ground truth from hardware execution.
    let mut best_history_anchor: Option<u32> = None;
    for &boundary in known_boundaries.iter().rev() {
        let b = boundary & 0x00FF_FFFE;
        if b < target_pc && target_pc.saturating_sub(b) <= 32 {
            let mut curr = b;
            let mut count = 0;
            let mut matched = false;
            while curr < target_pc && count <= desired_prior_instructions + 2 {
                let (_, byte_len) = disassemble(curr, &read_word);
                curr = curr.wrapping_add(byte_len);
                count += 1;
                if curr == target_pc {
                    matched = true;
                    break;
                }
            }
            if matched {
                best_history_anchor = Some(b);
                if count >= desired_prior_instructions {
                    break;
                }
            }
        }
    }

    if let Some(anchor) = best_history_anchor {
        return anchor;
    }

    // Step 2: Heuristic candidate evaluation (disassembler code guessing).
    // Collect candidates that sweep forward cleanly to target_pc.
    struct Candidate {
        addr: u32,
        first_inst_len: u32,
        score: i32,
    }

    let max_back_bytes = (desired_prior_instructions * 8 + 4).min(36) as u32;
    let mut candidates: Vec<Candidate> = Vec::new();

    let mut delta = 2u32;
    while delta <= max_back_bytes {
        if target_pc < delta {
            break;
        }
        let candidate_addr = (target_pc - delta) & 0x00FF_FFFE;

        let mut curr = candidate_addr;
        let mut count = 0;
        let mut score = 0i32;
        let mut first_len = 0u32;
        let mut reached_target = false;

        while curr < target_pc && count <= desired_prior_instructions + 3 {
            let (disasm, byte_len) = disassemble(curr, &read_word);
            if count == 0 {
                first_len = byte_len;
            }
            curr = curr.wrapping_add(byte_len);
            count += 1;

            if disasm.mnemonic == "DATA.W" {
                score -= 1000;
            } else if disasm.mnemonic == "ORI.B" && disasm.words[0] == 0x0000 {
                // $0000 in memory is uninitialized RAM or padding
                score -= 60;
            } else {
                score += 30;
            }

            if curr == target_pc {
                reached_target = true;
                break;
            }
        }

        if reached_target && curr == target_pc && score > 0 {
            let dist_from_desired = (count as isize - desired_prior_instructions as isize).abs();
            score += 40 - (dist_from_desired as i32 * 10);
            candidates.push(Candidate {
                addr: candidate_addr,
                first_inst_len: first_len,
                score,
            });
        }

        delta += 2;
    }

    // Filter out candidates that fall strictly inside the first instruction of an earlier candidate
    let valid_candidates: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| {
            !candidates
                .iter()
                .any(|p| p.addr < c.addr && c.addr < p.addr.wrapping_add(p.first_inst_len))
        })
        .collect();

    let mut best_candidate = target_pc;
    let mut best_score = 0i32;

    for c in valid_candidates {
        if c.score > best_score || (c.score == best_score && c.addr < best_candidate) {
            best_score = c.score;
            best_candidate = c.addr;
        }
    }

    best_candidate
}
