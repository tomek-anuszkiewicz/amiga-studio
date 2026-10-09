"""Run the fixed verification profile; route only expensive synthetic CPU suites."""

import hashlib
import subprocess
from pathlib import Path


def git(root, *args):
    result = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))
    return result.stdout


def collect_changes(root, mode):
    """Use both rename endpoints; commit verification tests an exact index snapshot."""
    if git(root, "diff", "--name-only", "-z"):
        raise RuntimeError("Tracked unstaged changes exist. Stage the intended snapshot first.")
    paths = git(root, "diff", "--cached", "--name-only", "--no-renames", "-z")
    untracked = git(root, "ls-files", "--others", "--exclude-standard", "-z")
    if untracked:
        raise RuntimeError("Untracked files exist. Stage intended files or move diagnostics to .agent/tmp/.")
    changes = set(paths.decode("utf-8").rstrip("\0").split("\0")) - {""}
    if mode == "commit" and not changes:
        raise RuntimeError("No staged changes. Use --milestone for full repository verification.")
    return changes


def snapshot(root):
    """Fingerprint the source/index, excluding ignored test outputs."""
    digest = hashlib.sha256()
    for args in (("rev-parse", "HEAD"), ("diff", "--cached", "--binary"),
                 ("diff", "--binary"), ("ls-files", "--others", "--exclude-standard", "-z")):
        digest.update(git(root, *args))
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

    complete = mode == "milestone" or bool(changes & WORKSPACE_INPUTS)
    dma = complete or verify_dma or any(p.startswith(DMA_PATHS) for p in changes)
    singlestep = complete or verify_singlestep or any(p.startswith(SINGLESTEP_PATHS) for p in changes)
    commands = [
        ("All ordinary crate tests", ["cargo", "test", "--no-fail-fast", "--workspace",
                                      "--exclude", "test_runner", "--", "--quiet"], {}),
    ]
    suites = {p.stem for p in (root / "crates/test_runner/tests").glob("test_*.rs")}
    standard = suites - {"test_architecture_rules", "test_dma_cartesian", "test_singlestep"}
    command = ["cargo", "test", "--no-fail-fast", "-p", "test_runner"]
    for suite in sorted(standard):
        command.extend(["--test", suite])
    commands.append(("All standard harness tests", command + ["--", "--quiet"], {}))
    if dma:
        commands.append(("Cartesian DMA", ["cargo", "test", "-p", "test_runner", "--test",
                                           "test_dma_cartesian", "--", "--quiet"], {}))
    if singlestep:
        commands.append(("Full SingleStepTests", ["cargo", "test", "-p", "test_runner", "--test",
                                                  "test_singlestep", "--", "--quiet"],
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}))
    commands.append(("All Python tool contracts", [sys.executable, "-m", "unittest", "discover",
                                                 "-s", "tests", "-q"], {}))
    if mode == "milestone":
        commands.append(("Chipset throughput", ["cargo", "run", "--release", "-p", "test_runner", "--",
                                                "benchmark-chipset", "--compare"], {}))
        commands.append(("All runnable vAmigaTS captures", ["cargo", "run", "--release", "-p", "test_runner", "--",
                                                          "vamiga", "--category", "all", "--summary"], {}))
    return commands
