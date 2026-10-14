//! Decoder-derived primary-opcode inventory for benchmark coverage work.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use cpu::{micro::STEPS_DECODE_EXCEPTION, OPCODE_DESCRIPTOR_TABLE};

use super::catalog::all_benchmark_specs;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct InventoryCounts {
    pub legal_encodings: usize,
    pub decode_exceptions: usize,
    pub declared_encodings: usize,
    pub undeclared_legal_encodings: usize,
    pub catalog_cases: usize,
    pub catalog_decode_exceptions: usize,
    pub structural_groups: usize,
}

pub fn decoder_inventory() -> (InventoryCounts, BTreeMap<u16, Vec<&'static str>>) {
    let mut declared: BTreeMap<u16, Vec<&'static str>> = BTreeMap::new();
    let mut counts = InventoryCounts::default();
    for spec in all_benchmark_specs() {
        counts.catalog_cases += 1;
        if let Some(&opcode) = spec.opcode_words.first() {
            declared.entry(opcode).or_default().push(spec.id);
        }
    }
    for (opcode, descriptor) in OPCODE_DESCRIPTOR_TABLE.iter().enumerate() {
        if std::ptr::eq(descriptor.steps, &STEPS_DECODE_EXCEPTION[..]) {
            counts.decode_exceptions += 1;
            if declared.contains_key(&(opcode as u16)) {
                counts.catalog_decode_exceptions += 1;
            }
        } else {
            counts.legal_encodings += 1;
            if declared.contains_key(&(opcode as u16)) {
                counts.declared_encodings += 1;
            } else {
                counts.undeclared_legal_encodings += 1;
            }
        }
    }
    (counts, declared)
}

pub fn write_decoder_inventory(out_dir: &Path) -> Result<InventoryCounts, String> {
    fs::create_dir_all(out_dir).map_err(|error| error.to_string())?;
    let path = out_dir.join("decoder_inventory.csv");
    let file = File::create(&path).map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "opcode,decoder_status,catalog_ids,coverage_status")
        .map_err(|error| error.to_string())?;
    let (mut counts, declared) = decoder_inventory();
    let mut groups: BTreeMap<usize, Vec<u16>> = BTreeMap::new();
    for (opcode, descriptor) in OPCODE_DESCRIPTOR_TABLE.iter().enumerate() {
        let is_exception = std::ptr::eq(descriptor.steps, &STEPS_DECODE_EXCEPTION[..]);
        if !is_exception {
            groups
                .entry(descriptor.steps.as_ptr() as usize)
                .or_default()
                .push(opcode as u16);
        }
        let ids = declared.get(&(opcode as u16));
        let (decoder_status, coverage_status) = match (is_exception, ids.is_some()) {
            (true, true) => ("decode_exception", "catalog_conflict"),
            (true, false) => ("decode_exception", "excluded"),
            (false, true) => ("legal", "declared_unvalidated"),
            (false, false) => ("legal", "needs_form_classification"),
        };
        writeln!(
            writer,
            "{opcode:04X},{decoder_status},{},{coverage_status}",
            ids.map_or_else(String::new, |ids| ids.join("|"))
        )
        .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())?;
    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by_key(|opcodes| opcodes[0]);
    counts.structural_groups = groups.len();
    let group_file =
        File::create(out_dir.join("structural_groups.csv")).map_err(|error| error.to_string())?;
    let mut group_writer = BufWriter::new(group_file);
    writeln!(group_writer, "group_id,opcode_count,opcodes").map_err(|error| error.to_string())?;
    for opcodes in &groups {
        let members = opcodes
            .iter()
            .map(|opcode| format!("{opcode:04X}"))
            .collect::<Vec<_>>()
            .join("|");
        writeln!(
            group_writer,
            "{:04X},{},{}",
            opcodes[0],
            opcodes.len(),
            members
        )
        .map_err(|error| error.to_string())?;
    }
    group_writer.flush().map_err(|error| error.to_string())?;
    Ok(counts)
}
