"""Run the fixed verification profile; route only expensive synthetic CPU suites."""

import hashlib
import subprocess
from pathlib import Path


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
        commands.extend([
            ("All ordinary crate tests", ["cargo", "test", "--no-fail-fast", "--workspace",
                                          "--exclude", "test_runner", "--", "--quiet"], {}),
        ])
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
    if completion:
        commands.append(("All Python tool contracts", [sys.executable, "-m", "unittest", "discover",
                                                     "-s", "tests", "-q"], {}))
    if mode == "milestone":
        commands.append(("Chipset throughput", ["cargo", "run", "--release", "-p", "test_runner", "--",
                                                "benchmark-chipset", "--compare"], {}))
        commands.append(("All runnable vAmigaTS captures", ["cargo", "run", "--release", "-p", "test_runner", "--",
                                                          "vamiga", "--category", "all", "--summary"], {}))
    return commands
