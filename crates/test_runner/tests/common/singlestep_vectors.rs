//! Shared vector loading with reviewed corrections to exact upstream records.

use super::schema::SingleStepTest;
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

const CORRECTIONS: &str = include_str!("../overrides/singlestep/corrections.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorCorrection {
    suite: String,
    test_name: String,
    original_sha256: String,
    #[serde(rename = "final")]
    final_fields: BTreeMap<String, Value>,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug)]
pub struct LoadedVectors {
    pub tests: Vec<SingleStepTest>,
    pub overridden_test_names: Vec<String>,
}

/// Parses the reference and applies corrections before filtering or sampling.
pub fn load_test_vectors(path: &Path) -> Result<LoadedVectors, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(path)?);
    let is_gzip = path.extension().is_some_and(|extension| extension == "gz");
    let mut tests: Vec<SingleStepTest> = if is_gzip {
        serde_json::from_reader(BufReader::new(GzDecoder::new(reader)))?
    } else {
        serde_json::from_reader(reader)?
    };
    for test in &tests {
        test.validate().map_err(invalid_correction)?;
    }
    let suite = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid_correction("Vector filename is not valid UTF-8"))?
        .trim_end_matches(".gz");
    let corrections: Vec<VectorCorrection> = serde_json::from_str(CORRECTIONS)?;
    let mut overridden_test_names = Vec::new();
    for correction in corrections
        .iter()
        .filter(|correction| correction.suite == suite)
    {
        if overridden_test_names.contains(&correction.test_name) {
            return Err(invalid_correction(format!(
                "Duplicate override for {suite}: {}",
                correction.test_name
            ))
            .into());
        }
        apply_correction(&mut tests, correction)?;
        overridden_test_names.push(correction.test_name.clone());
    }
    Ok(LoadedVectors {
        tests,
        overridden_test_names,
    })
}

fn apply_correction(
    tests: &mut [SingleStepTest],
    correction: &VectorCorrection,
) -> Result<(), Box<dyn std::error::Error>> {
    if correction.reason.trim().is_empty()
        || correction.evidence.is_empty()
        || correction.final_fields.is_empty()
    {
        return Err(invalid_correction(
            "Override requires a reason, evidence and corrected fields",
        )
        .into());
    }
    let mut matches = tests
        .iter_mut()
        .filter(|test| test.name == correction.test_name);
    let test = matches.next().ok_or_else(|| {
        invalid_correction(format!("Override target missing: {}", correction.test_name))
    })?;
    if matches.next().is_some() {
        return Err(invalid_correction(format!(
            "Override target is ambiguous: {}",
            correction.test_name
        ))
        .into());
    }
    // Value objects use sorted keys; whitespace and source property order do not affect identity.
    let original = serde_json::to_vec(&serde_json::to_value(&*test)?)?;
    let actual_sha256 = format!("{:x}", Sha256::digest(original));
    if actual_sha256 != correction.original_sha256 {
        return Err(invalid_correction(format!(
            "Override source changed for {}: expected SHA-256 {}, found {}",
            correction.test_name, correction.original_sha256, actual_sha256
        ))
        .into());
    }
    let mut final_state = serde_json::to_value(&test.final_state)?;
    for (field, replacement) in &correction.final_fields {
        let original_field = final_state.get_mut(field).ok_or_else(|| {
            invalid_correction(format!("Unknown final-state field in override: {field}"))
        })?;
        *original_field = replacement.clone();
    }
    test.final_state = serde_json::from_value(final_state)?;
    Ok(())
}

fn invalid_correction(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
