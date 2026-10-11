#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::reporter::{resolve_results_dir, SuiteResult};
use common::singlestep_runner::{run_single_test_detail, run_test_file, run_test_file_filtered};
use common::singlestep_vectors::load_test_vectors;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source_cases() -> Vec<Value> {
    let source = repo_root().join("ref_src/SingleStepTests-680x0/68000/v1/ASL.b.json");
    let cases: Vec<Value> = serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
    vec![cases[0].clone(), cases[1582].clone(), cases[1760].clone()]
}

fn fixture(name: &str, cases: &[Value], gzip: bool) -> PathBuf {
    let directory = repo_root()
        .join(".agent/tmp/singlestep-override-contracts")
        .join(std::process::id().to_string())
        .join(name);
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(if gzip { "ASL.b.json.gz" } else { "ASL.b.json" });
    let bytes = serde_json::to_vec(cases).unwrap();
    if gzip {
        let mut encoder = GzEncoder::new(fs::File::create(&path).unwrap(), Compression::default());
        encoder.write_all(&bytes).unwrap();
        encoder.finish().unwrap();
    } else {
        fs::write(&path, bytes).unwrap();
    }
    path
}

#[test]
fn test_overrides_change_only_approved_fields_and_execute() {
    let mut expected = source_cases();
    let source = fixture("approved", &expected, false);
    expected[1]["final"]["d2"] = Value::from(0xCDFB7FF8_u32);
    expected[1]["final"]["sr"] = Value::from(0x270A_u16);
    expected[2]["final"]["d2"] = Value::from(0x417C7EF4_u32);
    let loaded = load_test_vectors(&source).unwrap();
    assert_eq!(
        serde_json::to_value(&loaded.tests).unwrap(),
        Value::from(expected)
    );
    assert_eq!(loaded.overridden_test_names.len(), 2);
    for test in &loaded.tests {
        run_single_test_detail(test, source.to_str().unwrap(), 0).unwrap();
    }
}

#[test]
fn test_override_rejects_changed_source() {
    let mut cases = source_cases();
    cases[1]["initial"]["d0"] = Value::from(0_u32);
    let error = load_test_vectors(&fixture("changed", &cases, false)).unwrap_err();
    assert!(error.to_string().contains("Override source changed"));
}

#[test]
fn test_override_rejects_missing_or_ambiguous_target() {
    let mut cases = source_cases();
    cases.remove(1);
    let error = load_test_vectors(&fixture("missing", &cases, false)).unwrap_err();
    assert!(error.to_string().contains("Override target missing"));
    let mut cases = source_cases();
    cases.push(cases[1].clone());
    let error = load_test_vectors(&fixture("ambiguous", &cases, false)).unwrap_err();
    assert!(error.to_string().contains("Override target is ambiguous"));
}

#[test]
fn test_plain_and_gzip_apply_identical_overrides() {
    let cases = source_cases();
    let plain = load_test_vectors(&fixture("plain", &cases, false)).unwrap();
    let gzip = load_test_vectors(&fixture("gzip", &cases, true)).unwrap();
    assert_eq!(
        serde_json::to_value(plain.tests).unwrap(),
        serde_json::to_value(gzip.tests).unwrap()
    );
    assert_eq!(plain.overridden_test_names, gzip.overridden_test_names);
}

#[test]
fn test_filtered_and_sampled_override_execution() {
    let cases = source_cases();
    let source = fixture("selection", &cases, false);
    assert_eq!(
        run_test_file(source.to_str().unwrap(), Some(1)).unwrap(),
        (1, 0)
    );
    let result_path = resolve_results_dir().join("latest/Real68k__ASL.b.json");
    let sampled: SuiteResult = serde_json::from_slice(&fs::read(&result_path).unwrap()).unwrap();
    assert_eq!(sampled.total_executed, 1);
    assert!(sampled.overridden_test_names.is_empty());
    assert_eq!(
        run_test_file_filtered(source.to_str().unwrap(), None, |test| test
            .name
            .ends_with("1583")
            || test.name.ends_with("1761"))
        .unwrap(),
        (2, 0)
    );
    let filtered: SuiteResult = serde_json::from_slice(&fs::read(result_path).unwrap()).unwrap();
    assert_eq!(filtered.total_executed, 2);
    assert_eq!(filtered.overridden_test_names.len(), 2);
}
