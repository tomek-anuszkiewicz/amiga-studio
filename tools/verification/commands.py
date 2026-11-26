"""Authoritative completion profiles and diagnostic command selection."""
import sys
from pathlib import Path
from repository import REPO_ROOT, workspace_packages


CAPTURE_CONTRACTS = "Workflow capture contracts"
PUBLIC_PROBE = "tools/verification/unused_public.py"
FPS_WORKER = "--_mixed-scene-fps"


def singlestep_command(profile="verification", case=None, exact=False):
    command = [sys.executable, "tools/verification/run_singlestep.py", "--profile", profile]
    if case:
        command += ["--case", case]
    if exact:
        command.append("--exact")
    return command


def full_singlestep():
    return ("Full SingleStepTests", singlestep_command(),
            {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"})


def select_coverage_commands(package=None):
    """Measure ordinary workspace tests or all ordinary tests of one package."""
    package_args = ["-p", package] if package is not None else []
    test_args = package_args or ["--workspace", "--exclude", "test_runner"]
    report_scope = ["--package", package] if package is not None else []
    return [
        ("Prepare coverage report", [sys.executable, "tools/verification/coverage_report.py", "--prepare"], {}),
        ("Reset coverage counters", ["cargo", "llvm-cov", "clean", "--profraw-only"], {}),
        ("Ordinary crate tests" if package is not None else "All ordinary crate tests",
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


def select_quality_commands():
    """Declare external quality commands; the verifier only runs them."""

    def script(name, *args):
        return [sys.executable, f"tools/verification/checks/{name}.py", *args]

    return [
        ("Obsidian Frontmatter", script("check_obsidian_frontmatter"), {}),
        ("Formatting", ["cargo", "fmt", "--all", "--", "--check"], {}),
        ("Clippy Invariants", script("check_clippy_policy"), {}),
        ("WebAssembly build", ["pwsh", "-NoProfile", "-File", "tools/web.ps1", "build"], {}),
        ("Architecture Rules", script("check_architecture"), {}),
        ("Hardware Quality", script("audit_hardware_quality"), {}),
        ("Code Quality", script("audit_code_quality"), {}),
    ]


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
)

CAPTURE_TEST_INPUTS = (
    "tools/verification/run_tests.py", "tools/verification/worker_processes.py",
    "tools/verification/case_report.py", "tools/verification/repository.py",
    "tools/verification/tests/git_fixtures.py", "tools/verification/tests/fixtures/build_repositories.py",
)

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
                                   ".agents/skills/TA-WF-Step-review-docs/scripts/audit_vault.py",
                                   ".gitignore"),
    CAPTURE_CONTRACTS: ("tools/workflow/capture/", ".gitignore", *CAPTURE_TEST_INPUTS),

}


def python_contract_input(name, path):
    return any(path.startswith(owned) if owned.endswith("/") else path == owned
               for owned in PYTHON_INPUTS[name])


def verification_contract_input(path):
    """Inputs consumed by contracts of verification and its shared launchers."""
    target = Path(path)
    vocabulary = (target.parent.as_posix() == "tools/verification/checks/english_words"
                  and target.suffix == ".txt")
    return (not path.endswith(".md")
            and not vocabulary
            and python_contract_input("Verification CLI contracts", path))


def python_commands(directory=None, pattern=None, case=None, changes=None):
    registered_directories = {REPO_ROOT / suite for _, suite in PYTHON_SUITES}
    selections = PYTHON_SUITES if directory is None else [("Python contracts", directory)]
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
            if directory is not None:
                raise ValueError(f"Python test directory does not exist: {target}")
            raise ValueError(f"{name}: Python test directory does not exist: {target}")
        resolved = path.resolve()
        if resolved == REPO_ROOT / "tools/bootstrap/book-to-markdown/tests":
            command = [sys.executable, "tools/bootstrap/book-to-markdown/run_tests.py"]
        elif resolved in registered_directories:
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
    suites = {p.stem for p in (root / "crates/test_runner/tests").glob("test_*.rs") if p.is_file()}
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
        return [full_singlestep()]
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


def unused_commands(words, results_dir):
    forms = "Use check unused dependencies [crate NAME] or check unused public crate NAME"
    if not words or words[0] not in ("dependencies", "public"):
        raise ValueError(forms)
    action = words[0]
    if action == "dependencies" and len(words) == 1:
        return [("Unused Rust dependencies", ["cargo", "check", "--workspace"], {})]
    if len(words) != 3 or words[1] != "crate" or not words[2].strip():
        raise ValueError(forms)
    package = words[2]
    _, roots = workspace_packages(REPO_ROOT)
    if package not in roots.values():
        raise ValueError(f"Unknown workspace package: {package}")
    if action == "dependencies":
        return [("Unused Rust dependencies", ["cargo", "check", "-p", package], {})]
    command = [sys.executable, PUBLIC_PROBE, package]
    if results_dir is not None:
        command += ["--results-dir", str(results_dir)]
    return [("Candidate unused public Rust functions", command, {})]


def check_commands(name, words, results_dir=None):
    from cli import keyword_values
    labels = {"formatting": "Formatting", "clippy": "Clippy Invariants",
              "architecture": "Architecture Rules", "hardware": "Hardware Quality", "code": "Code Quality",
              "web": "WebAssembly build"}
    if name == "all" and not words:
        return [*check_commands("english", []), *select_quality_commands()]
    if name == "unused":
        return unused_commands(words, results_dir)
    if name == "english":
        if not words:
            scanner_args = []
        elif words[0] == "files" and len(words) > 1:
            scanner_args = ["--", *words[1:]]
        elif len(words) == 2 and words[0] == "removals" and words[1].strip():
            scanner_args = ["--removals", words[1]]
        else:
            raise ValueError("Use check english [files PATH ... | removals PATH]")
        return [("Strict English Policy",
                 [sys.executable, "tools/verification/checks/check_english.py", *scanner_args], {})]
    if name == "build":
        values = keyword_values(words, ("crate",))
        target = ["-p", values["crate"]] if values else ["--workspace"]
        return [("Rust compilation", ["cargo", "check", *target], {})]
    selected = [item for item in select_quality_commands() if item[0] == labels.get(name)]
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


def fps_group(results_dir):
    command = [sys.executable, "tools/verification/verify.py", FPS_WORKER]
    if results_dir is not None:
        command.append(str(results_dir))
    return ("Performance", [("Mixed Scene FPS", command, {})])


def select_phases(args, changes=(), *, results_dir=None):
    from cli import keyword_values
    if args.command in ("commit", "milestone"):
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        if len(set(args.sets)) != len(args.sets) or set(args.sets) - {"singlestep"}:
            raise ValueError("Completion profiles accept singlestep at most once")
        if args.command == "milestone":
            sweep = [sys.executable, "tools/verification/unused_public_workspace.py"]
            if results_dir is not None:
                sweep += ["--results-dir", str(results_dir)]
            return [("Preflight", [("Workspace unused public Rust functions", sweep, {})]),
                    ("Verification", [full_singlestep()])]
        harness = standard_harness(REPO_ROOT)
        if not harness:
            raise ValueError("No standard harness test targets in crates/test_runner/tests")
        quality = select_quality_commands()
        build_checks = {"Clippy Invariants", "WebAssembly build"}
        return [
            ("Preflight", [item for item in quality if item[0] not in build_checks]),
            ("Build checks", [item for item in quality if item[0] in build_checks]),
            ("Coverage", select_coverage_commands()),
            ("Verification", [*harness, *python_commands()]),
            fps_group(results_dir),
        ]
    if args.command == "check":
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        selected = check_commands(args.name, args.words, results_dir)
    elif args.command == "runner":
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        selected = runner_commands(args.action, args.words)
    elif args.kind == "fps":
        if args.exact:
            raise ValueError("--exact requires a focused Rust or SingleStep case")
        return [fps_group(results_dir)]
    elif args.kind == "coverage":
        if args.exact:
            raise ValueError("Coverage measures all ordinary tests of the selected packages; --exact is unsupported")
        package = None
        if args.words != ["workspace"]:
            values = keyword_values(args.words, ("crate",))
            if "crate" not in values:
                raise ValueError("Use test coverage crate NAME or test coverage workspace")
            _, roots = workspace_packages(REPO_ROOT)
            if values["crate"] not in set(roots.values()) - {"test_runner"}:
                raise ValueError("Coverage requires an ordinary workspace package, excluding test_runner")
            package = values["crate"]
        return [("Coverage", select_coverage_commands(package))]
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
