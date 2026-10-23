//! Decoder-derived primary-opcode inventory for benchmark coverage work.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use cpu::{micro::STEPS_DECODE_EXCEPTION, OPCODE_DESCRIPTOR_TABLE};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct InventoryCounts {
    pub legal_encodings: usize,
    pub decode_exceptions: usize,
    pub structural_groups: usize,
}

pub fn decoder_inventory() -> InventoryCounts {
    let mut counts = InventoryCounts::default();
    for descriptor in OPCODE_DESCRIPTOR_TABLE.iter() {
        if std::ptr::eq(descriptor.steps, &STEPS_DECODE_EXCEPTION[..]) {
            counts.decode_exceptions += 1;
        } else {
            counts.legal_encodings += 1;
        }
    }
    counts
}

pub fn write_decoder_inventory(out_dir: &Path) -> Result<InventoryCounts, String> {
    fs::create_dir_all(out_dir).map_err(|error| error.to_string())?;
    let path = out_dir.join("decoder_inventory.csv");
    let file = File::create(&path).map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "opcode,decoder_status,coverage_status").map_err(|error| error.to_string())?;
    let mut counts = decoder_inventory();
    let mut groups: BTreeMap<usize, Vec<u16>> = BTreeMap::new();
    for (opcode, descriptor) in OPCODE_DESCRIPTOR_TABLE.iter().enumerate() {
        let is_exception = std::ptr::eq(descriptor.steps, &STEPS_DECODE_EXCEPTION[..]);
        if !is_exception {
            groups
                .entry(descriptor.steps.as_ptr() as usize)
                .or_default()
                .push(opcode as u16);
        }
        let (decoder_status, coverage_status) = if is_exception {
            ("decode_exception", "excluded")
        } else {
            ("legal", "needs_form_classification")
        };
        writeln!(writer, "{opcode:04X},{decoder_status},{coverage_status}")
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
