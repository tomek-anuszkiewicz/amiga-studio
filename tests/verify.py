#!/usr/bin/env python3
"""
Amiga 500 verification and diagnostic test entry point.
"""

import argparse
import os
import sys
import subprocess
import time
import tempfile
import re
from collections import deque
import hashlib
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parents[1]


def git(root, *args):
    result = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))
    return result.stdout


def collect_changes(root):
    """Include staged, unstaged and new paths, with both rename endpoints."""
    changes = set()
    for args in (("diff", "--cached", "--name-only", "--no-renames", "-z"),
                 ("diff", "--name-only", "--no-renames", "-z"),
                 ("ls-files", "--others", "--exclude-standard", "-z")):
        changes.update(git(root, *args).decode("utf-8").split("\0"))
    return changes - {""}


def snapshot(root):
    """Fingerprint the source/index, excluding ignored test outputs."""
    digest = hashlib.sha256()
    for args in (("rev-parse", "HEAD"), ("diff", "--cached", "--binary"),
                 ("diff", "--binary"), ("ls-files", "--others", "--exclude-standard", "-z")):
        digest.update(git(root, *args))
    for path in git(root, "ls-files", "--others", "--exclude-standard", "-z").decode("utf-8").split("\0"):
        if path:
            digest.update(hashlib.sha256((root / path).read_bytes()).digest())
    return digest.hexdigest()


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
        suites = {p.stem for p in (root / "crates/test_runner/tests").glob("test_*.rs")}
        standard = suites - {"test_architecture_rules", "test_dma_cartesian", "test_singlestep"}
        command = ["cargo", "test", "--no-fail-fast", "-p", "test_runner"]
        for suite in sorted(standard):
            command.extend(["--test", suite])
        commands.append(("All standard harness tests", command + ["--", "--quiet"], {}))
    if dma or singlestep:
        commands.append(("Silicon vector prerequisites", [sys.executable,
                         "tests/checks/check_silicon_vectors.py"], {}))
    if dma:
        commands.append(("Cartesian DMA", ["cargo", "test", "-p", "test_runner", "--test",
                                           "test_dma_cartesian", "--", "--quiet"], {}))
    if singlestep:
        commands.append(("Full SingleStepTests", ["cargo", "test", "-p", "test_runner", "--test",
                                                  "test_singlestep", "--", "--quiet"],
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}))
    if completion:
        commands.append(("All Python tool contracts", [sys.executable, "-m", "unittest", "discover",
                                                     "-s", "tests", "-q"], {}))
    return commands


def select_coverage_commands(mode):
    """Only ordinary workspace tests contribute to the commit coverage report."""
    import sys

    if mode != "commit":
        return []
    return [
        ("Prepare coverage report", [sys.executable, "tests/coverage_report.py", "--prepare"], {}),
        ("Reset coverage counters", ["cargo", "llvm-cov", "clean", "--profraw-only"], {}),
        ("All ordinary crate tests", ["cargo", "llvm-cov", "--no-report", "--no-fail-fast", "--workspace",
                                      "--exclude", "test_runner", "--", "--quiet"], {}),
        ("Export ordinary crate coverage", ["cargo", "llvm-cov", "report", "--json", "--summary-only",
                                            "--ignore-filename-regex", r"[/\\]test_runner[/\\]",
                                            "--output-path", ".agent/tmp/coverage/summary.json"], {}),
        ("Ordinary crate coverage summary", [sys.executable, "tests/coverage_report.py"], {}),
    ]


def select_quality_commands(mode):
    """Declare external quality commands; the verifier only runs them."""
    import sys

    def script(name, *args):
        return [sys.executable, f"tests/checks/{name}.py", *args]

    if mode == "milestone":
        return [
            ("Strict English Policy", script("check_polish", "--staged"), {}),
        ]
    if mode == "commit":
        return [
            ("Formatting", ["cargo", "fmt", "--all", "--", "--check"], {}),
            ("Clippy Invariants", ["cargo", "clippy", "--workspace", "--all-targets"], {}),
            ("Architecture Rules", ["cargo", "test", "-p", "test_runner", "--test",
                                    "test_architecture_rules", "--", "--quiet"], {}),
            ("Hardware Quality", script("audit_hardware_quality"), {}),
            ("Code Quality", script("audit_code_quality"), {}),
        ]
    return []


TIER1_UNIT_CRATES = [
    "agnus", "audio", "blitter", "cia", "config", "copper", "denise",
    "disassembler", "dma", "floppy", "frame_builder", "game_ports", "interrupts",
    "joystick", "keyboard", "cpu", "mouse", "paula", "physical_memory", "rtc", "sprites",
]

TIER1_TEST_RUNNER_TESTS = ['test_golden_row_hashes']

TIER2_INTEGRATION_CRATES = ['memory_bus', 'machine_loop', 'debugger', 'gui']

TIER3_HARNESS_TESTS = [
    ("test_runner", "test_architecture_rules"),
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
        # DMA is selected through select_tests so prerequisites and force selectors run once.
        command = ["cargo", "test", "-p", "test_runner"]
        for _, suite in TIER3_HARNESS_TESTS:
            if suite != "test_dma_cartesian":
                command.extend(["--test", suite])
        commands.append(("Tier 3 architecture and benchmark smoke tests", command + ["--", "--quiet"], {}))
    return commands


def color_label(label, color, stream=None):
    """Color console labels while keeping redirected output plain."""
    stream = sys.stdout if stream is None else stream
    if not stream.isatty() or "NO_COLOR" in os.environ or os.environ.get("TERM") == "dumb":
        return label
    if os.name == "nt":
        import ctypes
        import msvcrt

        try:
            handle = ctypes.c_void_p(msvcrt.get_osfhandle(stream.fileno()))
            mode = ctypes.c_ulong()
            kernel = ctypes.windll.kernel32
            if not kernel.GetConsoleMode(handle, ctypes.byref(mode)):
                return label
            if not kernel.SetConsoleMode(handle, mode.value | 0x0004):
                return label
        except (AttributeError, OSError, ValueError):
            return label
    return f"\033[{color}m{label}\033[0m"


def result_label(ok, stream=None):
    return color_label("[PASS]" if ok else "[FAIL]", "32" if ok else "31", stream)


def print_info(message):
    print(f"{color_label('[INFO]', '33')} {message}", flush=True)


def diagnostic_level(line):
    """Recognize conventional diagnostic markers in heterogeneous tool output."""
    text = re.sub(r"\x1b\[[0-9;]*m", "", line).strip()
    if text.startswith("[INFO]"):
        return "info"
    if re.search(r"\[(?:FAIL|ERROR|ERR|FATAL)\]|^error(?:\[[^]]+\])?:|^fatal:|"
                 r"^FAIL:|^FAILED\b|^failures:|\sFAILED$|\bpanicked\b|^Traceback\b",
                 text, re.IGNORECASE):
        return "error"
    if re.search(r"\[(?:WARN|WARNING|ANOMALY)\]|^warning(?:\[[^]]+\])?:", text, re.IGNORECASE):
        return "warn"
    return None


def run_cmd(cmd, cwd=REPO_ROOT, env=None, log_level="warn"):
    print(f"[CMD] {subprocess.list2cmdline(cmd)}", flush=True)
    log_dir = REPO_ROOT / ".agent/tmp/verify"
    log_dir.mkdir(parents=True, exist_ok=True)
    start = time.time()
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=log_dir,
        prefix=f"{Path(cmd[0]).stem}-", suffix=".log", delete=False,
    ) as log:
        log_path = Path(log.name)
        log.write(f"Command: {subprocess.list2cmdline(cmd)}\n")
        log.flush()
        tail = deque(maxlen=12)
        context_left = 0
        error_seen = False
        with subprocess.Popen(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, encoding="utf-8",
                              errors="replace") as process:
            for line in process.stdout:
                log.write(line)
                tail.append(line)
                level = diagnostic_level(line)
                show = level == "error" or (level == "warn" and log_level == "warn")
                error_seen = error_seen or level == "error"
                if level == "info":
                    print_info(line.strip()[len("[INFO]"):].strip())
                    context_left = 0
                elif log_level == "full":
                    print(line, end="", flush=True)
                elif show:
                    print(line.rstrip()[:300], flush=True)
                    context_left = 12
                elif level:
                    context_left = 0
                elif context_left and (not line.strip() or line[0].isspace()):
                    print(line.rstrip()[:300], flush=True)
                    context_left -= 1
                else:
                    context_left = 0
            code = process.wait()
    elapsed = time.time() - start
    if code:
        if log_level != "full" and not error_seen:
            print("[DETAIL] Unclassified command failure; last log lines:", flush=True)
            for line in tail:
                print(line.rstrip()[:300], flush=True)
        print(f"[LOG] {log_path.relative_to(REPO_ROOT).as_posix()}", flush=True)
    return code, elapsed

def execute_command(command, overrides, log_level="warn"):
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUNBUFFERED"] = "1"
    env.update(overrides)
    if overrides:
        print(f"[ENV] {' '.join(f'{key}={value}' for key, value in overrides.items())}", flush=True)
    try:
        code, elapsed = run_cmd(command, env=env, log_level=log_level)
    except OSError as error:
        return False, f"Cannot launch command: {error}", 0
    detail = f"command exited with code {code}"
    return code == 0, detail, elapsed


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""Verification sets:
  --commit
    Formatting, Clippy, architecture rules and per-commit
    hardware/code audits; all ordinary crate tests, standard harness tests and
    Python tool contracts.
    Ordinary crate tests run first with coverage instrumentation; their line,
    function and region percentages are summarized before uninstrumented checks.
    Also selects Cartesian DMA and full SingleStepTests when their relevant
    CPU, shared contract or test-harness paths change (staged, unstaged or new).
    Each CPU suite has its own change triggers; unrelated changes select neither.

  --milestone
    Staged English
    checks. Domain skills own design drift, vault metadata/links, semantic parity,
    diary chronology and roadmap retention; this set does not run their scripts.
    vAmigaTS captures and chipset workloads are not part of verification.
    A separate set: it does not include --commit or automatically select CPU suites.
    To close a milestone, run both --commit and --milestone.

  --verify-dma / --verify-singlestep
    Explicitly run Cartesian DMA / full SingleStepTests regardless of changes.
    These sets may run alone, together, or alongside --commit or --milestone.

  --unit / --integration / --harness / --all
    Focused diagnostics without completion gates or coverage instrumentation.
    --unit: isolated crate tests and golden-row fixtures.
    --integration: memory_bus, machine_loop, debugger and GUI tests.
    --harness: architecture, benchmark smoke and Cartesian DMA tests.
    --all: unit + integration (the former tier runner's scope).
    Combine diagnostic selectors with each other or explicit CPU selectors;
    run completion profiles separately.

  --plan
    Show the selected checks, test commands and environment overrides without
    executing checks or tests. Combine it with a verification set.

  --log-level full|warn|error
    full: show all command output; warn (default): warnings and errors;
    error: errors only. Diagnostic context is included, and full logs are saved
    in .agent/tmp/verify/ in every mode. Commands and final results remain visible.

All sets run on the current working tree; no staging or pending changes required.
The verifier does not create a Git commit.

Examples:
  python tests/verify.py --commit
  python tests/verify.py --milestone
  python tests/verify.py --commit --plan
  python tests/verify.py --verify-dma --verify-singlestep
""")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--commit", action="store_true", help="Run all standard tests; select DMA/SingleStep from working-tree changes")
    mode.add_argument("--milestone", action="store_true", help="Run only the milestone language audit")
    parser.add_argument("--verify-dma", action="store_true", help="Run the Cartesian DMA set, alone or alongside a completion profile")
    parser.add_argument("--verify-singlestep", action="store_true", help="Run the full SingleStepTests set, alone or alongside a completion profile")
    parser.add_argument("--unit", "-u", action="store_true", help="Run isolated crate tests and golden-row fixtures")
    parser.add_argument("--integration", "-i", action="store_true", help="Run machine, bus, debugger and GUI integration tests")
    parser.add_argument("--harness", "-H", action="store_true", help="Run architecture, benchmark smoke and Cartesian DMA tests")
    parser.add_argument("--all", "-a", action="store_true", help="Run unit and integration diagnostic tiers")
    parser.add_argument("--plan", action="store_true", help="Print selected commands without executing checks")
    parser.add_argument("--log-level", choices=("full", "warn", "error"), default="warn",
                        help="Command output verbosity (default: warn); full logs are always saved")
    argv = sys.argv[1:] if argv is None else argv
    if not argv:
        parser.print_help()
        return 0
    args = parser.parse_args(argv)
    diagnostic = any((args.unit, args.integration, args.harness, args.all))
    if diagnostic and (args.commit or args.milestone):
        parser.error("Run diagnostic tiers separately from --commit and --milestone")
    if not any((args.commit, args.milestone, args.verify_dma, args.verify_singlestep, diagnostic)):
        parser.error("Select a completion profile, diagnostic tier, --verify-dma or --verify-singlestep")
    selected_mode = "milestone" if args.milestone else "commit" if args.commit else "standalone"
    try:
        changes = collect_changes(REPO_ROOT) if selected_mode != "standalone" else set()
        before = snapshot(REPO_ROOT)
        commands = select_tests(REPO_ROOT, changes, selected_mode,
                                verify_dma=args.verify_dma or args.harness, verify_singlestep=args.verify_singlestep)
        commands = select_diagnostic_commands(args.unit or args.all, args.integration or args.all,
                                              args.harness) + commands
    except (RuntimeError, OSError, ValueError) as error:
        print(f"{result_label(False, sys.stderr)} Cannot select verification: {error}", file=sys.stderr)
        return 1
    gates = select_quality_commands(selected_mode)
    coverage = select_coverage_commands(selected_mode)
    if args.plan:
        print("[PLAN] Preview only; no checks or tests will run.")
        for kind, selected in (("Coverage", coverage), ("Quality", gates), ("Tests", commands)):
            for name, command, overrides in selected:
                print(f"  {kind}: {name}\n    {subprocess.list2cmdline(command)}")
                if overrides:
                    print(f"    Environment: {overrides}")
        return 0
    results = []
    phases = []
    if coverage:
        phases.append(("Building/reusing Cargo tests WITH coverage instrumentation.", coverage))
    phases.append(("Building/reusing Cargo artifacts WITHOUT coverage instrumentation."
                   if any(command[0] == "cargo" for _, command, _ in gates + commands) else None,
                   gates + commands))
    for description, selected in phases:
        if description:
            print_info(description)
        for name, command, overrides in selected:
            print(f"[RUN] {name}", flush=True)
            ok, detail, elapsed = execute_command(command, overrides, log_level=args.log_level)
            results.append(ok)
            print(f"{result_label(ok)} {name} ({elapsed:.2f}s): {detail}", flush=True)
    try:
        unchanged = before == snapshot(REPO_ROOT)
    except (RuntimeError, OSError):
        unchanged = False
    if not unchanged:
        print(f"{result_label(False)} Source or index changed during verification. Verify the current snapshot again.")
        results.append(False)
    print(f"{result_label(all(results))} {selected_mode} verification")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
