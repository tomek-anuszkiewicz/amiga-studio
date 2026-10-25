//! Small JSON mutation pilot through the normal vector loader and CPU comparator.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::diagnostic::StateDiff;
use super::singlestep_runner::run_single_test_detail_with_bus;
use super::singlestep_vectors::load_test_vectors;
use super::test_memory_bus::TestMemoryBus;
use super::transactions::{parse_transactions, BusAccessSize, ExpectedTransaction};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub const SELECTION: &[(&str, usize, &str)] = &[
    ("NOP.json", 0, "normal"),
    ("ADD.l.json", 7, "register"),
    ("ADD.l.json", 29, "long_rmw"),
    ("ASL.w.json", 86, "indexed"),
    ("ASL.w.json", 7, "address_error"),
    ("MULS.json", 20, "signed_memory"),
    ("DIVU.json", 1, "quotient"),
    ("DIVU.json", 5744, "zero_divisor"),
    ("DIVS.json", 3, "signed_quotient"),
    ("ADDX.l.json", 0, "dual_predecrement"),
    ("ABCD.json", 0, "decimal_memory"),
];

pub struct Seed {
    pub suite: String,
    pub index: usize,
    pub path: String,
    pub raw: Value,
    pub sha256: String,
}

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn packet() -> PathBuf {
    root().join(".agent/tmp/SINGLESTEP-MUTATION-01")
}

pub fn seeds() -> Vec<Seed> {
    SELECTION
        .iter()
        .map(|&(suite, index, path)| {
            let source = root()
                .join("ref_src/SingleStepTests-680x0/68000/v1")
                .join(suite);
            let bytes = std::fs::read(&source).expect("original reference file is required");
            let raw: Vec<Value> = serde_json::from_slice(&bytes).unwrap();
            let loaded = load_test_vectors(&source).unwrap();
            assert_eq!(
                serde_json::to_value(&loaded.tests[index]).unwrap(),
                raw[index]
            );
            let opcode = raw[index]["initial"]["prefetch"][0].as_u64().unwrap();
            let mode = (opcode >> 3) & 7;
            let txs = raw[index]["transactions"].as_array().unwrap();
            match path {
                "normal" => assert_eq!(opcode, 0x4E71),
                "register" => assert_eq!(mode, 0),
                "long_rmw" => {
                    assert_eq!(mode, 5);
                    assert_eq!(txs.iter().filter(|t| t[0] == "w").count(), 2);
                }
                "indexed" => assert_eq!(mode, 6),
                "address_error" => assert!(txs.iter().any(|t| t[0] == "r" && t[3] == 12)),
                "signed_memory" => assert!((2..=6).contains(&mode)),
                "zero_divisor" => assert!(txs.iter().any(|t| t[0] == "r" && t[3] == 20)),
                "quotient" | "signed_quotient" => assert!(!txs.iter().any(|t| t[0] == "w")),
                "dual_predecrement" | "decimal_memory" => assert_ne!(opcode & 8, 0),
                _ => panic!("unknown pilot path"),
            }
            Seed {
                suite: suite.into(),
                index,
                path: path.into(),
                raw: raw[index].clone(),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            }
        })
        .collect()
}

fn category(diff: &StateDiff) -> String {
    match diff {
        StateDiff::DataRegister { reg, .. } => format!("d{reg}"),
        StateDiff::AddressRegister { reg, .. } => format!("a{reg}"),
        StateDiff::UserStackPointer { .. } => "usp".into(),
        StateDiff::SupervisorStackPointer { .. } => "ssp".into(),
        StateDiff::StatusRegister { .. } => "sr".into(),
        StateDiff::ProgramCounter { .. } => "pc".into(),
        StateDiff::Prefetch { word, .. } => format!("prefetch{word}"),
        StateDiff::CycleLength { .. } => "length".into(),
        StateDiff::TransactionCountMismatch { .. } => "transaction_count".into(),
        StateDiff::TransactionMismatch { details } => {
            let text = details.to_lowercase();
            let field = if text.contains("count mismatch") {
                "count"
            } else if text.contains("callback clock") {
                "clock"
            } else if text.contains("direction mismatch") {
                "direction"
            } else if text.contains("address mismatch") {
                "address"
            } else if text.contains("access size") {
                "size"
            } else if text.contains("duration mismatch") {
                "duration"
            } else if text.contains("data mismatch") {
                "data"
            } else if text.contains("timeline length") {
                "timeline"
            } else {
                "other"
            };
            format!("transaction_{field}")
        }
    }
}

pub struct Pilot {
    pub bus: TestMemoryBus,
    pub records: Vec<Value>,
    pub failures: Vec<String>,
}

impl Pilot {
    pub fn new() -> Self {
        std::fs::create_dir_all(packet().join("cases")).unwrap();
        Self {
            bus: TestMemoryBus::new_flat(),
            records: Vec::new(),
            failures: Vec::new(),
        }
    }

    fn execute(&mut self, seed: &Seed, raw: &Value, input_path: &str) -> Value {
        let path = packet().join("cases/current.json");
        std::fs::write(&path, serde_json::to_vec(&json!([raw])).unwrap()).unwrap();
        let loaded = match load_test_vectors(&path) {
            Ok(loaded) => loaded,
            Err(error) => {
                return json!({"status":"format_rejection", "cpu_executed":false, "diagnostics":error.to_string()})
            }
        };
        let test = &loaded.tests[0];
        let typed = serde_json::to_value(test).unwrap();
        // Unknown metadata is intentionally discarded; all schema fields must survive loading.
        for key in ["name", "initial", "final", "length", "transactions"] {
            assert_eq!(
                typed[key], raw[key],
                "JSON field did not reach the shared schema: {key}"
            );
        }
        let parsed = match parse_transactions(&test.transactions, &test.initial.ram) {
            Ok(parsed) => parsed,
            Err(error) => {
                return json!({"status":"format_rejection", "cpu_executed":false, "diagnostics":error})
            }
        };
        // Pilot seeds contain ordinary records; require exact propagation of their projection.
        assert_eq!(parsed.len(), test.transactions.len());
        for (tx, expected) in test.transactions.iter().zip(&parsed) {
            let projected = match expected {
                ExpectedTransaction::Internal { duration } => json!(["n", duration]),
                ExpectedTransaction::Bus {
                    is_read,
                    duration,
                    addr,
                    size,
                    data,
                } => json!([
                    if *is_read { "r" } else { "w" },
                    duration,
                    tx[2],
                    addr,
                    if *size == BusAccessSize::Byte {
                        ".b"
                    } else {
                        ".w"
                    },
                    data
                ]),
            };
            // FC is preserved in raw JSON but has no independent observed counterpart.
            assert_eq!(
                &projected, tx,
                "transaction value did not reach the shared parser"
            );
        }
        match run_single_test_detail_with_bus(test, &mut self.bus, input_path, seed.index) {
            Ok(()) => json!({"status":"pass", "cpu_executed":true, "categories":[]}),
            Err(failure) => json!({"status":"comparison_rejection", "cpu_executed":true,
                "categories":failure.diffs.iter().map(category).collect::<Vec<_>>(),
                "diagnostics":failure.diffs}),
        }
    }

    pub fn baseline(&mut self, seed: &Seed) {
        let outcome = self.execute(seed, &seed.raw, &seed.suite);
        assert_eq!(
            outcome["status"], "pass",
            "baseline {} #{}: {outcome}",
            seed.suite, seed.index
        );
        self.records.push(
            json!({"suite":seed.suite, "index":seed.index, "path":seed.path,
            "kind":"baseline", "outcome":outcome}),
        );
    }

    pub fn probe(
        &mut self,
        seed: &Seed,
        label: &str,
        raw: Value,
        edits: Vec<Value>,
        required: Option<&str>,
        disposition: &str,
        input_path: &str,
    ) {
        let outcome = self.execute(seed, &raw, input_path);
        let correct = match required {
            Some("format") => outcome["status"] == "format_rejection",
            Some("pass") => outcome["status"] == "pass",
            Some(category) => {
                outcome["status"] == "comparison_rejection"
                    && outcome["categories"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v == category)
            }
            None => true,
        };
        if !correct {
            self.failures.push(format!(
                "{} #{} {label}: required {required:?}, got {outcome}",
                seed.suite, seed.index
            ));
        }
        let serial = self.records.len();
        let file = format!("cases/mutation-{serial}.json");
        std::fs::write(
            packet().join(&file),
            serde_json::to_vec(&json!([raw])).unwrap(),
        )
        .unwrap();
        self.records.push(
            json!({"suite":seed.suite,"index":seed.index,"name":seed.raw["name"],
            "path":seed.path,"source_sha256":seed.sha256,"label":label,"edits":edits,
            "required_category":required,
            "enforced_here":required.is_some(),"disposition":disposition,"input_path":input_path,
            "copy":file,"outcome":outcome,
            "expectation_met":correct}),
        );
        // Run the untouched seed again on the same reusable bus to expose state leakage.
        let control = self.execute(seed, &seed.raw, &seed.suite);
        assert_eq!(
            control["status"], "pass",
            "seed leaked state after {label}: {control}"
        );
        self.records.push(
            json!({"suite":seed.suite,"index":seed.index,"kind":"untouched_control",
            "after":label,"outcome":control}),
        );
    }

    pub fn change(&mut self, seed: &Seed, path: &str, value: Value, required: &str) {
        let mut raw = seed.raw.clone();
        let old = raw.pointer(path).unwrap().clone();
        *raw.pointer_mut(path).unwrap() = value.clone();
        self.probe(
            seed,
            path,
            raw,
            vec![json!({"path":path,"old":old,"new":value})],
            Some(required),
            if required == "format" {
                "format_validation"
            } else {
                "observable_expectation"
            },
            &seed.suite,
        );
    }

    pub fn finish(&self, seeds: &[Seed]) {
        let phase = std::env::var("SINGLESTEP_MUTATION_PHASE").unwrap_or_else(|_| "final".into());
        assert!(phase
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_'));
        let identities: Vec<_> = seeds
            .iter()
            .map(|s| {
                json!({"suite":s.suite,"index":s.index,
            "name":s.raw["name"],"opcode":s.raw["initial"]["prefetch"][0],"path":s.path,
            "source_sha256":s.sha256})
            })
            .collect();
        std::fs::write(
            packet().join("seeds.json"),
            serde_json::to_vec_pretty(&identities).unwrap(),
        )
        .unwrap();
        std::fs::write(
            packet().join(format!("mutations-{phase}.json")),
            serde_json::to_vec_pretty(&self.records).unwrap(),
        )
        .unwrap();
        if std::env::var_os("SINGLESTEP_MUTATION_DISCOVERY").is_none() {
            assert!(
                self.failures.is_empty(),
                "{} failed sensitivity expectations:\n{}",
                self.failures.len(),
                self.failures.join("\n")
            );
        }
    }
}
