"""Authoritative completion profiles and diagnostic command selection."""
import os
import sys
from pathlib import Path
from repository import REPO_ROOT


# Only the two expensive synthetic CPU suites depend on changed source paths.
# These environments use TestMemoryBus, not production MemoryBus/PhysicalMemory.
COMMON_CPU_PATHS = (
    "crates/cpu/", "crates/config/", "crates/physical_memory/src/address_bus",
    "crates/test_runner/Cargo.toml", "crates/test_runner/src/test_runner.rs",
    "crates/test_runner/src/test_memory_bus.rs", "crates/test_runner/src/schema.rs",
    "crates/test_runner/src/transactions.rs", "crates/test_runner/src/diagnostic.rs",
    "crates/test_runner/tests/common/", "ref_src/SingleStepTests-680x0/",
)
DMA_PATHS = COMMON_CPU_PATHS + (
    "crates/test_runner/src/dma_harness.rs", "crates/test_runner/tests/test_dma_cartesian.rs",
)
SINGLESTEP_PATHS = COMMON_CPU_PATHS + (
    "crates/test_runner/src/runner.rs", "crates/test_runner/src/reporter.rs",
    "crates/test_runner/tests/test_singlestep.rs",
)
WORKSPACE_INPUTS = {"Cargo.toml", "Cargo.lock", ".cargo/config.toml"}


def select_tests(root, changes, mode, verify_dma=False, verify_singlestep=False):
    import sys

    completion = mode == "commit"
    complete = completion and bool(changes & WORKSPACE_INPUTS)
    dma = complete or verify_dma or (completion and any(p.startswith(DMA_PATHS) for p in changes))
    singlestep = complete or verify_singlestep or (completion and any(p.startswith(SINGLESTEP_PATHS) for p in changes))
    commands = []
    if completion:
        commands.append(standard_harness(root))
    if dma:
        commands.append(("Cartesian DMA", ["cargo", "test", "-p", "test_runner", "--test",
                                           "test_dma_cartesian", "--", "--quiet"], {}))
    if singlestep:
        commands.append(("Full SingleStepTests", ["cargo", "test", "-p", "test_runner", "--test",
                                                  "test_singlestep", "--", "--quiet"],
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}))
    if completion:
        commands.extend(python_commands())
    return commands


def select_coverage_commands(mode):
    """Only ordinary workspace tests contribute to the commit coverage report."""
    import sys

    if mode != "commit":
        return []
    return [
        ("Prepare coverage report", [sys.executable, "tools/verification/coverage_report.py", "--prepare"], {}),
        ("Reset coverage counters", ["cargo", "llvm-cov", "clean", "--profraw-only"], {}),
        ("All ordinary crate tests", ["cargo", "llvm-cov", "--no-report", "--no-fail-fast", "--workspace",
                                      "--exclude", "test_runner", "--", "--quiet"], {}),
        ("Export ordinary crate coverage", ["cargo", "llvm-cov", "report", "--json", "--summary-only",
                                            "--ignore-filename-regex", r"[/\\]test_runner[/\\]",
                                            "--output-path", ".agent/tmp/coverage/summary.json"], {}),
        ("Ordinary crate coverage summary", [sys.executable, "tools/verification/coverage_report.py"], {}),
    ]


def select_quality_commands(mode):
    """Declare external quality commands; the verifier only runs them."""
    import sys

    def script(name, *args):
        return [sys.executable, f"tools/verification/checks/{name}.py", *args]

    if mode == "milestone":
        return [
            ("Strict English Policy", script("check_english"), {}),
        ]
    if mode == "commit":
        return [
            ("Strict English Policy", script("check_english"), {}),
            ("Formatting", ["cargo", "fmt", "--all", "--", "--check"], {}),
            ("Clippy Invariants", ["cargo", "clippy", "--workspace", "--all-targets"], {}),
            ("Architecture Rules", script("check_architecture"), {}),
            ("Hardware Quality", script("audit_hardware_quality"), {}),
            ("Code Quality", script("audit_code_quality"), {}),
        ]
    return []


TIER1_UNIT_CRATES = [
    "agnus", "audio", "blitter", "cia", "config", "copper", "denise",
    "disassembler", "dma", "floppy", "frame_builder", "game_ports", "interrupts",
    "joystick", "keyboard", "cpu", "mouse", "paula", "physical_memory", "rtc", "sprites",
]

TIER1_TEST_RUNNER_TESTS = ['test_benchmark_csv']

TIER2_INTEGRATION_CRATES = ['memory_bus', 'machine_loop', 'debugger', 'gui']

TIER3_HARNESS_TESTS = [
    ("test_runner", "test_dma_cartesian"),
    ("test_runner", "test_benchmark_smoke"),
]


def select_diagnostic_commands(unit=False, integration=False, harness=False):
    """Select the former tier runner's Cargo targets without completion gates."""
    commands = []
    if unit:
        command = ["cargo", "test"]
        for crate in TIER1_UNIT_CRATES:
            command.extend(["-p", crate])
        commands.append(("Tier 1 ordinary crate tests", command + ["--", "--quiet"], {}))
        command = ["cargo", "test", "-p", "test_runner"]
        for suite in TIER1_TEST_RUNNER_TESTS:
            command.extend(["--test", suite])
        commands.append(("Tier 1 harness fixtures", command + ["--", "--quiet"], {}))
    if integration:
        command = ["cargo", "test"]
        for crate in TIER2_INTEGRATION_CRATES:
            command.extend(["-p", crate])
        commands.append(("Tier 2 integration tests", command + ["--", "--quiet"], {}))
    if harness:
        # DMA is selected through select_tests so force selectors run once.
        command = ["cargo", "test", "-p", "test_runner"]
        for _, suite in TIER3_HARNESS_TESTS:
            if suite != "test_dma_cartesian":
                command.extend(["--test", suite])
        commands.append(("Tier 3 benchmark smoke tests", command + ["--", "--quiet"], {}))
    return commands



PYTHON_SUITES = (
    ("Bootstrap contracts", "tools/bootstrap/tests"),
    ("Conversion contracts", "tools/bootstrap/conversion/tests"),
    ("PDF conversion contracts", "tools/bootstrap/pdf-to-markdown/tests"),
    ("RAG MCP contracts", "tools/amiga-rag-mcp-server/tests"),
    ("Verification CLI contracts", "tools/verification/tests"),
)


def python_commands(directory=None, pattern=None, case=None):
    selections = PYTHON_SUITES if directory is None else [("Python contracts", directory)]
    result = []
    for name, target in selections:
        path = Path(target)
        if not path.is_absolute():
            path = REPO_ROOT / path
        if not path.is_dir():
            raise ValueError(f"Python test directory does not exist: {target}")
        command = [sys.executable, "-m", "unittest", "discover", "-s", str(target), "-q"]
        if pattern:
            command += ["-p", pattern]
        if case:
            command += ["-k", case]
        result.append((name, command, {}))
    return result


def standard_harness(root):
    suites = {p.stem for p in (root / "crates/test_runner/tests").glob("test_*.rs")}
    standard = suites - {"test_dma_cartesian", "test_singlestep"}
    command = ["cargo", "test", "--no-fail-fast", "-p", "test_runner"]
    for suite in sorted(standard):
        command += ["--test", suite]
    return "All standard harness tests", command + ["--", "--quiet"], {}


def rust_commands(words):
    from cli import keyword_values
    if not words or words in (["unit"], ["integration"], ["all"]):
        tier = words[0] if words else "all"
        return select_diagnostic_commands(unit=tier in ("unit", "all"),
                                          integration=tier in ("integration", "all"))
    if words == ["workspace"]:
        return [("Ordinary workspace tests", ["cargo", "test", "--no-fail-fast", "--workspace",
                                               "--exclude", "test_runner", "--", "--quiet"], {})]
    values = keyword_values(words, ("crate", "suite", "case"))
    if "crate" not in values:
        raise ValueError("Rust tests require crate NAME")
    command = ["cargo", "test", "-p", values["crate"]]
    if "suite" in values:
        command += ["--test", values["suite"]]
    command += ["--", "--quiet"]
    if "case" in values:
        command.append(values["case"])
    return [("Focused Rust tests", command, {})]


def harness_commands(words):
    from cli import keyword_values
    scope = words[0] if words else "standard"
    if scope == "standard" and len(words) <= 1:
        return [standard_harness(REPO_ROOT)]
    if scope == "harness" and len(words) == 1:
        return select_diagnostic_commands(harness=True) + select_tests(REPO_ROOT, set(), "standalone", verify_dma=True)
    if scope == "silicon" and len(words) == 1:
        return select_tests(REPO_ROOT, set(), "standalone", verify_dma=True, verify_singlestep=True)
    if scope not in ("singlestep", "dma"):
        raise ValueError("Runner tests select standard, harness, singlestep or dma")
    selectors = list(words[1:])
    full = bool(selectors) and selectors[-1] == "full"
    if full:
        selectors.pop()
    values = keyword_values(selectors, ("case", "sample"))
    if scope == "dma" and (full or "sample" in values):
        raise ValueError("Sampling applies only to SingleStepTests")
    if full and "sample" in values:
        raise ValueError("Choose full or sample N, not both")
    env = {}
    if scope == "singlestep":
        env = {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}
        if "sample" in values:
            count = int(values["sample"])
            if count <= 0:
                raise ValueError("Sample count must be positive")
            env = {"SINGLESTEP_FULL": "0", "SINGLESTEP_LIMIT": str(count)}
    suite = "test_singlestep" if scope == "singlestep" else "test_dma_cartesian"
    command = ["cargo", "test", "-p", "test_runner", "--test", suite, "--", "--quiet"]
    if "case" in values:
        command.append(values["case"])
    return [("SingleStepTests" if scope == "singlestep" else "Cartesian DMA", command, env)]


def check_commands(name, words):
    from cli import keyword_values
    labels = {"english": "Strict English Policy", "formatting": "Formatting", "clippy": "Clippy Invariants",
              "architecture": "Architecture Rules", "hardware": "Hardware Quality", "code": "Code Quality"}
    if name == "all" and not words:
        return select_quality_commands("commit")
    if name == "build":
        values = keyword_values(words, ("crate",))
        target = ["-p", values["crate"]] if values else ["--workspace"]
        return [("Rust compilation", ["cargo", "check", *target], {})]
    selected = [item for item in select_quality_commands("commit") if item[0] == labels.get(name)]
    if words:
        if name == "english" and words[0] == "files" and len(words) > 1:
            selected[0][1].extend(words[1:])
        elif name == "architecture":
            values = keyword_values(words, ("case",))
            selected[0][1].append(values["case"])
        else:
            raise ValueError("Only architecture accepts case FILTER; build accepts crate NAME")
    if not selected:
        raise ValueError(f"Unsupported check: {name}")
    return selected


def runner_commands(action, words):
    args = list(words)
    if action == "gui" and "--" in args:
        args.remove("--")
    if action != "gui" and any(word.startswith("-") for word in args):
        raise ValueError("Runner commands use sentence selectors; use runner bench quick passes N")
    if action == "build":
        if args:
            raise ValueError("Runner build accepts no additional arguments")
        return [("Build diagnostic runner", ["cargo", "build", "--release", "-p", "test_runner"], {})]
    if action in ("summary", "diff", "suite"):
        if (action == "suite" and len(args) != 1) or (action != "suite" and args):
            raise ValueError("Reports take no arguments; suite requires one opcode name")
        return [("Runner " + action, ["cargo", "run", "-p", "test_runner", "--", action, *args], {})]
    if action == "analyze":
        if len(args) not in (1, 3) or len(args) == 3 and args[1] != "report":
            raise ValueError("Use runner analyze CSV [report MARKDOWN]")
        command = [sys.executable, "crates/test_runner/src/analyze_benchmarks.py", args[0]]
        if len(args) == 3:
            command += ["--output", args[2]]
        return [("Benchmark analysis", command, {})]
    if action == "gui":
        return [("GUI inspector", ["cargo", "run", "-p", "gui", "--bin", "gui-inspector", "--", *args], {})]
    if action == "profile":
        if not args or args[0] != "bench":
            raise ValueError("Use runner profile bench [sentence selectors]")
        executable = "test_runner.exe" if os.name == "nt" else "test_runner"
        return [("Build profiling runner", ["cargo", "build", "--release", "-p", "test_runner"], {}),
                ("External sampling profile", ["samply", "record", "target/release/" + executable,
                                                *args], {})]
    return [("Runner " + action, ["cargo", "run", "--release", "-p", "test_runner", "--", action, *args], {})]


def select_phases(args, changes):
    from cli import keyword_values
    if args.command in ("commit", "milestone"):
        if len(set(args.sets)) != len(args.sets) or set(args.sets) - {"singlestep", "dma"}:
            raise ValueError("Completion profiles accept each of singlestep and dma at most once")
        coverage = select_coverage_commands(args.command)
        commands = select_quality_commands(args.command) + select_tests(
            REPO_ROOT, changes, args.command, verify_dma="dma" in args.sets, verify_singlestep="singlestep" in args.sets)
        phases = []
        if coverage:
            phases.append(("Coverage", coverage))
        phases.append(("Verification", commands))
        return phases
    if args.command == "check":
        selected = check_commands(args.name, args.words)
    elif args.command == "runner":
        selected = runner_commands(args.action, args.words)
    elif args.kind == "rust":
        selected = rust_commands(args.words)
    elif args.kind == "runner":
        selected = harness_commands(args.words)
    else:
        values = keyword_values(args.words, ("directory", "file", "case"))
        if values and "directory" not in values:
            raise ValueError("Focused Python selection requires directory PATH")
        selected = python_commands(values.get("directory"), values.get("file"), values.get("case"))
    return [("Diagnostics", selected)]
