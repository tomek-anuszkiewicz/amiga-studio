//! Execution audit before timing for decoder-derived benchmark candidates.
//!
//! This is a fixture discovery pass. A sequential first step is not a validated
//! 700-target workload and must never be included in performance results.

use std::fs;
use std::path::Path;

use cpu::Cpu;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ENTRY: usize = 0x1000;
const HANDLER: usize = 0x3000;
pub const EXIT: usize = 0x4FFE;
const RETURN_PAD: usize = 0x7000;
const OPERAND: u16 = 0x6000;

#[derive(Debug, Deserialize)]
pub struct Candidate {
    pub(super) case_id: String,
    pub(super) representative_opcode: String,
    pub(super) path: String,
    pub(super) index_choices: String,
    pub(super) cpu_mode: String,
}

#[derive(Debug, Serialize)]
struct ProbeRow {
    case_id: String,
    opcode: String,
    path: String,
    cpu_mode: String,
    emitted_words: Vec<String>,
    observed_instruction_pc: u32,
    observed_sr: u16,
    observed_clocks: u32,
    status: &'static str,
    exception_return_pc: Option<u32>,
    block_status: String,
    block_targets: usize,
    block_clocks: u64,
    block_min_instruction_clocks: u32,
    block_max_instruction_clocks: u32,
    fixture_sha256: String,
    final_state_sha256: String,
    replay_status: &'static str,
}

fn put_word(bus: &mut PhysicalMemory, address: usize, word: u16) {
    let bytes = word.to_be_bytes();
    bus.chip_ram[address] = bytes[0];
    bus.chip_ram[address + 1] = bytes[1];
}

fn ea_extension(mode: u16, reg: u16, index_choice: Option<u16>) -> Vec<u16> {
    match (mode, reg) {
        (5, _) => vec![0],
        (6, _) | (7, 3) => {
            let choice = index_choice.unwrap_or(0);
            let address_index = (choice & 2) != 0;
            let long_index = (choice & 1) != 0;
            vec![((address_index as u16) << 15) | ((long_index as u16) << 11)]
        }
        (7, 0) => vec![OPERAND],
        (7, 1) => vec![0, OPERAND],
        (7, 2) => vec![0],
        (7, 4) => vec![0],
        _ => Vec::new(),
    }
}

fn index_choices(source: &str) -> Vec<u16> {
    source
        .chars()
        .filter_map(|character| character.to_digit(10).map(|value| value as u16))
        .collect()
}

fn is_pc_relative_division(opcode: u16) -> bool {
    matches!(opcode & 0xF1C0, 0x80C0 | 0x81C0)
        && (opcode >> 3) & 7 == 7
        && matches!(opcode & 7, 2 | 3)
}

fn is_control_transfer(opcode: u16) -> bool {
    (0x4E90..=0x4EBF).contains(&opcode)
        || (0x4ED0..=0x4EFF).contains(&opcode)
        || opcode >> 8 == 0x61
}

fn is_return(opcode: u16) -> bool {
    matches!(opcode, 0x4E73 | 0x4E75 | 0x4E77)
}

fn needs_a7_reset(opcode: u16) -> bool {
    let family = opcode >> 12;
    let source_is_a7_update = matches!((opcode >> 3) & 7, 3 | 4) && opcode & 7 == 7;
    let move_to_a7 = matches!(family, 2 | 3) && (opcode >> 6) & 7 == 1 && (opcode >> 9) & 7 == 7;
    let arithmetic_to_a7 =
        matches!(family, 9 | 13) && (opcode >> 9) & 7 == 7 && matches!((opcode >> 6) & 7, 3 | 7);
    (source_is_a7_update && (move_to_a7 || arithmetic_to_a7)) || (opcode & 0xFFF8) == 0x4E58
}

fn is_pc_relative_fixture(opcode: u16, path: &str) -> bool {
    is_pc_relative_division(opcode)
        || ((opcode & 0xF1C0) == 0x4180
            && (opcode >> 3) & 7 == 7
            && opcode & 7 == 3
            && path == "in_range")
        || ((opcode & 0xFFC0) == 0x46C0
            && (opcode >> 3) & 7 == 7
            && matches!(opcode & 7, 2 | 3)
            && matches!(path, "remain_supervisor" | "enter_user"))
}

fn division_operand(path: &str) -> u16 {
    match path {
        "divide_zero" => 0,
        "valid_fast" => 0x7FFF,
        "in_range" => 3,
        "remain_supervisor" => 0x2700,
        "enter_user" => 0x0700,
        _ => 1,
    }
}

pub fn instruction_words(opcode: u16, candidate: &Candidate) -> Vec<u16> {
    let mut words = vec![opcode];
    let mut choices = index_choices(&candidate.index_choices).into_iter();
    let family = opcode >> 12;
    if (1..=3).contains(&family) {
        let source_mode = (opcode >> 3) & 7;
        let source_reg = opcode & 7;
        let destination_mode = (opcode >> 6) & 7;
        let destination_reg = (opcode >> 9) & 7;
        if source_mode == 7 && source_reg == 4 && family == 2 {
            words.extend([0, 0]);
        } else {
            words.extend(ea_extension(source_mode, source_reg, choices.next()));
        }
        words.extend(ea_extension(
            destination_mode,
            destination_reg,
            choices.next(),
        ));
    } else if family == 6 {
        if opcode & 0xff == 0 {
            words.push(4);
        }
    } else if (opcode & 0xF0F8) == 0x50C8 || (opcode & 0xFFF8) == 0x4E50 {
        words.push(2);
    } else if matches!(
        opcode,
        0x4E72 | 0x003C | 0x007C | 0x023C | 0x027C | 0x0A3C | 0x0A7C
    ) {
        let immediate = match (opcode, candidate.path.as_str()) {
            (0x027C, "enter_user") => 0xDFFF,
            (0x027C, _) => 0xFFFF,
            (0x0A7C, "enter_user") => 0x2000,
            (0x0A7C, _) => 0,
            (0x4E72, "enter_user") => 0x0700,
            _ => 0x2700,
        };
        words.push(immediate);
    } else if (opcode & 0xF138) == 0x0108 {
        words.push(0);
    } else if opcode >> 12 == 14 && (opcode >> 6) & 3 != 3 {
        // Register shifts encode their count in the primary opcode.
    } else if matches!(opcode, 0x4E70 | 0x4E71 | 0x4E73 | 0x4E75 | 0x4E76 | 0x4E77)
        || (0x4E40..=0x4E6F).contains(&opcode)
    {
        // Fixed control and system opcodes have no effective-address extension.
    } else {
        let mode = (opcode >> 3) & 7;
        let reg = opcode & 7;
        if (opcode & 0xFF00) == 0x0800 {
            words.push(if candidate.path == "high_bit" { 31 } else { 0 });
        } else if family == 0 && matches!(opcode >> 8, 0x00 | 0x02 | 0x04 | 0x06 | 0x0A | 0x0C) {
            words.push(0);
            if (opcode >> 6) & 3 == 2 {
                words.push(0);
            }
        }
        if (opcode & 0xFB80) == 0x4880 && mode >= 2 {
            let predecrement_store = opcode & 0x0400 == 0 && mode == 4;
            let mask = match (candidate.path.as_str(), predecrement_store) {
                ("empty_mask", _) => 0,
                ("single_non_a7", true) => 0x4000,
                ("single_non_a7", false) => 0x0002,
                ("single_a7", true) => 0x0001,
                ("single_a7", false) => 0x8000,
                ("multiple_non_a7", true) => 0x7000,
                ("multiple_non_a7", false) => 0x000E,
                ("multiple_a7", true) => 0x4001,
                ("multiple_a7", false) => 0x8002,
                _ => 1,
            };
            words.push(mask);
        }
        words.extend(ea_extension(mode, reg, choices.next()));
        if is_pc_relative_fixture(opcode, &candidate.path) {
            if let Some(extension) = words.last_mut() {
                *extension |= if candidate.path == "enter_user" { 6 } else { 4 };
            }
        }
        if (8..=13).contains(&family)
            && mode == 7
            && reg == 4
            && matches!((opcode >> 6) & 7, 2 | 7)
            && (opcode & 0xF1C0) != 0x80C0
            && (opcode & 0xF1C0) != 0x81C0
            && (opcode & 0xF1C0) != 0xC0C0
            && (opcode & 0xF1C0) != 0xC1C0
        {
            words.push(0);
        }
        if ((opcode & 0xF1C0) == 0x80C0 || (opcode & 0xF1C0) == 0x81C0) && mode == 7 && reg == 4 {
            if let Some(immediate) = words.last_mut() {
                *immediate = match candidate.path.as_str() {
                    "valid_fast" => 0x7FFF,
                    "divide_zero" => 0,
                    _ => 1,
                };
            }
        }
        if ((opcode & 0xF1C0) == 0xC0C0 || (opcode & 0xF1C0) == 0xC1C0) && mode == 7 && reg == 4 {
            if let Some(immediate) = words.last_mut() {
                *immediate = if candidate.path == "dense_operand" {
                    0xFFFF
                } else {
                    0
                };
            }
        }
        if (opcode & 0xFFC0) == 0x46C0 && mode == 7 && reg == 4 {
            if let Some(immediate) = words.last_mut() {
                *immediate = if candidate.path == "enter_user" {
                    0x0700
                } else {
                    0x2700
                };
            }
        }
    }
    words
}

fn set_condition(cpu: &mut Cpu, condition: u8, wanted: bool) -> bool {
    for ccr in 0..32u8 {
        cpu.state.set_ccr(ccr);
        if cpu.state.eval_condition(condition) == wanted {
            return true;
        }
    }
    false
}

fn fixture_sha256(candidate: &Candidate, cpu: &Cpu, bus: &PhysicalMemory) -> String {
    let mut hash = Sha256::new();
    hash.update(b"candidate-probe-fixture-v2");
    hash.update(candidate.case_id.as_bytes());
    hash.update(candidate.path.as_bytes());
    hash.update(candidate.cpu_mode.as_bytes());
    hash.update(final_state_sha256(cpu, bus).as_bytes());
    format!("{:x}", hash.finalize())
}

pub fn final_state_sha256(cpu: &Cpu, bus: &PhysicalMemory) -> String {
    let mut hash = Sha256::new();
    for register in 0..8 {
        hash.update(cpu.state.d_long(register).to_be_bytes());
        hash.update(cpu.state.a_long(register).to_be_bytes());
    }
    hash.update(cpu.state.sr().to_be_bytes());
    hash.update(cpu.state.pc().to_be_bytes());
    hash.update(cpu.state.ssp().to_be_bytes());
    hash.update(cpu.state.usp().to_be_bytes());
    hash.update(cpu.cycle_counter().to_be_bytes());
    hash.update(&bus.chip_ram[..0x10000]);
    format!("{:x}", hash.finalize())
}

fn prepare_cpu(cpu: &mut Cpu, candidate: &Candidate, opcode: u16) {
    *cpu = Cpu::new();
    cpu.state.set_ssp(0xF000);
    cpu.state.set_usp(0xE000);
    cpu.state.set_sr(if candidate.cpu_mode == "user" {
        0x0700
    } else {
        0x2700
    });
    for register in 0..8 {
        cpu.state.set_d_long(register, 0);
        if register < 7 {
            cpu.state
                .set_a_long(register, 0x6800 + register as u32 * 0x100);
        }
    }
    if index_choices(&candidate.index_choices)
        .iter()
        .any(|choice| *choice >= 2)
    {
        cpu.state.set_a_long(0, 0);
    }
    if candidate.path == "multiple_non_a7" && (opcode & 0xFB80) == 0x4880 && (opcode >> 3) & 7 == 4
    {
        cpu.state.set_a_long((opcode & 7) as usize, 0x9000);
    }
    let path = candidate.path.as_str();
    if (0x6200..=0x6FFF).contains(&opcode)
        || (opcode & 0xF0F8) == 0x50C8
        || (opcode & 0xF0C0) == 0x50C0
    {
        let condition = ((opcode >> 8) & 15) as u8;
        let wanted = matches!(path, "taken" | "condition_true");
        let _ = set_condition(cpu, condition, wanted);
    }
    if (opcode & 0xF1C0) == 0x80C0 || (opcode & 0xF1C0) == 0x81C0 {
        let destination = ((opcode >> 9) & 7) as usize;
        let dividend = match path {
            "valid_fast" => 0,
            "valid_slow" if opcode & 0x0100 != 0 => 0x7FFF,
            "valid_slow" => 0xFFFF,
            "overflow_positive" => 0x0001_0000,
            "overflow_negative" | "overflow" => 0xFFFF_0000,
            _ => 0x1234,
        };
        cpu.state.set_d_long(destination, dividend);
        let source = (opcode & 7) as usize;
        if (opcode >> 3) & 7 == 0 {
            let divisor = match path {
                "divide_zero" => 0,
                "valid_fast" => 0x7FFF,
                _ => 1,
            };
            cpu.state.set_d_word(source, divisor);
        }
    }
    if matches!(path, "zero_count" | "one_count" | "large_count")
        && opcode >> 12 == 14
        && opcode & 0x20 != 0
    {
        let count_register = ((opcode >> 9) & 7) as usize;
        let count = match path {
            "zero_count" => 0,
            "one_count" => 1,
            _ => 17,
        };
        cpu.state.set_d_long(count_register, count);
    }
    if matches!(path, "sparse_operand" | "dense_operand") {
        if (opcode >> 3) & 7 == 0 {
            let source = (opcode & 7) as usize;
            cpu.state
                .set_d_word(source, if path == "dense_operand" { 0xFFFF } else { 0 });
        }
    }
    if matches!(path, "low_bit" | "high_bit") && (opcode & 0xF100) == 0x0100 {
        let bit_register = ((opcode >> 9) & 7) as usize;
        cpu.state
            .set_d_long(bit_register, if path == "high_bit" { 31 } else { 0 });
    }
    if opcode == 0x4E76 {
        cpu.state.set_ccr(if path == "trap" { 2 } else { 0 });
    }
    if (opcode & 0xF1C0) == 0x4180 {
        let destination = ((opcode >> 9) & 7) as usize;
        let value = match path {
            "above_bound" => 0x7FFF,
            "negative" => 0xFFFF,
            _ => 0,
        };
        cpu.state.set_d_word(destination, value);
    }
    if (opcode & 0xF0F8) == 0x50C8 {
        let counter = (opcode & 7) as usize;
        if path == "branch" {
            cpu.state.set_d_word(counter, 1000);
        } else if path == "expired" {
            cpu.state.set_d_word(counter, 0);
        }
    }
    if (opcode & 0xFFC0) == 0x46C0 && (opcode >> 3) & 7 == 0 {
        let source = (opcode & 7) as usize;
        cpu.state
            .set_d_word(source, if path == "enter_user" { 0x0700 } else { 0x2700 });
    }
}

fn seed_memory(bus: &mut PhysicalMemory, path: &str) {
    bus.chip_ram[..0x10000].fill(0);
    let operand_word = match path {
        "divide_zero" | "sparse_operand" => 0,
        "dense_operand" => 0xFFFF,
        "valid_fast" => 0x7FFF,
        "valid_slow" | "overflow" | "overflow_positive" | "overflow_negative" => 1,
        "remain_supervisor" => 0x2700,
        "enter_user" => 0x0700,
        _ => 3,
    };
    for address in (0x5000..0x9000).step_by(2) {
        put_word(bus, address, operand_word);
    }
    for address in (0xB000..0x10000).step_by(2) {
        put_word(bus, address, operand_word);
    }
    for vector in 2..64 {
        let address = vector * 4;
        put_word(bus, address, 0);
        put_word(bus, address + 2, HANDLER as u16);
    }
    put_word(bus, HANDLER, 0x4E73);
}

fn validate_unrolled(
    bus: &mut PhysicalMemory,
    cpu: &mut Cpu,
    candidate: &Candidate,
    opcode: u16,
    words: &[u16],
    fixture_fingerprint: &mut String,
    prepared: Option<&mut Option<(Cpu, PhysicalMemory)>>,
) -> (String, usize, u64, u32, u32) {
    let branch_over_padding =
        candidate.path == "taken" || (opcode >> 8 == 0x60 && candidate.path == "normal");
    let trap_instruction = (0x4E40..=0x4E4F).contains(&opcode);
    let exception_path = matches!(
        candidate.path.as_str(),
        "above_bound" | "negative" | "divide_zero" | "privilege_violation" | "trap"
    ) || trap_instruction;
    let adjust_return_pc = matches!(
        candidate.path.as_str(),
        "divide_zero" | "privilege_violation"
    );
    let db_expired = candidate.path == "expired" && (opcode & 0xF0F8) == 0x50C8;
    let pc_relative_operand = is_pc_relative_fixture(opcode, &candidate.path);
    let a7_divide_zero = candidate.path == "divide_zero"
        && (opcode >> 3) & 7 == 4
        && opcode & 7 == 7
        && candidate.cpu_mode == "supervisor";
    let enter_user = candidate.path == "enter_user" && opcode != 0x4E72 && opcode != 0x4E73;
    let reset_a7 = needs_a7_reset(opcode);
    let restore_frame = (opcode & 0xFFF8) == 0x4E58 && (opcode & 7) != 7;
    let stride = words.len() * 2
        + if db_expired {
            6
        } else if pc_relative_operand {
            if enter_user {
                6
            } else {
                4
            }
        } else if a7_divide_zero {
            4
        } else if reset_a7 {
            6 + if restore_frame { 6 } else { 0 }
        } else if branch_over_padding || enter_user {
            2
        } else {
            0
        };
    let block_bytes = stride * 700;
    if ENTRY + block_bytes + 6 >= EXIT {
        return ("code_exceeds_reserved_area".to_string(), 0, 0, 0, 0);
    }
    seed_memory(bus, &candidate.path);
    prepare_cpu(cpu, candidate, opcode);
    let intended_ccr = cpu.state.ccr();
    if adjust_return_pc {
        let length = (words.len() * 2) as u16;
        if length == 0 || length > 8 || length & 1 != 0 {
            return (
                "unsupported_exception_instruction_length".to_string(),
                0,
                0,
                0,
                0,
            );
        }
        let quick = if length == 8 { 0 } else { length };
        put_word(bus, HANDLER, 0x50AF | (quick << 9));
        put_word(bus, HANDLER + 2, 2);
        put_word(bus, HANDLER + 4, 0x4E73);
    } else if enter_user {
        put_word(bus, HANDLER, 0x3EBC);
        put_word(bus, HANDLER + 2, 0x2700);
        put_word(bus, HANDLER + 4, 0x4E73);
    }
    for operation in 0..700 {
        let address = ENTRY + operation * stride;
        for (index, word) in words.iter().enumerate() {
            put_word(bus, address + index * 2, *word);
        }
        if branch_over_padding {
            put_word(bus, address + words.len() * 2, 0x4E71);
        } else if pc_relative_operand {
            let helper = address + words.len() * 2;
            if enter_user {
                put_word(bus, helper, 0x4E40);
                put_word(bus, helper + 2, 0x6002);
                put_word(bus, helper + 4, division_operand(&candidate.path));
            } else {
                put_word(bus, helper, 0x6002);
                put_word(bus, helper + 2, division_operand(&candidate.path));
            }
        } else if a7_divide_zero {
            put_word(bus, address + words.len() * 2, 0x426F);
            put_word(bus, address + words.len() * 2 + 2, 0xFFFE);
        } else if reset_a7 {
            let mut helper = address + words.len() * 2;
            if restore_frame {
                put_word(bus, helper, 0x207C | ((opcode & 7) << 9));
                put_word(bus, helper + 2, 0);
                put_word(bus, helper + 4, 0x6800 + (opcode & 7) * 0x100);
                helper += 6;
            }
            put_word(bus, helper, 0x2E7C);
            put_word(bus, helper + 2, 0);
            put_word(
                bus,
                helper + 4,
                if candidate.cpu_mode == "user" {
                    0xE000
                } else {
                    0xF000
                },
            );
        } else if enter_user {
            put_word(bus, address + words.len() * 2, 0x4E40);
        } else if db_expired {
            put_word(bus, address + words.len() * 2, 0x7000 | ((opcode & 7) << 9));
            put_word(bus, address + words.len() * 2 + 2, 0x44FC);
            put_word(bus, address + words.len() * 2 + 4, u16::from(intended_ccr));
        }
    }
    let footer = ENTRY + block_bytes;
    put_word(bus, footer, 0x4EF9);
    put_word(bus, footer + 2, 0);
    put_word(bus, footer + 4, EXIT as u16);
    put_word(bus, EXIT, 0x4E71);
    let initial_memory = bus.chip_ram.clone();
    cpu.set_pc_and_prime_prefetch(ENTRY as u32, bus);
    *fixture_fingerprint = fixture_sha256(candidate, cpu, bus);
    if let Some(slot) = prepared {
        *slot = Some((cpu.clone(), bus.clone()));
    }
    let mut clocks = 0u64;
    let mut min_instruction_clocks = u32::MAX;
    let mut max_instruction_clocks = 0;
    for operation in 0..700 {
        let expected_pc = ENTRY + operation * stride;
        if cpu.state.instruction_pc() != expected_pc as u32 {
            return (
                format!("wrong_path_at_{operation}"),
                operation,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
        let expected_condition = match candidate.path.as_str() {
            "taken" | "condition_true" => Some(true),
            "untaken" | "condition_false" | "branch" | "expired" => Some(false),
            _ => None,
        };
        if let Some(wanted) = expected_condition {
            let condition = ((opcode >> 8) & 15) as u8;
            if cpu.state.eval_condition(condition) != wanted {
                return (
                    format!("condition_changed_at_{operation}"),
                    operation,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
        }
        if candidate.path == "no_trap" && cpu.state.is_v() {
            return (
                format!("overflow_flag_changed_at_{operation}"),
                operation,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
        let counter_before = if matches!(candidate.path.as_str(), "branch" | "expired") {
            Some(cpu.state.d_word((opcode & 7) as usize))
        } else {
            None
        };
        let actual_opcode =
            u16::from_be_bytes([bus.chip_ram[expected_pc], bus.chip_ram[expected_pc + 1]]);
        if actual_opcode != opcode {
            return (
                format!("target_modified_at_{operation}"),
                operation,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
        let step_clocks = cpu.step_instruction(bus);
        if step_clocks == 0 || cpu.state.is_halted() || cpu.state.is_stopped() {
            return (
                format!("halt_or_no_progress_at_{operation}"),
                operation,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
        clocks += u64::from(step_clocks);
        if matches!(candidate.path.as_str(), "valid_fast" | "valid_slow") && cpu.state.is_v() {
            return (
                format!("unexpected_divide_overflow_at_{operation}"),
                operation + 1,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
        if candidate.path.contains("overflow") && !cpu.state.is_v() {
            return (
                format!("missing_divide_overflow_at_{operation}"),
                operation + 1,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
        min_instruction_clocks = min_instruction_clocks.min(step_clocks);
        max_instruction_clocks = max_instruction_clocks.max(step_clocks);
        if let Some(before) = counter_before {
            let after = cpu.state.d_word((opcode & 7) as usize);
            if after != before.wrapping_sub(1)
                || (candidate.path == "branch" && after == 0xFFFF)
                || (candidate.path == "expired" && after != 0xFFFF)
            {
                return (
                    format!("wrong_dbcc_counter_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
        }
        if exception_path {
            if cpu.state.instruction_pc() != HANDLER as u32 {
                return (
                    format!("expected_exception_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            if adjust_return_pc {
                let adjust_clocks = cpu.step_instruction(bus);
                if adjust_clocks == 0 || cpu.state.instruction_pc() != (HANDLER + 4) as u32 {
                    return (
                        format!("exception_pc_adjust_failed_at_{operation}"),
                        operation + 1,
                        clocks,
                        min_instruction_clocks,
                        max_instruction_clocks,
                    );
                }
                clocks += u64::from(adjust_clocks);
            }
            let return_clocks = cpu.step_instruction(bus);
            if return_clocks == 0 {
                return (
                    format!("exception_return_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            clocks += u64::from(return_clocks);
        }
        if db_expired {
            if cpu.state.instruction_pc() != (expected_pc + words.len() * 2) as u32 {
                return (
                    format!("expired_dbcc_did_not_fall_through_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            let helper_clocks = cpu.step_instruction(bus);
            if helper_clocks == 0 || cpu.state.d_word((opcode & 7) as usize) != 0 {
                return (
                    format!("dbcc_counter_reset_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            clocks += u64::from(helper_clocks);
            let ccr_clocks = cpu.step_instruction(bus);
            if ccr_clocks == 0 || cpu.state.ccr() != intended_ccr {
                return (
                    format!("dbcc_condition_reset_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            clocks += u64::from(ccr_clocks);
        }
        if pc_relative_operand && !enter_user {
            if cpu.state.instruction_pc() != (expected_pc + words.len() * 2) as u32 {
                return (
                    format!("pc_operand_helper_not_reached_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            let helper_clocks = cpu.step_instruction(bus);
            if helper_clocks == 0 {
                return (
                    format!("pc_operand_helper_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            clocks += u64::from(helper_clocks);
        }
        if a7_divide_zero {
            if cpu.state.instruction_pc() != (expected_pc + words.len() * 2) as u32 {
                return (
                    format!("a7_divide_helper_not_reached_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            let helper_clocks = cpu.step_instruction(bus);
            if helper_clocks == 0 {
                return (
                    format!("a7_divide_helper_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            clocks += u64::from(helper_clocks);
        }
        if reset_a7 {
            if cpu.state.instruction_pc() != (expected_pc + words.len() * 2) as u32 {
                return (
                    format!("a7_reset_helper_not_reached_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            if restore_frame {
                let frame_clocks = cpu.step_instruction(bus);
                if frame_clocks == 0
                    || cpu.state.a_long((opcode & 7) as usize)
                        != 0x6800 + u32::from(opcode & 7) * 0x100
                {
                    return (
                        format!("frame_register_reset_failed_at_{operation}"),
                        operation + 1,
                        clocks,
                        min_instruction_clocks,
                        max_instruction_clocks,
                    );
                }
                clocks += u64::from(frame_clocks);
            }
            let stack_clocks = cpu.step_instruction(bus);
            let expected_stack = if candidate.cpu_mode == "user" {
                0xE000
            } else {
                0xF000
            };
            if stack_clocks == 0 || cpu.state.a_long(7) != expected_stack {
                return (
                    format!("a7_reset_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            clocks += u64::from(stack_clocks);
        }
        if matches!(candidate.path.as_str(), "remain_supervisor" | "enter_user") {
            if candidate.path == "remain_supervisor" && !cpu.state.is_supervisor() {
                return (
                    format!("unexpected_user_mode_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            if enter_user {
                if cpu.state.is_supervisor()
                    || cpu.state.instruction_pc() != (expected_pc + words.len() * 2) as u32
                {
                    return (
                        format!("did_not_enter_user_at_{operation}"),
                        operation + 1,
                        clocks,
                        min_instruction_clocks,
                        max_instruction_clocks,
                    );
                }
                let trap_clocks = cpu.step_instruction(bus);
                if trap_clocks == 0 || cpu.state.instruction_pc() != HANDLER as u32 {
                    return (
                        format!("supervisor_restore_trap_failed_at_{operation}"),
                        operation + 1,
                        clocks,
                        min_instruction_clocks,
                        max_instruction_clocks,
                    );
                }
                let move_clocks = cpu.step_instruction(bus);
                let return_clocks = cpu.step_instruction(bus);
                if move_clocks == 0 || return_clocks == 0 || !cpu.state.is_supervisor() {
                    return (
                        format!("supervisor_restore_failed_at_{operation}"),
                        operation + 1,
                        clocks,
                        min_instruction_clocks,
                        max_instruction_clocks,
                    );
                }
                clocks +=
                    u64::from(trap_clocks) + u64::from(move_clocks) + u64::from(return_clocks);
            }
        }
        if pc_relative_operand && enter_user {
            if cpu.state.instruction_pc() != (expected_pc + words.len() * 2 + 2) as u32 {
                return (
                    format!("pc_sr_helper_not_reached_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            let helper_clocks = cpu.step_instruction(bus);
            if helper_clocks == 0 {
                return (
                    format!("pc_sr_helper_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_instruction_clocks,
                    max_instruction_clocks,
                );
            }
            clocks += u64::from(helper_clocks);
        }
        if cpu.state.instruction_pc() != (expected_pc + stride) as u32 {
            return (
                format!("wrong_next_pc_at_{operation}"),
                operation + 1,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
    }
    let footer_clocks = cpu.step_instruction(bus);
    if footer_clocks == 0 || cpu.state.instruction_pc() != EXIT as u32 {
        return (
            "footer_did_not_reach_exit".to_string(),
            700,
            clocks,
            min_instruction_clocks,
            max_instruction_clocks,
        );
    }
    if matches!(
        candidate.path.as_str(),
        "valid_fast" | "valid_slow" | "overflow" | "overflow_positive" | "overflow_negative"
    ) && min_instruction_clocks != max_instruction_clocks
    {
        return (
            "divide_path_clocks_changed".to_string(),
            700,
            clocks,
            min_instruction_clocks,
            max_instruction_clocks,
        );
    }
    for (address, (&before, &after)) in initial_memory.iter().zip(bus.chip_ram.iter()).enumerate() {
        let operand_write = (0x5000..0x9000).contains(&address);
        let stack_write = (0xB000..0x10000).contains(&address);
        if before != after && !operand_write && !stack_write {
            return (
                format!("write_outside_fixture_at_{address:06X}"),
                700,
                clocks,
                min_instruction_clocks,
                max_instruction_clocks,
            );
        }
    }
    (
        "sequential_700_reached_exit".to_string(),
        700,
        clocks,
        min_instruction_clocks,
        max_instruction_clocks,
    )
}

fn validate_control_unrolled(
    bus: &mut PhysicalMemory,
    cpu: &mut Cpu,
    candidate: &Candidate,
    opcode: u16,
    words: &[u16],
    fixture_fingerprint: &mut String,
    prepared: Option<&mut Option<(Cpu, PhysicalMemory)>>,
) -> (String, usize, u64, u32, u32) {
    let is_bsr = opcode >> 8 == 0x61;
    let is_jsr = (0x4E90..=0x4EBF).contains(&opcode);
    let mode = (opcode >> 3) & 7;
    let reg = opcode & 7;
    let local_return = is_bsr || (mode == 7 && reg == 3);
    let stride = words.len() * 2 + if local_return { 4 } else { 0 };
    if ENTRY + stride * 700 + 6 >= EXIT {
        return ("control_code_exceeds_reserved_area".to_string(), 0, 0, 0, 0);
    }
    seed_memory(bus, &candidate.path);
    prepare_cpu(cpu, candidate, opcode);
    put_word(bus, RETURN_PAD, 0x4E75);
    if (2..=6).contains(&mode) {
        cpu.state.set_a_long(reg as usize, RETURN_PAD as u32);
    }
    for operation in 0..700 {
        let address = ENTRY + operation * stride;
        let mut target_words = words.to_vec();
        if !is_bsr && mode == 7 {
            match reg {
                0 => target_words[1] = RETURN_PAD as u16,
                1 => {
                    target_words[1] = 0;
                    target_words[2] = RETURN_PAD as u16;
                }
                2 => target_words[1] = (RETURN_PAD as i32 - (address + 2) as i32) as u16,
                3 => target_words[1] |= 4,
                _ => {}
            }
        }
        for (index, word) in target_words.iter().enumerate() {
            put_word(bus, address + index * 2, *word);
        }
        if local_return {
            put_word(bus, address + words.len() * 2, 0x6002);
            put_word(bus, address + words.len() * 2 + 2, 0x4E75);
        }
    }
    let footer = ENTRY + stride * 700;
    put_word(bus, footer, 0x4EF9);
    put_word(bus, footer + 2, 0);
    put_word(bus, footer + 4, EXIT as u16);
    put_word(bus, EXIT, 0x4E71);
    if !is_bsr && !is_jsr {
        let stack = cpu.state.a_long(7) as usize;
        if stack + 700 * 4 >= bus.chip_ram.len() {
            return ("control_return_stack_out_of_ram".to_string(), 0, 0, 0, 0);
        }
        for operation in 0..700 {
            let next = ENTRY + (operation + 1) * stride;
            put_word(bus, stack + operation * 4, (next >> 16) as u16);
            put_word(bus, stack + operation * 4 + 2, next as u16);
        }
    }
    let initial_memory = bus.chip_ram.clone();
    cpu.set_pc_and_prime_prefetch(ENTRY as u32, bus);
    *fixture_fingerprint = fixture_sha256(candidate, cpu, bus);
    if let Some(slot) = prepared {
        *slot = Some((cpu.clone(), bus.clone()));
    }
    let mut clocks = 0u64;
    let mut min_clocks = u32::MAX;
    let mut max_clocks = 0;
    for operation in 0..700 {
        let target_pc = ENTRY + operation * stride;
        if cpu.state.instruction_pc() != target_pc as u32 {
            return (
                format!("control_wrong_target_at_{operation}"),
                operation,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        let actual_opcode =
            u16::from_be_bytes([bus.chip_ram[target_pc], bus.chip_ram[target_pc + 1]]);
        if actual_opcode != opcode {
            return (
                format!("control_target_modified_at_{operation}"),
                operation,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        let target_clocks = cpu.step_instruction(bus);
        let expected_return_pc = if local_return {
            target_pc + words.len() * 2 + 2
        } else {
            RETURN_PAD
        };
        if target_clocks == 0 || cpu.state.instruction_pc() != expected_return_pc as u32 {
            return (
                format!("control_target_did_not_branch_at_{operation}"),
                operation + 1,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        clocks += u64::from(target_clocks);
        min_clocks = min_clocks.min(target_clocks);
        max_clocks = max_clocks.max(target_clocks);
        let return_clocks = cpu.step_instruction(bus);
        let expected_after_return = if is_jsr || is_bsr {
            target_pc + words.len() * 2
        } else {
            target_pc + stride
        };
        if return_clocks == 0 || cpu.state.instruction_pc() != expected_after_return as u32 {
            return (
                format!("control_return_failed_at_{operation}"),
                operation + 1,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        clocks += u64::from(return_clocks);
        if local_return && (is_jsr || is_bsr) {
            let skip_clocks = cpu.step_instruction(bus);
            if skip_clocks == 0 || cpu.state.instruction_pc() != (target_pc + stride) as u32 {
                return (
                    format!("control_skip_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_clocks,
                    max_clocks,
                );
            }
            clocks += u64::from(skip_clocks);
        }
    }
    let footer_clocks = cpu.step_instruction(bus);
    if footer_clocks == 0 || cpu.state.instruction_pc() != EXIT as u32 {
        return (
            "control_footer_failed".to_string(),
            700,
            clocks,
            min_clocks,
            max_clocks,
        );
    }
    for (address, (&before, &after)) in initial_memory.iter().zip(bus.chip_ram.iter()).enumerate() {
        if before != after && !(0xB000..0x10000).contains(&address) {
            return (
                format!("control_write_outside_stack_at_{address:06X}"),
                700,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
    }
    (
        "sequential_700_reached_exit".to_string(),
        700,
        clocks,
        min_clocks,
        max_clocks,
    )
}

fn validate_jump_only(
    bus: &mut PhysicalMemory,
    cpu: &mut Cpu,
    candidate: &Candidate,
    opcode: u16,
    words: &[u16],
    fixture_fingerprint: &mut String,
    prepared: Option<&mut Option<(Cpu, PhysicalMemory)>>,
) -> (String, usize, u64, u32, u32) {
    let mode = (opcode >> 3) & 7;
    let reg = opcode & 7;
    let fixed_target = matches!(mode, 2 | 6);
    let stride = words.len() * 2;
    if ENTRY + stride * 700 >= HANDLER {
        return ("jump_code_exceeds_reserved_area".to_string(), 0, 0, 0, 0);
    }
    seed_memory(bus, &candidate.path);
    prepare_cpu(cpu, candidate, opcode);
    if matches!(mode, 2 | 5 | 6) {
        cpu.state.set_a_long(reg as usize, ENTRY as u32);
    }
    for operation in 0..700 {
        let address = if fixed_target {
            ENTRY
        } else {
            ENTRY + operation * stride
        };
        let next = if fixed_target || operation == 699 {
            address
        } else {
            address + stride
        };
        let mut target_words = words.to_vec();
        match (mode, reg) {
            (5, _) => target_words[1] = (next - ENTRY) as u16,
            (7, 0) => target_words[1] = next as u16,
            (7, 1) => {
                target_words[1] = 0;
                target_words[2] = next as u16;
            }
            (7, 2) => target_words[1] = (next as i32 - (address + 2) as i32) as u16,
            (7, 3) => {
                let displacement = (next as i32 - (address + 2) as i32) as i8;
                target_words[1] = (target_words[1] & 0xFF00) | displacement as u8 as u16;
            }
            _ => {}
        }
        for (index, word) in target_words.iter().enumerate() {
            put_word(bus, address + index * 2, *word);
        }
    }
    let initial_memory = bus.chip_ram.clone();
    cpu.set_pc_and_prime_prefetch(ENTRY as u32, bus);
    *fixture_fingerprint = fixture_sha256(candidate, cpu, bus);
    if let Some(slot) = prepared {
        *slot = Some((cpu.clone(), bus.clone()));
    }
    let mut clocks = 0u64;
    let mut min_clocks = u32::MAX;
    let mut max_clocks = 0;
    for operation in 0..700 {
        let address = if fixed_target {
            ENTRY
        } else {
            ENTRY + operation * stride
        };
        if cpu.state.instruction_pc() != address as u32 {
            return (
                format!("jump_wrong_target_at_{operation}"),
                operation,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        let actual_opcode = u16::from_be_bytes([bus.chip_ram[address], bus.chip_ram[address + 1]]);
        if actual_opcode != opcode {
            return (
                format!("jump_target_modified_at_{operation}"),
                operation,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        let target_clocks = cpu.step_instruction(bus);
        let expected_next = if fixed_target || operation == 699 {
            address
        } else {
            address + stride
        };
        if target_clocks == 0 || cpu.state.instruction_pc() != expected_next as u32 {
            return (
                format!("jump_did_not_branch_at_{operation}"),
                operation + 1,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        clocks += u64::from(target_clocks);
        min_clocks = min_clocks.min(target_clocks);
        max_clocks = max_clocks.max(target_clocks);
    }
    if bus.chip_ram != initial_memory {
        return (
            "jump_modified_memory".to_string(),
            700,
            clocks,
            min_clocks,
            max_clocks,
        );
    }
    (
        "counted_700_jumps".to_string(),
        700,
        clocks,
        min_clocks,
        max_clocks,
    )
}

fn validate_return_unrolled(
    bus: &mut PhysicalMemory,
    cpu: &mut Cpu,
    candidate: &Candidate,
    opcode: u16,
    fixture_fingerprint: &mut String,
    prepared: Option<&mut Option<(Cpu, PhysicalMemory)>>,
) -> (String, usize, u64, u32, u32) {
    let enter_user = opcode == 0x4E73 && candidate.path == "enter_user";
    let stride = if enter_user { 4 } else { 2 };
    let frame_size = if opcode == 0x4E75 { 4 } else { 6 };
    seed_memory(bus, &candidate.path);
    prepare_cpu(cpu, candidate, opcode);
    cpu.state.set_ssp(0xE000);
    cpu.state.set_usp(0xD000);
    let stack = if candidate.cpu_mode == "user" {
        0xD000
    } else {
        0xE000
    };
    cpu.state.set_a_long(7, stack);
    if enter_user {
        put_word(bus, HANDLER, 0x3EBC);
        put_word(bus, HANDLER + 2, 0x2700);
        put_word(bus, HANDLER + 4, 0x4E73);
    }
    for operation in 0..700 {
        let address = ENTRY + operation * stride;
        let next = if operation == 699 {
            EXIT
        } else {
            address + stride
        };
        put_word(bus, address, opcode);
        if enter_user {
            put_word(bus, address + 2, 0x4E40);
        }
        let frame = stack as usize + operation * frame_size;
        if opcode != 0x4E75 {
            let status = if opcode == 0x4E73 {
                if enter_user {
                    0x0700
                } else {
                    0x2700
                }
            } else {
                0
            };
            put_word(bus, frame, status);
        }
        let pc_offset = if opcode == 0x4E75 { 0 } else { 2 };
        let frame_pc = if enter_user && operation != 699 {
            address + 2
        } else {
            next
        };
        put_word(bus, frame + pc_offset, (frame_pc >> 16) as u16);
        put_word(bus, frame + pc_offset + 2, frame_pc as u16);
    }
    put_word(bus, EXIT, 0x4E71);
    let initial_memory = bus.chip_ram.clone();
    cpu.set_pc_and_prime_prefetch(ENTRY as u32, bus);
    *fixture_fingerprint = fixture_sha256(candidate, cpu, bus);
    if let Some(slot) = prepared {
        *slot = Some((cpu.clone(), bus.clone()));
    }
    let mut clocks = 0u64;
    let mut min_clocks = u32::MAX;
    let mut max_clocks = 0;
    for operation in 0..700 {
        let address = ENTRY + operation * stride;
        if cpu.state.instruction_pc() != address as u32 {
            return (
                format!("return_wrong_target_at_{operation}"),
                operation,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        let target_clocks = cpu.step_instruction(bus);
        let next = if operation == 699 {
            EXIT
        } else {
            address + stride
        };
        let expected = if enter_user && operation != 699 {
            address + 2
        } else {
            next
        };
        if target_clocks == 0 || cpu.state.instruction_pc() != expected as u32 {
            return (
                format!("return_wrong_pc_at_{operation}"),
                operation + 1,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
        clocks += u64::from(target_clocks);
        min_clocks = min_clocks.min(target_clocks);
        max_clocks = max_clocks.max(target_clocks);
        if enter_user {
            if cpu.state.is_supervisor() {
                return (
                    format!("rte_did_not_enter_user_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_clocks,
                    max_clocks,
                );
            }
            if operation == 699 {
                continue;
            }
            let trap_clocks = cpu.step_instruction(bus);
            if trap_clocks == 0 || cpu.state.instruction_pc() != HANDLER as u32 {
                return (
                    format!("rte_restore_trap_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_clocks,
                    max_clocks,
                );
            }
            let move_clocks = cpu.step_instruction(bus);
            let rte_clocks = cpu.step_instruction(bus);
            if move_clocks == 0
                || rte_clocks == 0
                || !cpu.state.is_supervisor()
                || cpu.state.instruction_pc() != next as u32
            {
                return (
                    format!("rte_supervisor_restore_failed_at_{operation}"),
                    operation + 1,
                    clocks,
                    min_clocks,
                    max_clocks,
                );
            }
            clocks += u64::from(trap_clocks) + u64::from(move_clocks) + u64::from(rte_clocks);
        }
    }
    for (address, (&before, &after)) in initial_memory.iter().zip(bus.chip_ram.iter()).enumerate() {
        if before != after && !(0xB000..0x10000).contains(&address) {
            return (
                format!("return_write_outside_stack_at_{address:06X}"),
                700,
                clocks,
                min_clocks,
                max_clocks,
            );
        }
    }
    (
        "sequential_700_reached_exit".to_string(),
        700,
        clocks,
        min_clocks,
        max_clocks,
    )
}

fn validate_stop(
    bus: &mut PhysicalMemory,
    cpu: &mut Cpu,
    candidate: &Candidate,
    words: &[u16],
    fixture_fingerprint: &mut String,
    prepared: Option<&mut Option<(Cpu, PhysicalMemory)>>,
) -> (String, usize, u64, u32, u32) {
    seed_memory(bus, &candidate.path);
    for (index, word) in words.iter().enumerate() {
        put_word(bus, ENTRY + index * 2, *word);
    }
    prepare_cpu(cpu, candidate, words[0]);
    cpu.set_pc_and_prime_prefetch(ENTRY as u32, bus);
    *fixture_fingerprint = fixture_sha256(candidate, cpu, bus);
    if let Some(slot) = prepared {
        *slot = Some((cpu.clone(), bus.clone()));
    }
    let clocks = cpu.step_instruction(bus);
    let expected_supervisor = candidate.path == "remain_supervisor";
    if clocks == 0
        || !cpu.state.is_stopped()
        || cpu.state.is_supervisor() != expected_supervisor
        || cpu.state.instruction_pc() != (ENTRY + words.len() * 2) as u32
    {
        return (
            "stop_wrong_state".to_string(),
            0,
            u64::from(clocks),
            clocks,
            clocks,
        );
    }
    (
        "single_target_stop".to_string(),
        1,
        u64::from(clocks),
        clocks,
        clocks,
    )
}

pub fn validate_candidate(
    bus: &mut PhysicalMemory,
    cpu: &mut Cpu,
    candidate: &Candidate,
    opcode: u16,
    words: &[u16],
    fingerprint: &mut String,
    prepared: Option<&mut Option<(Cpu, PhysicalMemory)>>,
) -> (String, usize, u64, u32, u32) {
    if candidate.path == "privilege_violation" {
        validate_unrolled(bus, cpu, candidate, opcode, words, fingerprint, prepared)
    } else if opcode == 0x4E72 {
        validate_stop(bus, cpu, candidate, words, fingerprint, prepared)
    } else if is_return(opcode) {
        validate_return_unrolled(bus, cpu, candidate, opcode, fingerprint, prepared)
    } else if (0x4ED0..=0x4EFF).contains(&opcode) {
        validate_jump_only(bus, cpu, candidate, opcode, words, fingerprint, prepared)
    } else if is_control_transfer(opcode) {
        validate_control_unrolled(bus, cpu, candidate, opcode, words, fingerprint, prepared)
    } else {
        validate_unrolled(bus, cpu, candidate, opcode, words, fingerprint, prepared)
    }
}

pub fn probe_candidates(directory: &Path) -> Result<(), String> {
    let input = fs::read_to_string(directory.join("case_candidates.json"))
        .map_err(|error| error.to_string())?;
    let candidates: Vec<Candidate> =
        serde_json::from_str(&input).map_err(|error| error.to_string())?;
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    let mut cpu = Cpu::new();
    let mut rows = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let opcode = u16::from_str_radix(&candidate.representative_opcode, 16)
            .map_err(|error| error.to_string())?;
        seed_memory(&mut bus, &candidate.path);
        let words = instruction_words(opcode, &candidate);
        for (index, word) in words.iter().enumerate() {
            put_word(&mut bus, ENTRY + index * 2, *word);
        }
        put_word(&mut bus, ENTRY + words.len() * 2, 0x4E71);
        if is_pc_relative_fixture(opcode, &candidate.path) {
            let helper = ENTRY + words.len() * 2;
            if candidate.path == "enter_user" {
                put_word(&mut bus, helper, 0x4E40);
                put_word(&mut bus, helper + 2, 0x6002);
                put_word(&mut bus, helper + 4, division_operand(&candidate.path));
            } else {
                put_word(&mut bus, helper, 0x6002);
                put_word(&mut bus, helper + 2, division_operand(&candidate.path));
            }
        }
        prepare_cpu(&mut cpu, &candidate, opcode);
        cpu.set_pc_and_prime_prefetch(ENTRY as u32, &mut bus);
        let clocks = cpu.step_instruction(&mut bus);
        let observed_pc = cpu.state.instruction_pc();
        let next_pc = (ENTRY + words.len() * 2) as u32;
        let status = if clocks == 0 {
            "no_progress"
        } else if observed_pc == HANDLER as u32 {
            "exception"
        } else if cpu.state.is_stopped() {
            "stopped"
        } else if observed_pc == next_pc {
            "sequential_first_step"
        } else {
            "different_path_or_fixture"
        };
        let exception_return_pc = if status == "exception" {
            let return_clocks = cpu.step_instruction(&mut bus);
            if return_clocks > 0 {
                Some(cpu.state.instruction_pc())
            } else {
                None
            }
        } else {
            None
        };
        let repeatable_path = matches!(
            candidate.path.as_str(),
            "normal"
                | "taken"
                | "untaken"
                | "condition_true"
                | "condition_false"
                | "no_trap"
                | "in_range"
                | "sparse_operand"
                | "dense_operand"
                | "zero_count"
                | "one_count"
                | "eight_count"
                | "large_count"
                | "low_bit"
                | "high_bit"
                | "empty_mask"
                | "single_non_a7"
                | "single_a7"
                | "multiple_non_a7"
                | "multiple_a7"
                | "valid_fast"
                | "valid_slow"
                | "overflow"
                | "overflow_positive"
                | "overflow_negative"
                | "above_bound"
                | "negative"
                | "divide_zero"
                | "privilege_violation"
                | "trap"
                | "branch"
                | "expired"
                | "remain_supervisor"
                | "enter_user"
        );
        let branch_over_padding =
            candidate.path == "taken" || (opcode >> 8 == 0x60 && candidate.path == "normal");
        let first_step_valid = status == "sequential_first_step"
            || is_control_transfer(opcode)
            || is_return(opcode)
            || opcode == 0x4E72
            || (branch_over_padding && observed_pc == (ENTRY + words.len() * 2 + 2) as u32)
            || (matches!(candidate.path.as_str(), "above_bound" | "negative")
                && status == "exception"
                && exception_return_pc == Some((ENTRY + words.len() * 2) as u32))
            || (matches!(
                candidate.path.as_str(),
                "divide_zero" | "privilege_violation"
            ) && status == "exception"
                && exception_return_pc == Some(ENTRY as u32))
            || ((candidate.path == "trap" || (0x4E40..=0x4E4F).contains(&opcode))
                && status == "exception"
                && exception_return_pc == Some((ENTRY + words.len() * 2) as u32));
        let mut fixture_fingerprint = String::new();
        let (
            mut block_status,
            block_targets,
            block_clocks,
            block_min_instruction_clocks,
            block_max_instruction_clocks,
        ) = if first_step_valid && repeatable_path {
            validate_candidate(
                &mut bus,
                &mut cpu,
                &candidate,
                opcode,
                &words,
                &mut fixture_fingerprint,
                None,
            )
        } else {
            ("requires_distinct_path_recipe".to_string(), 0, 0, 0, 0)
        };
        let state_fingerprint = final_state_sha256(&cpu, &bus);
        let mut replay_status = "not_run";
        if matches!(
            block_status.as_str(),
            "sequential_700_reached_exit" | "single_target_stop" | "counted_700_jumps"
        ) {
            let mut repeat_fingerprint = String::new();
            let repeat = validate_candidate(
                &mut bus,
                &mut cpu,
                &candidate,
                opcode,
                &words,
                &mut repeat_fingerprint,
                None,
            );
            let repeated_state = final_state_sha256(&cpu, &bus);
            if repeat.0 == block_status
                && repeat.1 == block_targets
                && repeat.2 == block_clocks
                && repeat_fingerprint == fixture_fingerprint
                && repeated_state == state_fingerprint
            {
                replay_status = "identical";
            } else {
                block_status = "replay_mismatch".to_string();
                replay_status = "different";
            }
        }
        rows.push(ProbeRow {
            case_id: candidate.case_id,
            opcode: candidate.representative_opcode,
            path: candidate.path,
            cpu_mode: candidate.cpu_mode,
            emitted_words: words.iter().map(|word| format!("{word:04X}")).collect(),
            observed_instruction_pc: observed_pc,
            observed_sr: cpu.state.sr(),
            observed_clocks: clocks,
            status,
            exception_return_pc,
            block_status,
            block_targets,
            block_clocks,
            block_min_instruction_clocks,
            block_max_instruction_clocks,
            fixture_sha256: fixture_fingerprint,
            final_state_sha256: state_fingerprint,
            replay_status,
        });
    }
    let output = serde_json::to_string_pretty(&rows).map_err(|error| error.to_string())?;
    fs::write(directory.join("candidate_probe.json"), output).map_err(|error| error.to_string())?;
    for status in [
        "sequential_first_step",
        "exception",
        "stopped",
        "different_path_or_fixture",
        "no_progress",
    ] {
        println!(
            "{status}: {}",
            rows.iter().filter(|row| row.status == status).count()
        );
    }
    println!(
        "700-target repeatable blocks: {}",
        rows.iter()
            .filter(|row| row.block_status == "sequential_700_reached_exit")
            .count()
    );
    println!(
        "single-target stopped cases: {}",
        rows.iter()
            .filter(|row| row.block_status == "single_target_stop")
            .count()
    );
    Ok(())
}
