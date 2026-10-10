//! Sentence-style diagnostic commands shared by the binary and its CLI tests.

use std::collections::HashMap;
use std::path::PathBuf;
use test_runner::benchmark::{BenchmarkConfig, BenchmarkProfile};

pub const USAGE: &str = "M68000 Verification and Benchmark Runner
Usage: cargo run -p test_runner -- COMMAND

Commands:
  summary                         Print persisted opcode coverage results
  diff                            Compare latest and previous test results
  suite OPCODE                    Run diagnostics for one opcode (e.g. ADD.b)
  bench [quick|standard|thorough] [filter TAG] [unroll N] [passes N]
        [output PATH] [pin on|off]
  help                            Print these instructions

Examples:
  cargo run -p test_runner -- bench quick filter arith_mul passes 3

Counts must be positive. Unknown or repeated selectors are errors.";

#[derive(Debug)]
pub enum Command {
    Help,
    Summary,
    Diff,
    Suite(String),
    Bench(BenchmarkConfig),
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some((command, words)) = args.split_first() else {
        return Ok(Command::Help);
    };
    match command.as_str() {
        "help" | "--help" | "-h" if words.is_empty() => Ok(Command::Help),
        "summary" if words.is_empty() => Ok(Command::Summary),
        "diff" if words.is_empty() => Ok(Command::Diff),
        "suite" if words.len() == 1 && !words[0].trim().is_empty() => {
            Ok(Command::Suite(words[0].clone()))
        }
        "bench" => parse_bench(words).map(Command::Bench),
        _ => Err(format!(
            "Unknown command or unexpected arguments: {}",
            args.join(" ")
        )),
    }
}

fn selectors<'a>(
    words: &'a [String],
    pairs: &[&str],
    flags: &[&str],
) -> Result<HashMap<&'a str, &'a str>, String> {
    let mut values = HashMap::new();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        let key = word.as_str();
        if values.contains_key(key) {
            return Err(format!("Repeated selector: {key}"));
        }
        let value = if flags.contains(&key) {
            ""
        } else if pairs.contains(&key) {
            let value = words
                .next()
                .ok_or_else(|| format!("Missing value for {key}"))?;
            if value.trim().is_empty() || value.starts_with('-') {
                return Err(format!("Invalid value for {key}: {value}"));
            }
            value.as_str()
        } else {
            return Err(format!("Unknown selector: {key}"));
        };
        values.insert(key, value);
    }
    Ok(values)
}

fn positive_count(values: &HashMap<&str, &str>, key: &str) -> Result<Option<usize>, String> {
    values
        .get(key)
        .map(|value| {
            value
                .parse::<usize>()
                .ok()
                .filter(|count| *count > 0)
                .ok_or_else(|| format!("{key} requires a positive integer: {value}"))
        })
        .transpose()
}

fn parse_bench(words: &[String]) -> Result<BenchmarkConfig, String> {
    let values = selectors(
        words,
        &["filter", "unroll", "passes", "output", "pin"],
        &["quick", "standard", "thorough"],
    )?;
    let profiles: Vec<_> = ["quick", "standard", "thorough"]
        .into_iter()
        .filter(|profile| values.contains_key(profile))
        .collect();
    if profiles.len() > 1 {
        return Err("Choose one benchmark profile: quick, standard or thorough".to_owned());
    }
    let mut config = BenchmarkConfig::default();
    if let Some(profile) = profiles.first() {
        config.profile = match *profile {
            "quick" => BenchmarkProfile::Quick,
            "thorough" => BenchmarkProfile::Thorough,
            _ => BenchmarkProfile::Standard,
        };
        config.passes = config.profile.default_passes();
        config.iterations = config.profile.default_iterations();
    }
    if let Some(count) = positive_count(&values, "passes")? {
        config.passes = count;
    }
    if let Some(count) = positive_count(&values, "unroll")? {
        config.unroll_k = count;
    }
    if let Some(value) = values.get("filter") {
        config.filter = Some((*value).to_owned());
    }
    if let Some(value) = values.get("output") {
        config.out_dir = PathBuf::from(value);
    }
    if let Some(value) = values.get("pin") {
        config.pin_core = match *value {
            "on" => true,
            "off" => false,
            _ => return Err("Use pin on or pin off".to_owned()),
        };
    }
    Ok(config)
}
