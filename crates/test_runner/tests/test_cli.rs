#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "../src/cli.rs"]
mod cli;

use cli::{parse, Command};
use std::process::Command as Process;
use test_runner::benchmark::BenchmarkProfile;

fn sentence(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn test_sentence_configs_preserve_execution_settings() {
    let Command::Bench(config) = parse(&sentence(
        "bench passes 9 quick filter arith_mul unroll 500 output reports pin off",
    ))
    .unwrap() else {
        panic!("Expected benchmark command")
    };
    assert_eq!(config.profile, BenchmarkProfile::Quick);
    assert_eq!(
        config.iterations,
        BenchmarkProfile::Quick.default_iterations()
    );
    assert_eq!(config.passes, 9);
    assert_eq!(config.unroll_k, 500);
    assert_eq!(config.filter.as_deref(), Some("arith_mul"));
    assert_eq!(config.out_dir.to_str(), Some("reports"));
    assert!(!config.pin_core);
}

#[test]
fn test_default_configs_and_report_commands() {
    assert!(matches!(parse(&[]), Ok(Command::Help)));
    assert!(matches!(parse(&sentence("summary")), Ok(Command::Summary)));
    assert!(matches!(parse(&sentence("diff")), Ok(Command::Diff)));
    let Command::Suite(opcode) = parse(&sentence("suite ADD.b")).unwrap() else {
        panic!("Expected opcode command")
    };
    assert_eq!(opcode, "ADD.b");
    let Command::Bench(config) = parse(&sentence("bench")).unwrap() else {
        panic!("Expected default benchmark")
    };
    assert_eq!(config.profile, BenchmarkProfile::Standard);
    assert_eq!(config.passes, BenchmarkProfile::Standard.default_passes());
    assert!(config.pin_core);
}

#[test]
fn test_invalid_sentences_fail_before_execution() {
    for words in [
        "summary extra",
        "suite",
        "suite ADD.b extra",
        "--summary",
        "bench --quick",
        "bench quick thorough",
        "bench quick quick",
        "bench passes",
        "bench passes 0",
        "bench unroll invalid",
        "bench pin maybe",
        "bench unknown 10",
    ] {
        assert!(
            parse(&sentence(words)).is_err(),
            "Accepted invalid command: {words}"
        );
    }
    let binary = env!("CARGO_BIN_EXE_test_runner");
    let error = Process::new(binary)
        .args(["bench", "passes", "invalid"])
        .output()
        .unwrap();
    assert_eq!(error.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&error.stderr).contains("positive integer"));
    assert!(error.stdout.is_empty());
    let help = Process::new(binary).output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains(cli::USAGE));
}
