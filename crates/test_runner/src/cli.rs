//! Sentence-style diagnostic commands shared by the binary and its CLI tests.

use std::collections::HashMap;
use std::path::PathBuf;
use test_runner::benchmark::{BenchmarkConfig, BenchmarkProfile, DEFAULT_BENCHMARK_FRAMES};
use test_runner::vamiga::{VamigaCategory, VamigaRunConfig};

pub const USAGE: &str = "M68000 SingleStepTest & vAmigaTS Verification Runner
Usage: cargo run -p test_runner -- COMMAND

Commands:
  summary                         Print persisted opcode coverage results
  diff                            Compare latest and previous test results
  suite OPCODE                    Run diagnostics for one opcode (e.g. ADD.b)
  bench [quick|standard|thorough] [filter TAG] [unroll N] [passes N]
        [output PATH] [pin on|off]
  chipset [compare|record] [frames N]
  vamiga [category NAME] [test NAME] [frames N] [limit N] [verbose] [summary]
  vamiga list deferred            List deferred capture suites
  help                            Print these instructions

Examples:
  cargo run -p test_runner -- bench quick filter arith_mul passes 3
  cargo run -p test_runner -- vamiga category Copper frames 16 limit 5
  python tools/verification/verify.py runner chipset compare frames 50

Counts must be positive. Unknown or repeated selectors are errors.
Record updates reference baselines and remains an explicit standalone operation.";

#[derive(Debug)]
pub enum Command {
    Help,
    Summary,
    Diff,
    Suite(String),
    Bench(BenchmarkConfig),
    Chipset { record: bool, frames: u32 },
    Vamiga(VamigaOptions),
}

#[derive(Debug, Default)]
pub struct VamigaOptions {
    pub(super) category: Option<VamigaCategory>,
    pub(super) test: Option<String>,
    pub(super) limit: Option<usize>,
    pub(super) list_deferred: bool,
    pub(super) config: VamigaRunConfig,
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
        "chipset" => parse_chipset(words),
        "vamiga" => parse_vamiga(words).map(Command::Vamiga),
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

fn frames(values: &HashMap<&str, &str>, default: u32) -> Result<u32, String> {
    match positive_count(values, "frames")? {
        Some(count) => u32::try_from(count).map_err(|_| "Frame count exceeds u32 range".to_owned()),
        None => Ok(default),
    }
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

fn parse_chipset(words: &[String]) -> Result<Command, String> {
    let values = selectors(words, &["frames"], &["compare", "record"])?;
    if values.contains_key("record") && values.contains_key("compare") {
        return Err("Choose chipset compare or chipset record".to_owned());
    }
    Ok(Command::Chipset {
        record: values.contains_key("record"),
        frames: frames(&values, DEFAULT_BENCHMARK_FRAMES)?,
    })
}

fn parse_vamiga(words: &[String]) -> Result<VamigaOptions, String> {
    let mut options = VamigaOptions::default();
    if words == ["list", "deferred"] {
        options.list_deferred = true;
        return Ok(options);
    }
    let values = selectors(
        words,
        &["category", "test", "frames", "limit"],
        &["verbose", "summary"],
    )?;
    if let Some(value) = values.get("category") {
        options.category = Some(
            VamigaCategory::from_str_loose(value)
                .ok_or_else(|| format!("Unknown category: {value}"))?,
        );
    }
    options.test = values.get("test").map(|value| (*value).to_owned());
    options.limit = positive_count(&values, "limit")?;
    options.config.frames_to_run = frames(&values, options.config.frames_to_run)?;
    options.config.verbose = values.contains_key("verbose");
    Ok(options)
}
