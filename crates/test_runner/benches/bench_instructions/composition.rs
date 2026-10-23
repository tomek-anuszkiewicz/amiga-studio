//! Counts the guest instructions actually executed by each validated workload.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use cpu::Cpu;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::candidate_measure::MAX_INSTRUCTIONS_PER_BLOCK;
use super::candidate_probe::{
    final_state_sha256, instruction_words, validate_candidate, Candidate, EXIT,
};

#[derive(Deserialize)]
struct ValidatedProbe {
    case_id: String,
    opcode: String,
    block_status: String,
    block_targets: usize,
    block_clocks: u64,
    fixture_sha256: String,
    final_state_sha256: String,
    replay_status: String,
}

#[derive(Serialize)]
struct InstructionCount {
    opcode: String,
    form: String,
    semantic_group: String,
    count: usize,
}

#[derive(Serialize)]
struct CompositionCase {
    case_id: String,
    target_opcode: String,
    target_instruction: String,
    target_semantic_group: String,
    path: String,
    cpu_mode: String,
    fixture_sha256: String,
    target_count: usize,
    total_guest_instructions: usize,
    additional_instructions: Vec<InstructionCount>,
}

#[derive(Serialize)]
struct CompositionReport {
    schema_version: u8,
    candidate_count: usize,
    candidates_sha256: String,
    probe_sha256: String,
    opcode_coverage_sha256: String,
    cases: Vec<CompositionCase>,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn opcode_groups(bytes: &[u8]) -> Result<HashMap<u16, String>, String> {
    let input = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let mut groups = HashMap::new();
    for row in input.lines().skip(1) {
        let columns: Vec<&str> = row.trim_end_matches('\r').split(',').collect();
        if columns.len() != 5 {
            return Err("opcode coverage has an unexpected column count".to_string());
        }
        if columns[1] == "legal" {
            let opcode = u16::from_str_radix(columns[0], 16).map_err(|error| error.to_string())?;
            if groups.insert(opcode, columns[2].to_string()).is_some() {
                return Err(format!("opcode {opcode:04X} has duplicate coverage rows"));
            }
        }
    }
    Ok(groups)
}

fn form(opcode: u16) -> String {
    match opcode {
        0x4E73 => "RTE".to_string(),
        0x4E75 => "RTS".to_string(),
        0x4EF9 => "JMP abs.l".to_string(),
        0x4FF9 => "LEA abs.l,A7".to_string(),
        0x3EBC => "MOVE.W #imm,(An)".to_string(),
        0x44FC => "MOVE #imm,CCR".to_string(),
        0x426F => "CLR.W d16(A7)".to_string(),
        0x6002 => "BRA.s".to_string(),
        0x4E71 => "NOP".to_string(),
        0x4E40..=0x4E4F => format!("TRAP #{}", opcode & 15),
        0x7000..=0x7EFF if opcode & 0xFF == 0 => {
            format!("MOVEQ #0,D{}", (opcode >> 9) & 7)
        }
        _ if opcode & 0xF1FF == 0x50AF => {
            let quick = (opcode >> 9) & 7;
            format!("ADDQ.L #{},d16(A7)", if quick == 0 { 8 } else { quick })
        }
        _ if opcode & 0xF1FF == 0x41F9 => {
            format!("LEA abs.l,A{}", (opcode >> 9) & 7)
        }
        _ => format!("unclassified opcode {opcode:04X}"),
    }
}

fn target_instruction(words: &[u16]) -> Result<String, String> {
    if let Some(&opcode) = words.first() {
        if opcode & 0xFFC0 == 0x4800 {
            let mut extensions = words.iter().skip(1).copied();
            let operand =
                disassembler::format_ea(((opcode >> 3) & 7) as u8, (opcode & 7) as u8, || {
                    extensions.next().unwrap_or(0)
                });
            return Ok(format!("NBCD {operand}"));
        }
    }
    let (decoded, _) = disassembler::disassemble(0, |address| {
        words.get((address / 2) as usize).copied().unwrap_or(0)
    });
    if decoded.mnemonic == "DATA.W" || decoded.operands.contains("UNKNOWN_EA") {
        return Err(format!(
            "disassembler cannot name target opcode {:04X}",
            words.first().copied().unwrap_or(0)
        ));
    }
    if decoded.operands.is_empty() {
        Ok(decoded.mnemonic.to_string())
    } else {
        Ok(format!("{} {}", decoded.mnemonic, decoded.operands))
    }
}

fn count_case(
    candidate: &Candidate,
    expected: &ValidatedProbe,
    groups: &HashMap<u16, String>,
) -> Result<CompositionCase, String> {
    if candidate.case_id != expected.case_id
        || candidate.representative_opcode != expected.opcode
        || expected.replay_status != "identical"
        || !matches!(
            expected.block_status.as_str(),
            "sequential_700_reached_exit" | "single_target_stop" | "counted_700_jumps"
        )
    {
        return Err(format!("{} is not a validated workload", candidate.case_id));
    }
    let opcode = u16::from_str_radix(&candidate.representative_opcode, 16)
        .map_err(|error| error.to_string())?;
    let words = instruction_words(opcode, candidate);
    let mut cpu = Cpu::new();
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    let mut fingerprint = String::new();
    let mut initial = None;
    let result = validate_candidate(
        &mut bus,
        &mut cpu,
        candidate,
        opcode,
        &words,
        &mut fingerprint,
        Some(&mut initial),
    );
    if result.0 != expected.block_status
        || result.1 != expected.block_targets
        || result.2 != expected.block_clocks
        || fingerprint != expected.fixture_sha256
        || final_state_sha256(&cpu, &bus) != expected.final_state_sha256
    {
        return Err(format!("{} fixture changed since probe", candidate.case_id));
    }
    let (mut replay_cpu, mut replay_bus) =
        initial.ok_or_else(|| format!("{} has no initial fixture", candidate.case_id))?;
    let mut counts = BTreeMap::<u16, usize>::new();
    let mut steps = 0;
    let step_limit = if expected.block_status == "counted_700_jumps" {
        expected.block_targets
    } else {
        MAX_INSTRUCTIONS_PER_BLOCK
    };
    while !replay_cpu.state.is_stopped()
        && !replay_cpu.state.is_halted()
        && replay_cpu.state.instruction_pc() != EXIT as u32
        && steps < step_limit
    {
        let pc = replay_cpu.state.instruction_pc() as usize;
        let bytes = replay_bus
            .chip_ram
            .get(pc..pc + 2)
            .ok_or_else(|| format!("{} executed outside fixture RAM", candidate.case_id))?;
        let executed_opcode = u16::from_be_bytes([bytes[0], bytes[1]]);
        if replay_cpu.step_instruction(&mut replay_bus) == 0 {
            return Err(format!("{} stopped making progress", candidate.case_id));
        }
        *counts.entry(executed_opcode).or_default() += 1;
        steps += 1;
    }
    if replay_cpu != cpu
        || final_state_sha256(&replay_cpu, &replay_bus) != expected.final_state_sha256
    {
        return Err(format!("{} trace diverged from probe", candidate.case_id));
    }
    let target_total = counts
        .get_mut(&opcode)
        .ok_or_else(|| format!("{} did not execute its target opcode", candidate.case_id))?;
    if *target_total < expected.block_targets {
        return Err(format!(
            "{} executed too few target opcodes",
            candidate.case_id
        ));
    }
    *target_total -= expected.block_targets;
    let additional_instructions = counts
        .into_iter()
        .filter(|(_, count)| *count != 0)
        .map(|(opcode, count)| {
            Ok(InstructionCount {
                opcode: format!("{opcode:04X}"),
                form: form(opcode),
                semantic_group: groups
                    .get(&opcode)
                    .ok_or_else(|| format!("helper opcode {opcode:04X} lacks a legal group"))?
                    .clone(),
                count,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CompositionCase {
        case_id: candidate.case_id.clone(),
        target_opcode: candidate.representative_opcode.clone(),
        target_instruction: target_instruction(&words)?,
        target_semantic_group: groups
            .get(&opcode)
            .ok_or_else(|| format!("target opcode {opcode:04X} lacks a legal group"))?
            .clone(),
        path: candidate.path.clone(),
        cpu_mode: candidate.cpu_mode.clone(),
        fixture_sha256: fingerprint,
        target_count: expected.block_targets,
        total_guest_instructions: steps,
        additional_instructions,
    })
}

pub fn export_composition(directory: &Path) -> Result<(), String> {
    let candidates_bytes =
        fs::read(directory.join("case_candidates.json")).map_err(|error| error.to_string())?;
    let probe_bytes =
        fs::read(directory.join("candidate_probe.json")).map_err(|error| error.to_string())?;
    let coverage_bytes =
        fs::read(directory.join("opcode_coverage.csv")).map_err(|error| error.to_string())?;
    let groups = opcode_groups(&coverage_bytes)?;
    let candidates: Vec<Candidate> =
        serde_json::from_slice(&candidates_bytes).map_err(|error| error.to_string())?;
    let probes: Vec<ValidatedProbe> =
        serde_json::from_slice(&probe_bytes).map_err(|error| error.to_string())?;
    if candidates.len() != probes.len() {
        return Err("candidate and probe counts differ".to_string());
    }
    let mut cases = Vec::with_capacity(candidates.len());
    for (candidate, probe) in candidates.iter().zip(&probes) {
        cases.push(count_case(candidate, probe, &groups)?);
    }
    let report = CompositionReport {
        schema_version: 1,
        candidate_count: cases.len(),
        candidates_sha256: sha256(&candidates_bytes),
        probe_sha256: sha256(&probe_bytes),
        opcode_coverage_sha256: sha256(&coverage_bytes),
        cases,
    };
    let output = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
    let path = directory.join("instruction_composition.json");
    fs::write(&path, output).map_err(|error| error.to_string())?;
    println!(
        "Instruction composition: {} cases, {}",
        report.candidate_count,
        path.display()
    );
    Ok(())
}
