"""Authoritative completion profiles and diagnostic command selection."""
import os
import sys
from pathlib import Path
from repository import REPO_ROOT
from impact import RustScope, select_rust_scope, workspace_packages


CAPTURE_CONTRACTS = "Workflow capture contracts"
ROUTING_CONTRACTS = "Workflow routing contracts"


def singlestep_command(profile="verification", case=None, exact=False):
    command = [sys.executable, "tools/verification/run_singlestep.py", "--profile", profile]
    if case:
        command += ["--case", case]
    if exact:
        command.append("--exact")
    return command


# Full SingleStep verification runs automatically only at milestones.
# SingleStep uses TestMemoryBus, not production MemoryBus/PhysicalMemory.
def select_tests(root, mode, verify_singlestep=False, changes=(), rust_scope=None):
    completion = mode == "commit"
    singlestep = mode == "milestone" or verify_singlestep
    commands = []
    if completion and (rust_scope is None or rust_scope.harness):
        commands.extend(standard_harness(root))
    if singlestep:
        commands.append(("Full SingleStepTests", singlestep_command(),
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}))
    if completion:
        commands.extend(python_commands(
            include_bootstrap=any(path.startswith("tools/bootstrap/") for path in changes)))
        if not any(verification_contract_input(path) for path in changes):
            commands = [item for item in commands if item[0] != "Verification CLI contracts"]
        for group in (CAPTURE_CONTRACTS, ROUTING_CONTRACTS):
            if not any(python_contract_input(group, path) for path in changes):
                commands = [item for item in commands if item[0] != group]
    return commands


def select_coverage_commands(mode, rust_scope=None):
    """Only ordinary workspace tests contribute to the commit coverage report."""
    import sys

    if mode not in ("commit", "coverage") or rust_scope is not None and rust_scope.ordinary == ():
        return []
    packages = rust_scope.ordinary if rust_scope is not None else None
    package_args = [part for name in packages for part in ("-p", name)] if packages is not None else []
    test_args = package_args if packages is not None else ["--workspace", "--exclude", "test_runner"]
    report_scope = [part for name in packages for part in ("--package", name)] if packages is not None else []
    return [
        ("Prepare coverage report", [sys.executable, "tools/verification/coverage_report.py", "--prepare"], {}),
        ("Reset coverage counters", ["cargo", "llvm-cov", "clean", "--profraw-only"], {}),
        ("Ordinary crate tests" if packages is not None else "All ordinary crate tests",
         ["cargo", "llvm-cov", "nextest", "--no-report", "--no-fail-fast", *test_args,
          "--config-file", ".config/nextest.toml", "--profile", "default",
          "--retries", "0", "--no-tests", "fail"], {}),
        ("Export ordinary crate coverage", ["cargo", "llvm-cov", "report", "--json",
                                            *package_args,
                                            "--ignore-filename-regex", r"[/\\]test_runner[/\\]",
                                            "--output-path", ".agent/tmp/coverage/coverage.json"], {}),
        ("Ordinary crate coverage summary", [sys.executable, "tools/verification/coverage_report.py",
                                             *report_scope], {}),
    ]


def select_quality_commands(mode):
    """Declare external quality commands; the verifier only runs them."""
    import sys

    def script(name, *args):
        return [sys.executable, f"tools/verification/checks/{name}.py", *args]

    if mode == "commit":
        return [
            ("Obsidian Frontmatter", script("check_obsidian_frontmatter"), {}),
            ("Formatting", ["cargo", "fmt", "--all", "--", "--check"], {}),
            ("Clippy Invariants", script("check_clippy_policy"), {}),
            ("WebAssembly build", ["powershell" if os.name == "nt" else "pwsh",
                                  "-NoProfile", "-File", "tools/web.ps1", "build"], {}),
            ("Architecture Rules", script("check_architecture"), {}),
            ("Hardware Quality", script("audit_hardware_quality"), {}),
            ("Code Quality", script("audit_code_quality"), {}),
        ]
    return []


TIER1_UNIT_CRATES = [
    "agnus", "cia", "config", "denise", "disassembler", "floppy", "frame_builder",
    "game_ports", "keyboard", "cpu", "paula", "physical_memory", "rtc",
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
    ("Graphify contracts", "tools/graphify/tests"),
    ("Conversion contracts", "tools/bootstrap/conversion/tests"),
    ("HTML conversion contracts", "tools/bootstrap/html-to-markdown/tests"),
    ("PDF conversion contracts", "tools/bootstrap/pdf-to-markdown/tests"),
    ("Book review contracts", "tools/bootstrap/book-to-markdown/tests"),
    ("RAG MCP contracts", "tools/amiga-rag-mcp-server/tests"),
    ("Artifact report contracts", "tools/artifacts/tests"),
    ("Verification CLI contracts", "tools/verification/tests"),
    (CAPTURE_CONTRACTS, "tools/workflow/capture/tests"),
    (ROUTING_CONTRACTS, "tools/workflow/tests"),
)


INDEPENDENT_PYTHON_SUITES = frozenset({
    "RAG MCP contracts", "Artifact report contracts",
})


def independent_contract(name, commands):
    """Only audited directory commands with spare capacity may overlap coverage."""
    if name not in INDEPENDENT_PYTHON_SUITES or len(commands) != 1:
        return False
    return any(label == name and commands[0] == (label, command, overrides)
               for label, command, overrides in python_commands(include_bootstrap=False))


PYTHON_INPUTS = {
    "Bootstrap contracts": (
        "tools/bootstrap/tests/", "tools/bootstrap/bootstrap.ps1",
        "tools/bootstrap/bootstrap_rag.ps1", "tools/bootstrap/ensure_qdrant.ps1",
        "tools/bootstrap/bootstrap_graphify.ps1",
    ),
    "Graphify contracts": ("tools/graphify/",),
    "Conversion contracts": (
        "tools/bootstrap/conversion/", "tools/bootstrap/html-to-markdown/",
        "tools/bootstrap/pdf-to-markdown/",
        "tools/bootstrap/bootstrap_documentation.ps1", "tools/bootstrap/bootstrap.ps1",
    ),
    "HTML conversion contracts": (
        "tools/bootstrap/html-to-markdown/", "tools/bootstrap/conversion/",
    ),
    "PDF conversion contracts": (
        "tools/bootstrap/pdf-to-markdown/", "tools/bootstrap/conversion/",
    ),
    "Book review contracts": (
        "tools/bootstrap/book-to-markdown/", "tools/bootstrap/pdf-to-markdown/",
        "tools/bootstrap/conversion/",
    ),
    "RAG MCP contracts": ("tools/amiga-rag-mcp-server/", ".codex/config.toml"),
    "Artifact report contracts": ("tools/artifacts/",),
    "Verification CLI contracts": ("tools/verification/", "tools/web.ps1", "tools/worktree.ps1", "tools/graphify_query.py",
                                   ".agents/skills/TA-audit-vault-links/scripts/", ".gitignore"),
    CAPTURE_CONTRACTS: ("tools/workflow/capture/", ".gitignore"),
    ROUTING_CONTRACTS: (
        "tools/workflow/review_routing.py", "tools/workflow/review_inputs.py",
        "tools/workflow/review_decision.py", "tools/workflow/review-routing.json",
        "tools/workflow/guidance.py", "tools/workflow/result.py", "tools/workflow/result.schema.json",
        "tools/workflow/requirements.txt", "tools/workflow/skill_matrix.py",
        "tools/workflow/skill-matrix.json", "tools/workflow/tests/",
    ),
}


def python_contract_input(name, path):
    return any(path.startswith(owned) if owned.endswith("/") else path == owned
               for owned in PYTHON_INPUTS[name])


def verification_contract_input(path):
    """Inputs consumed by contracts of verification and its shared launchers."""
    from impact import language_data_input
    return (not path.endswith(".md")
            and (path == "tools/verification/checks/check_english.json" or not language_data_input(path))
            and python_contract_input("Verification CLI contracts", path))


def python_commands(directory=None, pattern=None, case=None, changes=None, include_bootstrap=True):
    selections = PYTHON_SUITES if directory is None else [("Python contracts", directory)]
    if directory is None and not include_bootstrap:
        selections = [(name, target) for name, target in selections
                      if not target.startswith("tools/bootstrap/")]
    if directory is None and changes is not None:
        selections = [(name, target) for name, target in selections
                      if any(python_contract_input(name, path) for path in changes)]
        if not any(verification_contract_input(path) for path in changes):
            selections = [(name, target) for name, target in selections if name != "Verification CLI contracts"]
    result = []
    for name, target in selections:
        path = Path(target)
        if not path.is_absolute():
            path = REPO_ROOT / path
        if not path.is_dir():
            raise ValueError(f"Python test directory does not exist: {target}")
        if path.resolve() == REPO_ROOT / "tools/bootstrap/book-to-markdown/tests":
            command = [sys.executable, "tools/bootstrap/book-to-markdown/run_tests.py"]
        elif path.resolve() == REPO_ROOT / "tools/verification/tests":
            command = [sys.executable, "tools/verification/run_tests.py", "-s", str(target), "-q"]
        else:
            command = [sys.executable, "tools/verification/run_unittest.py", "discover", "-s", str(target), "-q"]
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
    labels = {"formatting": "Formatting", "clippy": "Clippy Invariants",
              "architecture": "Architecture Rules", "hardware": "Hardware Quality", "code": "Code Quality",
              "web": "WebAssembly build"}
    if name == "all" and not words:
        return [*check_commands("english", []), *select_quality_commands("commit")]
    if name == "english":
        if words and (words[0] != "files" or len(words) == 1):
            raise ValueError("Use check english [files PATH ...]")
        return [("Strict English Policy",
                 [sys.executable, "tools/verification/checks/check_english.py", *words[1:]], {})]
    if name == "build":
        values = keyword_values(words, ("crate",))
        target = ["-p", values["crate"]] if values else ["--workspace"]
        return [("Rust compilation", ["cargo", "check", *target], {})]
    selected = [item for item in select_quality_commands("commit") if item[0] == labels.get(name)]
    if words:
        if name == "architecture":
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


def select_phases(args, changes, explanations=None):
    from cli import keyword_values
    if args.command in ("commit", "milestone"):
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        if len(set(args.sets)) != len(args.sets) or set(args.sets) - {"singlestep"}:
            raise ValueError("Completion profiles accept singlestep at most once")
        rust_scope = None
        if args.command == "commit":
            # Workflow suite ownership is for Python selection. Keep the existing Rust fallback.
            rust_python_inputs = {path for name, inputs in PYTHON_INPUTS.items()
                                  if name not in (CAPTURE_CONTRACTS, ROUTING_CONTRACTS) for path in inputs}
            rust_scope = select_rust_scope(REPO_ROOT, changes, rust_python_inputs)
            if getattr(args, "integration_divergent", False):
                rust_scope = RustScope(None, "Divergent integration requires full ordinary workspace verification.")
            if explanations is not None:
                explanations.extend(rust_scope.explain())
        coverage = select_coverage_commands(args.command, rust_scope)
        quality = select_quality_commands(args.command)
        build_checks = {"Clippy Invariants", "WebAssembly build"}
        early = [item for item in quality if item[0] not in build_checks]
        builds = [item for item in quality if item[0] in build_checks]
        commands = select_tests(
            REPO_ROOT, args.command, verify_singlestep="singlestep" in args.sets,
            changes=changes, rust_scope=rust_scope)
        phases = [("Preflight", early)]
        if builds:
            phases.append(("Build checks", builds))
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
    elif args.kind == "coverage":
        if args.exact:
            raise ValueError("Coverage measures all ordinary tests of the selected packages; --exact is unsupported")
        scope = None
        if args.words != ["workspace"]:
            values = keyword_values(args.words, ("crate",))
            if "crate" not in values:
                raise ValueError("Use test coverage crate NAME or test coverage workspace")
            _, roots = workspace_packages(REPO_ROOT)
            if values["crate"] not in set(roots.values()) - {"test_runner"}:
                raise ValueError("Coverage requires an ordinary workspace package, excluding test_runner")
            scope = RustScope((values["crate"],), "Explicit coverage selection.")
        return [("Coverage", select_coverage_commands("coverage", scope))]
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
