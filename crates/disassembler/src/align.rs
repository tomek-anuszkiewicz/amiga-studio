//! Heuristic anchor finder for backward disassembly alignment in variable-length CISC streams

use crate::disassemble;

const BOUNDARY_LOOK_BACK_BYTES: u32 = 32;
const BOUNDARY_EXTRA_INSTRUCTIONS: usize = 2;
const ESTIMATED_INSTRUCTION_BYTES: usize = 8;
const SEARCH_EXTRA_BYTES: usize = 4;
const MAX_LOOK_BACK_BYTES: usize = 36;
const SEARCH_EXTRA_INSTRUCTIONS: usize = 3;
const INVALID_INSTRUCTION_PENALTY: i32 = 1000;
const EMPTY_ORI_OPCODE: u16 = 0x0000;
const EMPTY_ORI_PENALTY: i32 = 60;
const VALID_INSTRUCTION_REWARD: i32 = 30;
const DESIRED_DISTANCE_REWARD: i32 = 40;
const DISTANCE_PENALTY: i32 = 10;

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
    let target_pc = target_pc & crate::encoding::MASK_WORD_ADDRESS;
    if desired_prior_instructions == 0 || target_pc == 0 {
        return target_pc;
    }

    // Step 1: Check known boundaries from execution history (temporal/trace).
    // If we find an anchor in history that cleanly sweeps forward to `target_pc`,
    // that is verified ground truth from hardware execution.
    let mut best_history_anchor: Option<u32> = None;
    for &boundary in known_boundaries.iter().rev() {
        let b = boundary & crate::encoding::MASK_WORD_ADDRESS;
        if b < target_pc && target_pc.saturating_sub(b) <= BOUNDARY_LOOK_BACK_BYTES {
            let mut curr = b;
            let mut count = 0;
            let mut matched = false;
            while curr < target_pc
                && count <= desired_prior_instructions + BOUNDARY_EXTRA_INSTRUCTIONS
            {
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

    let max_back_bytes = (desired_prior_instructions * ESTIMATED_INSTRUCTION_BYTES
        + SEARCH_EXTRA_BYTES)
        .min(MAX_LOOK_BACK_BYTES) as u32;
    let mut candidates: Vec<Candidate> = Vec::new();

    let mut delta = 2u32;
    while delta <= max_back_bytes {
        if target_pc < delta {
            break;
        }
        let candidate_addr = (target_pc - delta) & crate::encoding::MASK_WORD_ADDRESS;

        let mut curr = candidate_addr;
        let mut count = 0;
        let mut score = 0i32;
        let mut first_len = 0u32;
        let mut reached_target = false;

        while curr < target_pc && count <= desired_prior_instructions + SEARCH_EXTRA_INSTRUCTIONS {
            let (disasm, byte_len) = disassemble(curr, &read_word);
            if count == 0 {
                first_len = byte_len;
            }
            curr = curr.wrapping_add(byte_len);
            count += 1;

            if disasm.mnemonic == "DATA.W" {
                score -= INVALID_INSTRUCTION_PENALTY;
            } else if disasm.mnemonic == "ORI.B" && disasm.words[0] == EMPTY_ORI_OPCODE {
                // $0000 in memory is uninitialized RAM or padding
                score -= EMPTY_ORI_PENALTY;
            } else {
                score += VALID_INSTRUCTION_REWARD;
            }

            if curr == target_pc {
                reached_target = true;
                break;
            }
        }

        if reached_target && curr == target_pc && score > 0 {
            let dist_from_desired = (count as isize - desired_prior_instructions as isize).abs();
            score += DESIRED_DISTANCE_REWARD - (dist_from_desired as i32 * DISTANCE_PENALTY);
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
        let is_better_candidate =
            c.score > best_score || (c.score == best_score && c.addr < best_candidate);
        if is_better_candidate {
            best_score = c.score;
            best_candidate = c.addr;
        }
    }

    best_candidate
}
