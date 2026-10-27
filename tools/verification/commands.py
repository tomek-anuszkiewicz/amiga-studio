"""Authoritative completion profiles and diagnostic command selection."""
import os
import sys
from pathlib import Path
from repository import REPO_ROOT


def singlestep_command(profile="verification", case=None, exact=False):
    command = [sys.executable, "tools/verification/run_singlestep.py", "--profile", profile]
    if case:
        command += ["--case", case]
    if exact:
        command.append("--exact")
    return command


# Full SingleStep verification runs automatically only at milestones.
# SingleStep uses TestMemoryBus, not production MemoryBus/PhysicalMemory.
def select_tests(root, mode, verify_singlestep=False, changes=()):
    completion = mode == "commit"
    singlestep = mode == "milestone" or verify_singlestep
    commands = []
    if completion:
        commands.extend(standard_harness(root))
    if singlestep:
        commands.append(("Full SingleStepTests", singlestep_command(),
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}))
    if completion:
        commands.extend(python_commands(
            include_bootstrap=any(path.startswith("tools/bootstrap/") for path in changes)))
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

TIER2_INTEGRATION_CRATES = ['memory_bus', 'machine_loop', 'debugger', 'gui']


def select_diagnostic_commands(unit=False, integration=False):
    """Select the former tier runner's Cargo targets without completion gates."""
    commands = []
    if unit:
        command = ["cargo", "test"]
        for crate in TIER1_UNIT_CRATES:
            command.extend(["-p", crate])
        commands.append(("Tier 1 ordinary crate tests", command + ["--", "--quiet"], {}))
    if integration:
        command = ["cargo", "test"]
        for crate in TIER2_INTEGRATION_CRATES:
            command.extend(["-p", crate])
        commands.append(("Tier 2 integration tests", command + ["--", "--quiet"], {}))
    return commands



PYTHON_SUITES = (
    ("Bootstrap contracts", "tools/bootstrap/tests"),
    ("Conversion contracts", "tools/bootstrap/conversion/tests"),
    ("PDF conversion contracts", "tools/bootstrap/pdf-to-markdown/tests"),
    ("Book review contracts", "tools/bootstrap/book-to-markdown/tests"),
    ("RAG MCP contracts", "tools/amiga-rag-mcp-server/tests"),
    ("Verification CLI contracts", "tools/verification/tests"),
)


PYTHON_INPUTS = {
    "Bootstrap contracts": (
        "tools/bootstrap/tests/", "tools/bootstrap/bootstrap.ps1",
        "tools/bootstrap/bootstrap_rag.ps1", "tools/bootstrap/ensure_qdrant.ps1",
    ),
    "Conversion contracts": (
        "tools/bootstrap/conversion/", "tools/bootstrap/html-to-markdown/",
        "tools/bootstrap/pdf-to-markdown/",
        "tools/bootstrap/bootstrap_documentation.ps1", "tools/bootstrap/bootstrap.ps1",
    ),
    "PDF conversion contracts": (
        "tools/bootstrap/pdf-to-markdown/", "tools/bootstrap/conversion/",
    ),
    "Book review contracts": (
        "tools/bootstrap/book-to-markdown/", "tools/bootstrap/pdf-to-markdown/",
        "tools/bootstrap/conversion/",
    ),
    "RAG MCP contracts": ("tools/amiga-rag-mcp-server/", ".codex/config.toml"),
    "Verification CLI contracts": ("tools/verification/",),
}


def python_commands(directory=None, pattern=None, case=None, changes=None, include_bootstrap=True):
    selections = PYTHON_SUITES if directory is None else [("Python contracts", directory)]
    if directory is None and not include_bootstrap:
        selections = [(name, target) for name, target in selections
                      if not target.startswith("tools/bootstrap/")]
    if directory is None and changes is not None:
        selections = [(name, target) for name, target in selections
                      if any(path.startswith(owned) if owned.endswith("/") else path == owned
                             for path in changes for owned in PYTHON_INPUTS[name])]
    result = []
    for name, target in selections:
        path = Path(target)
        if not path.is_absolute():
            path = REPO_ROOT / path
        if not path.is_dir():
            raise ValueError(f"Python test directory does not exist: {target}")
        if path.resolve() == REPO_ROOT / "tools/bootstrap/book-to-markdown/tests":
            command = [sys.executable, "tools/bootstrap/book-to-markdown/run_tests.py"]
        else:
            command = [sys.executable, "-m", "unittest", "discover", "-s", str(target), "-q"]
        if pattern:
            command += ["-p", pattern]
        if case:
            command += ["-k", case]
        result.append((name, command, {}))
    return result


def standard_harness(root):
    suites = {p.stem for p in (root / "crates/test_runner/tests").glob("test_*.rs")}
    standard = suites - {"test_singlestep"}
    if not standard:
        return []
    command = ["cargo", "test", "--no-fail-fast", "-p", "test_runner"]
    for suite in sorted(standard):
        command += ["--test", suite]
    return [("All standard harness tests", command + ["--", "--quiet"], {})]


def rust_commands(words, exact=False):
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
    if values["crate"] == "test_runner" and values.get("suite") == "test_singlestep":
        return [("Focused SingleStepTests", singlestep_command("test", values.get("case"), exact), {})]
    command = ["cargo", "test", "-p", values["crate"]]
    if "suite" in values:
        command += ["--test", values["suite"]]
    command += ["--", "--quiet"]
    if "case" in values:
        command.append(values["case"])
    if exact:
        command.append("--exact")
    return [("Focused Rust tests", command, {})]


def harness_commands(words, exact=False):
    from cli import keyword_values
    scope = words[0] if words else "standard"
    if scope == "standard" and len(words) <= 1:
        return standard_harness(REPO_ROOT)
    if scope == "silicon" and len(words) == 1:
        return select_tests(REPO_ROOT, "standalone", verify_singlestep=True)
    if scope != "singlestep":
        raise ValueError("Runner tests select standard, silicon or singlestep")
    selectors = list(words[1:])
    full = bool(selectors) and selectors[-1] == "full"
    if full:
        selectors.pop()
    values = keyword_values(selectors, ("case", "sample"))
    if full and "sample" in values:
        raise ValueError("Choose full or sample N, not both")
    env = {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}
    if "sample" in values:
        count = int(values["sample"])
        if count <= 0:
            raise ValueError("Sample count must be positive")
        env = {"SINGLESTEP_FULL": "0", "SINGLESTEP_LIMIT": str(count)}
    return [("SingleStepTests", singlestep_command(case=values.get("case"), exact=exact), env)]


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
    from cli import keyword_values

    args = list(words)
    if action == "gui" and "--" in args:
        args.remove("--")
    if action != "gui" and any(word.startswith("-") for word in args):
        raise ValueError("Runner commands use sentence selectors")
    if action == "build":
        if args:
            raise ValueError("Runner build accepts no additional arguments")
        return [("Build test harness targets", ["cargo", "test", "--release", "--no-run", "-p", "test_runner", "--tests", "--benches"], {})]
    if action == "analyze":
        if len(args) != 1:
            raise ValueError("Use runner analyze RUN_DIRECTORY")
        command = [sys.executable, "tools/verification/cpu_performance_tests.py", "analyze", args[0]]
        return [("Benchmark analysis", command, {})]
    if action == "refresh":
        if len(args) != 1:
            raise ValueError("Use runner refresh RUN_DIRECTORY")
        command = [sys.executable, "tools/verification/cpu_performance_tests.py", "refresh", args[0]]
        return [("Refresh current benchmark cases", command, {})]
    if action == "gui":
        return [("GUI inspector", ["cargo", "run", "-p", "gui", "--bin", "gui-inspector", "--", *args], {})]
    if action == "bench":
        if args == ["inventory"]:
            return [("Decoder inventory", ["cargo", "bench", "-p", "test_runner", "--bench",
                                            "bench_instructions", "--", "--inventory"], {})]
        selector = []
        if args and args[0] in ("case", "cases", "mnemonic", "opcode"):
            if len(args) < 2:
                raise ValueError("benchmark selector requires a value")
            selector, args = args[:2], args[2:]
        values = keyword_values(args, ("duration-ms", "series"))
        for key, value in values.items():
            if not value or any(char not in "0123456789" for char in value) or int(value) <= 0:
                raise ValueError(f"{key} must be a positive integer")
        return [("CPU benchmark", [sys.executable, "tools/verification/cpu_performance_tests.py",
                                   "measure", *selector, *args], {})]
    raise ValueError(f"Unsupported runner action: {action}")


def select_phases(args, changes):
    from cli import keyword_values
    if args.command in ("commit", "milestone"):
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        if len(set(args.sets)) != len(args.sets) or set(args.sets) - {"singlestep"}:
            raise ValueError("Completion profiles accept singlestep at most once")
        coverage = select_coverage_commands(args.command)
        commands = select_quality_commands(args.command) + select_tests(
            REPO_ROOT, args.command, verify_singlestep="singlestep" in args.sets,
            changes=changes)
        phases = []
        if coverage:
            phases.append(("Coverage", coverage))
        phases.append(("Verification", commands))
        return phases
    if args.command == "check":
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        selected = check_commands(args.name, args.words)
    elif args.command == "runner":
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        selected = runner_commands(args.action, args.words)
    elif args.kind == "rust":
        if args.exact and not all(word in args.words for word in ("crate", "suite", "case")):
            raise ValueError("--exact requires test rust crate NAME suite NAME case NAME")
        selected = rust_commands(args.words, exact=args.exact)
    elif args.kind == "runner":
        if args.exact and (not args.words or args.words[0] != "singlestep" or "case" not in args.words):
            raise ValueError("--exact requires test runner singlestep case NAME")
        selected = harness_commands(args.words, exact=args.exact)
    else:
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        values = keyword_values(args.words, ("directory", "file", "case"))
        if values and "directory" not in values:
            raise ValueError("Focused Python selection requires directory PATH")
        selected = python_commands(values.get("directory"), values.get("file"), values.get("case"),
                                   changes=changes)
    return [("Diagnostics", selected)]
