//! Shared runner for hand-written M68000 assembly benchmarks.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use cpu::Cpu;
use physical_memory::{A500Config, A500Preset, PhysicalMemory, VideoStandard, MAX_FAST_RAM_SIZE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(windows)]
use super::platform::windows::AffinityGuard;
use super::platform::HostEnvironment;

const CASE_ROOT: &str = "benchmarks/m68k";
const CONFIG_PATH: &str = "benchmarks/m68k/config.json";
const ASSEMBLER_MANIFEST: &str = "tools/bootstrap/assembler-manifest.json";
const FAST_RAM_BASE: u32 = 0x200000;
const COUNT_PLACEHOLDER: &str = "{{instruction_count}}";
const END_MARKER: &[u8] = b"M68K_END_MARKER!";
const RANDOM_WORD_SEED: u32 = 0x1234_5678;

#[derive(Clone, Deserialize, Serialize)]
struct BenchmarkConfig {
    instruction_count: usize,
    instruction_limit: usize,
    series: usize,
    target_series_ms: u64,
}

#[derive(Clone, Deserialize, Serialize)]
struct CaseConfig {
    origin: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_sr: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stack_reserve_bytes: Option<usize>,
    target_primary_opcode: String,
    #[serde(default = "one")]
    instructions_per_loop: usize,
    initial_registers: BTreeMap<String, u32>,
    memory_regions: Vec<MemoryRegionConfig>,
}

fn one() -> usize {
    1
}

#[derive(Clone, Deserialize, Serialize)]
struct MemoryRegionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<ExceptionVector>,
    fill: MemoryFill,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ExceptionVector {
    AddressError,
    ZeroDivide,
    Chk,
    PrivilegeViolation,
}

impl ExceptionVector {
    fn address(self) -> u32 {
        match self {
            Self::AddressError => cpu::vector::addr(cpu::vector::ADDRESS_ERROR),
            Self::ZeroDivide => cpu::vector::addr(cpu::vector::ZERO_DIVIDE),
            Self::Chk => cpu::vector::addr(cpu::vector::CHK),
            Self::PrivilegeViolation => cpu::vector::addr(cpu::vector::PRIVILEGE_VIOLATION),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum MemoryFill {
    RandomWords { count: usize },
    RandomBcdWords { count: usize },
    FixedWords { values: Vec<u16> },
    LabelAddress { label: String },
}

#[derive(Deserialize)]
struct AssemblerManifest {
    release: String,
    platforms: HashMap<String, AssemblerPlatform>,
}

#[derive(Deserialize)]
struct AssemblerPlatform {
    executable_name: String,
    executable_sha256: String,
}

struct Assembler {
    path: PathBuf,
    release: String,
    sha256: String,
}

#[derive(Serialize)]
struct Sample {
    runs: u64,
    target_instruction_count: u64,
    total_instruction_count: u64,
    elapsed_ns: u128,
    wall_elapsed_ns: u128,
    ns_per_target: f64,
}

#[derive(Serialize)]
struct ProgramResult {
    target_instructions_per_run: usize,
    total_instructions_per_run: usize,
    helper_instructions_per_run: usize,
    image_bytes: usize,
    clocks_per_run: u64,
    median_ns_per_target: f64,
    range_pct: f64,
    samples: Vec<Sample>,
}

#[derive(Serialize)]
struct Report {
    case_id: String,
    case_config: CaseConfig,
    benchmark_config: BenchmarkConfig,
    assembler_release: String,
    assembler_sha256: String,
    host_os: &'static str,
    host_arch: &'static str,
    affinity_core: Option<usize>,
    priority_class: Option<u32>,
    source_sha256: String,
    assembly_source_sha256: String,
    image_sha256: String,
    origin: u32,
    entry: u32,
    program_end: u32,
    results: Vec<ProgramResult>,
}

struct PreparedRegion {
    label: String,
    address: u32,
    words: Vec<u16>,
}

struct Fixture {
    initial_cpu: Cpu,
    initial_bus: PhysicalMemory,
    image: Vec<u8>,
    origin: u32,
    program_end: u32,
    instruction_limit: usize,
    target_count: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ExecutionCounts {
    clocks: u64,
    targets: usize,
    total: usize,
}

struct ProgramMeasurement {
    counts: ExecutionCounts,
    cpu: Cpu,
    bus: PhysicalMemory,
    samples: Vec<Sample>,
}

fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "could not locate repository root".to_owned())
}

fn case_directory(root: &Path, case_id: &str) -> Result<PathBuf, String> {
    let components: Vec<&str> = case_id.split('/').collect();
    if components.len() != 2
        || components.iter().any(|component| {
            component.is_empty()
                || !component
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
    {
        return Err(
            "case ID must be <family>/<case_name> using lowercase letters, digits, and underscores"
                .to_owned(),
        );
    }
    Ok(root.join(CASE_ROOT).join(components[0]).join(components[1]))
}

fn assembler(root: &Path) -> Result<Assembler, String> {
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        (os, arch) => return Err(format!("no pinned assembler for {os}/{arch}")),
    };
    let manifest_path = root.join(ASSEMBLER_MANIFEST);
    let manifest: AssemblerManifest =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| format!("invalid assembler manifest: {error}"))?;
    let platform_manifest = manifest
        .platforms
        .get(platform)
        .ok_or_else(|| format!("assembler manifest has no entry for {platform}"))?;
    if manifest.release.is_empty()
        || platform_manifest.executable_name.is_empty()
        || platform_manifest.executable_sha256.len() != 64
    {
        return Err(format!("invalid assembler manifest entry for {platform}"));
    }
    let path = root
        .join("tools/m68k-asm")
        .join(&manifest.release)
        .join(platform)
        .join(&platform_manifest.executable_name);
    if !path.is_file() {
        return Err(format!(
            "assembler missing at {}; run tools/bootstrap/bootstrap.ps1 -Assembler",
            path.display()
        ));
    }
    let actual_hash = sha256(&fs::read(&path).map_err(|error| error.to_string())?);
    if !actual_hash.eq_ignore_ascii_case(&platform_manifest.executable_sha256) {
        return Err(format!(
            "assembler SHA-256 mismatch at {}; run tools/bootstrap/bootstrap.ps1 -Assembler",
            path.display()
        ));
    }
    Ok(Assembler {
        path,
        release: manifest.release,
        sha256: actual_hash,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn symbol_address(symbols: &str, name: &str) -> Result<Option<u32>, String> {
    for line in symbols.lines() {
        let Some((symbol, value)) = line.split_once('=') else {
            continue;
        };
        if symbol.trim() == name {
            let digits = value
                .trim()
                .strip_prefix('$')
                .ok_or_else(|| format!("invalid address for {name}"))?;
            return u32::from_str_radix(digits, 16)
                .map(Some)
                .map_err(|error| error.to_string());
        }
    }
    Ok(None)
}

fn next_random_word(state: &mut u32) -> u16 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 16) as u16
}

fn random_words(count: usize) -> Vec<u16> {
    let mut state = RANDOM_WORD_SEED;
    (0..count).map(|_| next_random_word(&mut state)).collect()
}

fn random_bcd_byte(state: &mut u32) -> u16 {
    let decimal = next_random_word(state) % 100;
    ((decimal / 10) << 4) | (decimal % 10)
}

fn random_bcd_words(count: usize) -> Vec<u16> {
    let mut state = RANDOM_WORD_SEED;
    (0..count)
        .map(|_| (random_bcd_byte(&mut state) << 8) | random_bcd_byte(&mut state))
        .collect()
}

fn word_data_source(words: &[u16]) -> Result<String, String> {
    let mut source = String::with_capacity(words.len() * 7);
    for chunk in words.chunks(16) {
        source.push_str("    dc.w ");
        for (index, word) in chunk.iter().enumerate() {
            if index > 0 {
                source.push(',');
            }
            write!(&mut source, "${word:04X}").map_err(|error| error.to_string())?;
        }
        source.push('\n');
    }
    Ok(source)
}

fn data_register_index(name: &str) -> Result<usize, String> {
    let bytes = name.as_bytes();
    if bytes.len() == 2 && bytes[0] == b'd' && (b'0'..=b'7').contains(&bytes[1]) {
        Ok(usize::from(bytes[1] - b'0'))
    } else {
        Err(format!("invalid data register: {name}"))
    }
}

fn address_register_index(name: &str) -> Result<usize, String> {
    let bytes = name.as_bytes();
    if bytes.len() == 2 && bytes[0] == b'a' && (b'0'..=b'6').contains(&bytes[1]) {
        Ok(usize::from(bytes[1] - b'0'))
    } else {
        Err(format!(
            "invalid address register: {name}; A7 needs explicit stack-bank setup"
        ))
    }
}

fn initial_cpu(case: &CaseConfig) -> Result<Cpu, String> {
    let mut cpu = Cpu::new();
    if let Some(initial_sr) = case.initial_sr {
        cpu.state.set_sr(initial_sr);
    }
    for (name, value) in &case.initial_registers {
        if name == "ssp" {
            cpu.state.set_ssp(*value);
        } else if name == "usp" {
            cpu.state.set_usp(*value);
        } else if name.starts_with('d') {
            cpu.state.set_d_long(data_register_index(name)?, *value);
        } else {
            cpu.state.set_a_long(address_register_index(name)?, *value);
        }
    }
    Ok(cpu)
}

fn prepare(
    root: &Path,
    case_id: &str,
    benchmark: &BenchmarkConfig,
    case: &CaseConfig,
    output: &Path,
) -> Result<(Fixture, Report), String> {
    if benchmark.instruction_count == 0 || benchmark.series < 3 || benchmark.target_series_ms == 0 {
        return Err("invalid M68000 benchmark configuration".to_owned());
    }
    if benchmark.instruction_limit < benchmark.instruction_count {
        return Err("instruction limit is below the target instruction count".to_owned());
    }
    if case.instructions_per_loop == 0
        || benchmark.instruction_count % case.instructions_per_loop != 0
    {
        return Err("instruction count must divide evenly by instructions per loop".to_owned());
    }
    if case.origin < FAST_RAM_BASE || case.origin & 1 != 0 {
        return Err("assembly image must start at an even Fast RAM address".to_owned());
    }
    let stack_reserve_bytes = case.stack_reserve_bytes.unwrap_or(0);
    if stack_reserve_bytes != 0
        && (stack_reserve_bytes < 16
            || stack_reserve_bytes % 4 != 0
            || stack_reserve_bytes > MAX_FAST_RAM_SIZE)
    {
        return Err("invalid stack reserve".to_owned());
    }
    let start = (case.origin - FAST_RAM_BASE) as usize;
    let available_bytes = MAX_FAST_RAM_SIZE - stack_reserve_bytes;
    if start >= available_bytes {
        return Err("assembly origin leaves no Fast RAM for the image".to_owned());
    }
    if case.target_primary_opcode.len() != 4 {
        return Err("target primary opcode must have four hexadecimal digits".to_owned());
    }
    u16::from_str_radix(&case.target_primary_opcode, 16)
        .map_err(|error| format!("invalid target primary opcode: {error}"))?;
    let case_dir = case_directory(root, case_id)?;
    let source = case_dir.join("program.s");
    let source_text = fs::read_to_string(&source).map_err(|error| error.to_string())?;
    if source_text.matches(COUNT_PLACEHOLDER).count() != 1 {
        return Err("assembly source must contain one instruction_count marker".to_owned());
    }
    let repetitions = benchmark.instruction_count / case.instructions_per_loop;
    let mut assembly_source = source_text.replace(COUNT_PLACEHOLDER, &repetitions.to_string());
    let mut labels = HashSet::new();
    let mut regions = Vec::with_capacity(case.memory_regions.len());
    for region in &case.memory_regions {
        match &region.fill {
            MemoryFill::RandomWords { count } | MemoryFill::RandomBcdWords { count } => {
                let address = region
                    .address
                    .ok_or("random word region needs an address")?;
                if region.vector.is_some() {
                    return Err("random word region cannot name an exception vector".to_owned());
                }
                let label = region
                    .label
                    .as_deref()
                    .ok_or("random word region needs a label")?;
                if label.is_empty()
                    || !label.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
                    || label == "instruction_count"
                    || !labels.insert(label)
                {
                    return Err(format!("invalid or repeated memory region label: {label}"));
                }
                let marker = format!("{{{{{label}}}}}");
                if assembly_source.matches(&marker).count() != 1 {
                    return Err(format!("assembly source must contain one {marker} marker"));
                }
                if *count == 0 || *count > (available_bytes - start) / 2 {
                    return Err(format!("memory region is too large: {label}"));
                }
                let words = if matches!(&region.fill, MemoryFill::RandomBcdWords { .. }) {
                    random_bcd_words(*count)
                } else {
                    random_words(*count)
                };
                assembly_source = assembly_source.replace(&marker, &word_data_source(&words)?);
                regions.push(PreparedRegion {
                    label: label.to_owned(),
                    address,
                    words,
                });
            }
            MemoryFill::FixedWords { values } => {
                if region.label.is_some()
                    || region.vector.is_some()
                    || region.address.is_none()
                    || values.is_empty()
                {
                    return Err(
                        "fixed word region needs an address and values, but no label or vector"
                            .to_owned(),
                    );
                }
            }
            MemoryFill::LabelAddress { label } => {
                if region.label.is_some()
                    || region.address.is_some()
                    || region.vector.is_none()
                    || label.is_empty()
                {
                    return Err(
                        "label address region needs a vector and a target label, but no address"
                            .to_owned(),
                    );
                }
            }
        }
    }
    let assembly_path = output.join("program.assembled.s");
    fs::write(&assembly_path, &assembly_source).map_err(|error| error.to_string())?;
    let image_path = output.join("program.bin");
    let symbols_path = output.join("program.sym");
    let assembler = assembler(root)?;
    let assembled = Command::new(&assembler.path)
        .arg(&assembly_path)
        .args([
            "--origin",
            &format!("0x{:X}", case.origin),
            "--cpu",
            "68000",
        ])
        .arg("-o")
        .arg(&image_path)
        .arg("--sym")
        .arg(&symbols_path)
        .output()
        .map_err(|error| error.to_string())?;
    if !assembled.status.success() {
        return Err(format!(
            "assembler failed: {}",
            String::from_utf8_lossy(&assembled.stderr)
        ));
    }
    let image = fs::read(&image_path).map_err(|error| error.to_string())?;
    if image.is_empty() || image.len() > available_bytes - start {
        return Err("assembled image exceeds the available Fast RAM".to_owned());
    }
    let image_end = case.origin + image.len() as u32;
    let symbols = fs::read_to_string(&symbols_path).map_err(|error| error.to_string())?;
    let entry =
        symbol_address(&symbols, "entry")?.ok_or("assembler did not export symbol entry")?;
    let mut marker_matches = image
        .windows(END_MARKER.len())
        .enumerate()
        .filter(|(_, bytes)| *bytes == END_MARKER);
    let (marker_offset, _) = marker_matches
        .next()
        .ok_or("assembled image has no end marker")?;
    if marker_matches.next().is_some() {
        return Err("assembled image contains multiple end markers".to_owned());
    }
    let program_end = case.origin + marker_offset as u32;
    if entry < case.origin
        || entry >= image_end
        || program_end <= entry
        || entry & 1 != 0
        || program_end & 1 != 0
    {
        return Err("assembled program bounds are invalid".to_owned());
    }
    for region in &regions {
        let address = symbol_address(&symbols, &region.label)?
            .ok_or_else(|| format!("assembler did not export symbol {}", region.label))?;
        let bytes = region.words.len() * 2;
        let region_end = address
            .checked_add(bytes as u32)
            .ok_or("memory region exceeds the address space")?;
        if address != region.address
            || address & 1 != 0
            || address < case.origin
            || region_end > image_end
        {
            return Err(format!(
                "assembled memory region has wrong bounds: {}",
                region.label
            ));
        }
        let offset = (address - case.origin) as usize;
        if image[offset..offset + bytes]
            .chunks_exact(2)
            .zip(region.words.iter())
            .any(|(stored, expected)| u16::from_be_bytes([stored[0], stored[1]]) != *expected)
        {
            return Err(format!(
                "assembled memory region differs from its fill: {}",
                region.label
            ));
        }
    }

    let mut bus = PhysicalMemory::from_config(A500Config::from_preset(
        A500Preset::ExpandedPowerUser,
        VideoStandard::Pal,
    ));
    bus.map_chip_ram_to_low_memory();
    let fast_ram = bus.fast_ram.as_mut().ok_or("Fast RAM is not installed")?;
    fast_ram[start..start + image.len()].copy_from_slice(&image);
    for region in &case.memory_regions {
        let (region_address, values) = match &region.fill {
            MemoryFill::RandomWords { .. } | MemoryFill::RandomBcdWords { .. } => continue,
            MemoryFill::FixedWords { values } => (
                region.address.ok_or("fixed word region needs an address")?,
                values.clone(),
            ),
            MemoryFill::LabelAddress { label } => {
                let target = symbol_address(&symbols, label)?
                    .ok_or_else(|| format!("assembler did not export symbol {label}"))?;
                if target < case.origin || target >= program_end || target & 1 != 0 {
                    return Err(format!(
                        "invalid handler address for {label}: ${target:06X}"
                    ));
                }
                (
                    region
                        .vector
                        .ok_or("label address region needs a vector")?
                        .address(),
                    vec![(target >> 16) as u16, target as u16],
                )
            }
        };
        let address = region_address as usize;
        let end = values
            .len()
            .checked_mul(2)
            .and_then(|bytes| address.checked_add(bytes))
            .ok_or("fixed word region exceeds the address space")?;
        let in_chip_ram = end <= bus.chip_ram.len();
        let in_fast_ram = address >= FAST_RAM_BASE as usize
            && bus
                .fast_ram
                .as_ref()
                .is_some_and(|ram| end <= FAST_RAM_BASE as usize + ram.len());
        let in_slow_ram = address >= 0xC00000
            && bus
                .slow_ram
                .as_ref()
                .is_some_and(|ram| end <= 0xC00000 + ram.len());
        if region_address & 1 != 0 || !(in_chip_ram || in_fast_ram || in_slow_ram) {
            return Err(format!(
                "memory region is outside RAM: ${region_address:06X}"
            ));
        }
        if address < image_end as usize && end > case.origin as usize {
            return Err(format!(
                "memory region overlaps the image: ${region_address:06X}"
            ));
        }
        for (index, value) in values.iter().enumerate() {
            let word_address = region_address + (index * 2) as u32;
            bus.write_word_debug(word_address, *value);
            if bus.read_word_debug(word_address) != *value {
                return Err(format!(
                    "memory word is not CPU-visible: ${word_address:06X}"
                ));
            }
        }
    }
    let mut cpu = initial_cpu(case)?;
    cpu.set_pc_and_prime_prefetch(entry, &mut bus);

    let fixture = Fixture {
        initial_cpu: cpu,
        initial_bus: bus,
        image,
        origin: case.origin,
        program_end,
        instruction_limit: benchmark.instruction_limit,
        target_count: benchmark.instruction_count,
    };
    let report = Report {
        case_id: case_id.to_owned(),
        case_config: case.clone(),
        benchmark_config: benchmark.clone(),
        assembler_release: assembler.release,
        assembler_sha256: assembler.sha256,
        host_os: std::env::consts::OS,
        host_arch: std::env::consts::ARCH,
        affinity_core: None,
        priority_class: None,
        source_sha256: sha256(source_text.as_bytes()),
        assembly_source_sha256: sha256(assembly_source.as_bytes()),
        image_sha256: sha256(&fixture.image),
        origin: case.origin,
        entry,
        program_end,
        results: Vec::new(),
    };
    Ok((fixture, report))
}

fn execute_program(
    fixture: &Fixture,
    cpu: &mut Cpu,
    bus: &mut PhysicalMemory,
) -> Result<ExecutionCounts, String> {
    let mut clocks = 0u64;
    let mut total = 0usize;
    while cpu.state.instruction_pc() != fixture.program_end {
        let pc = cpu.state.instruction_pc();
        if pc < fixture.origin || pc >= fixture.program_end {
            return Err(format!("execution left the program at ${pc:06X}"));
        }
        if total >= fixture.instruction_limit {
            return Err("program did not reach its end before the instruction limit".to_owned());
        }
        let step_clocks = cpu.step_instruction(bus);
        if step_clocks == 0 {
            return Err("CPU stopped before program end".to_owned());
        }
        clocks = clocks
            .checked_add(u64::from(step_clocks))
            .ok_or("CPU clock count overflowed")?;
        total += 1;
    }
    if total < fixture.target_count {
        return Err("executed fewer instructions than configured targets".to_owned());
    }
    Ok(ExecutionCounts {
        clocks,
        targets: fixture.target_count,
        total,
    })
}

fn inspect_program(fixture: &Fixture) -> Result<ExecutionCounts, String> {
    let mut cpu = fixture.initial_cpu.clone();
    let mut bus = fixture.initial_bus.clone();
    execute_program(fixture, &mut cpu, &mut bus)
}

fn run_program(
    fixture: &Fixture,
    inspected: &ExecutionCounts,
    cpu: &mut Cpu,
    bus: &mut PhysicalMemory,
) -> Result<u128, String> {
    cpu.clone_from(&fixture.initial_cpu);
    bus.clone_from(&fixture.initial_bus);
    let start = Instant::now();
    let counts = execute_program(fixture, cpu, bus)?;
    let elapsed = start.elapsed().as_nanos();
    if counts.clocks != inspected.clocks || counts.total != inspected.total {
        return Err("timed run changed its instruction or clock count".to_owned());
    }
    Ok(elapsed)
}

fn measure_series(
    fixture: &Fixture,
    measurement: &mut ProgramMeasurement,
    target_ns: u128,
) -> Result<(), String> {
    let wall_start = Instant::now();
    let mut elapsed_ns = 0u128;
    let mut runs = 0u64;
    loop {
        elapsed_ns += run_program(
            fixture,
            &measurement.counts,
            &mut measurement.cpu,
            &mut measurement.bus,
        )?;
        runs += 1;
        if elapsed_ns >= target_ns {
            break;
        }
    }
    let wall_elapsed_ns = wall_start.elapsed().as_nanos();
    let target_instruction_count = runs * measurement.counts.targets as u64;
    let total_instruction_count = runs * measurement.counts.total as u64;
    measurement.samples.push(Sample {
        runs,
        target_instruction_count,
        total_instruction_count,
        elapsed_ns,
        wall_elapsed_ns,
        ns_per_target: elapsed_ns as f64 / target_instruction_count as f64,
    });
    Ok(())
}

fn finish_program(measurement: ProgramMeasurement, fixture: &Fixture) -> ProgramResult {
    let mut sorted: Vec<f64> = measurement
        .samples
        .iter()
        .map(|sample| sample.ns_per_target)
        .collect();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    let range_pct = (sorted[sorted.len() - 1] - sorted[0]) * 100.0 / median;
    ProgramResult {
        target_instructions_per_run: measurement.counts.targets,
        total_instructions_per_run: measurement.counts.total,
        helper_instructions_per_run: measurement.counts.total - measurement.counts.targets,
        image_bytes: fixture.image.len(),
        clocks_per_run: measurement.counts.clocks,
        median_ns_per_target: median,
        range_pct,
        samples: measurement.samples,
    }
}

pub fn measure(case_id: &str) -> Result<(), String> {
    let root = repository_root()?;
    let case_dir = case_directory(&root, case_id)?;
    let host = HostEnvironment::detect();
    #[cfg(windows)]
    let affinity = AffinityGuard::pin(host.p_core_id.ok_or("no performance core selected")?)?;
    let config_path = root.join(CONFIG_PATH);
    let benchmark: BenchmarkConfig =
        serde_json::from_slice(&fs::read(config_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let case_path = case_dir.join("case.json");
    let case: CaseConfig =
        serde_json::from_slice(&fs::read(case_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let output = root.join(".agent/tmp/benchmarks/handwritten").join(case_id);
    fs::create_dir_all(&output).map_err(|error| error.to_string())?;
    let (fixture, mut report) = prepare(&root, case_id, &benchmark, &case, &output)?;
    #[cfg(windows)]
    {
        report.affinity_core = Some(affinity.core);
        report.priority_class = Some(affinity.priority_class);
    }
    #[cfg(not(windows))]
    {
        let _ = host;
    }
    let counts = inspect_program(&fixture)?;
    let mut measurement = ProgramMeasurement {
        counts,
        cpu: fixture.initial_cpu.clone(),
        bus: fixture.initial_bus.clone(),
        samples: Vec::with_capacity(benchmark.series),
    };
    let target_ns = u128::from(benchmark.target_series_ms) * 1_000_000;
    for _ in 0..benchmark.series {
        measure_series(&fixture, &mut measurement, target_ns)?;
    }
    let result = finish_program(measurement, &fixture);
    println!(
        "{case_id}: {} targets/run, {:.3} ns/target median, {:.2}% range",
        result.target_instructions_per_run, result.median_ns_per_target, result.range_pct
    );
    report.results.push(result);
    let report_path = output.join("report.json");
    let json = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    fs::write(&report_path, json).map_err(|error| error.to_string())?;
    println!("ASM benchmark report: {}", report_path.display());
    Ok(())
}
