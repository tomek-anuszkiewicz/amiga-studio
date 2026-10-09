"""Select verification by the components instantiated in each test environment."""

import hashlib
import json
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


def package_graph(root):
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root, capture_output=True, text=True, encoding="utf-8",
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    packages = json.loads(result.stdout)["packages"]
    by_name = {p["name"]: p for p in packages}
    graph = {
        p["name"]: {d["name"] for d in p["dependencies"] if d["name"] in by_name}
        for p in packages
    }
    return by_name, graph


def dependencies(graph, name):
    result = set()
    pending = [name]
    while pending:
        current = pending.pop()
        if current not in result:
            result.add(current)
            pending.extend(graph.get(current, ()))
    return result


def select_tests(root, changes, mode):
    packages, graph = package_graph(root)
    full = mode == "milestone" or bool(changes & {"Cargo.toml", "Cargo.lock", ".cargo/config.toml"})
    changed = set()
    for name, package in packages.items():
        directory = Path(package["manifest_path"]).parent.relative_to(root.resolve()).as_posix() + "/"
        if any(p.startswith(directory) for p in changes):
            changed.add(name)

    commands = []
    # All ordinary crate tests use real components, including nested machine/session objects.
    selected = sorted(n for n in packages if n != "test_runner" and
                      (full or dependencies(graph, n) & changed))
    if selected:
        command = ["cargo", "test", "--no-fail-fast"]
        for name in selected:
            command.extend(["-p", name])
        commands.append(("Crate tests: " + ", ".join(selected), command + ["--", "--quiet"], {}))

    suites = {p.stem for p in (root / "crates/test_runner/tests").glob("test_*.rs")}
    cpu = {"test_singlestep", "test_dma_cartesian", "test_memory_bus"}
    vamiga = {s for s in suites if s.startswith("test_vamiga_")} | {"test_chipset_benchmark"}
    benchmark = suites - cpu - vamiga - {"test_architecture_rules"}
    shared_bus = any(p.startswith("crates/physical_memory/src/address_bus") for p in changes)
    runner_change = any(p.startswith("crates/test_runner/src/") and
                        not p.startswith(("crates/test_runner/src/benchmark/", "crates/test_runner/src/vamiga/"))
                        for p in changes)
    runner_manifest = "crates/test_runner/Cargo.toml" in changes
    synthetic = full or bool({"cpu", "config"} & changed) or shared_bus or runner_change or runner_manifest
    machine = full or bool(dependencies(graph, "machine_loop") & changed) or runner_manifest
    bench = full or bool({"cpu", "physical_memory", "config", "rtc"} & changed) or runner_manifest
    chosen = set()
    if synthetic:
        chosen.update(cpu)
    if machine or any(p.startswith(("crates/test_runner/src/vamiga/", "ref_src/vAmigaTS/")) for p in changes):
        chosen.update(vamiga)
    if bench or runner_change or any(p.startswith(("crates/test_runner/src/benchmark/",
                                                    "crates/test_runner/benches/", "tests/benchmarks/",
                                                    "tools/benchmarks/")) for p in changes):
        chosen.update(benchmark)
    if any(p.startswith("ref_src/SingleStepTests-680x0/") for p in changes):
        chosen.update(cpu)
    if runner_change:
        chosen.update(vamiga)
    if "debugger" not in selected and any(p.startswith("tests/bin/") for p in changes):
        commands.append(("Debugger sample programs", ["cargo", "test", "-p", "debugger", "--test", "test_sample_binaries", "--", "--quiet"], {}))
    # Helpers shared by test targets can affect all test_runner suites.
    if any(p.startswith("crates/test_runner/tests/") and not Path(p).stem.startswith("test_") for p in changes):
        chosen.update(suites - {"test_architecture_rules"})
    for path in changes:
        if path.startswith("crates/test_runner/tests/") and Path(path).stem.startswith("test_"):
            suite = Path(path).stem
            if suite == "test_architecture_rules":
                continue
            # Named test files can also be included as helpers by sibling targets
            # (e.g. golden row hashes imported by CSV tests). Recheck their group.
            if suite in cpu:
                chosen.update(cpu)
            elif suite.startswith("test_vamiga_") or suite == "test_chipset_benchmark":
                chosen.update(vamiga)
            else:
                chosen.update(benchmark)
    if chosen:
        command = ["cargo", "test", "--no-fail-fast", "-p", "test_runner"]
        for suite in sorted(chosen):
            command.extend(["--test", suite])
        commands.append(("Harness tests: " + ", ".join(sorted(chosen)), command + ["--", "--quiet"],
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"}))
    # Python environments mirror the tools their fixtures import or copy.
    import sys
    python_groups = {
        "harness": ({"test_verify", "test_audit_code_quality", "test_audit_docs_quality", "test_check_polish", "test_skill_governance"},
                    ("tools/harness/", ".agents/", "AGENTS.md")),
        "conversion": ({"test_conversion_codex", "test_conversion_publish", "test_pdf_conversion_codex"},
                       ("tools/bootstrap/conversion/", "tools/bootstrap/pdf-to-markdown/", "tools/bootstrap/html-to-markdown/")),
        "bootstrap": ({"test_bootstrap_contract"}, ("tools/bootstrap/",)),
        "mcp": ({"test_mcp_server_contract", "test_codex_mcp_launch"}, ("tools/amiga-rag-mcp-server/", ".codex/")),
        "worktree": ({"test_worktree_contract"}, ("tools/git/",)),
    }
    python_tests = set()
    for tests, prefixes in python_groups.values():
        if any(p.startswith(prefixes) or (p.startswith("tests/") and Path(p).stem in tests) for p in changes):
            python_tests.update(tests)
    for path in changes:
        if path.startswith("tests/test_") and path.endswith(".py"):
            python_tests.add(Path(path).stem)
    if full:
        commands.append(("All Python tool contracts", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-q"], {}))
    else:
        for test in sorted(python_tests):
            commands.append((f"Python: {test}", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", test + ".py", "-q"], {}))
    if mode == "milestone":
        commands.append(("Chipset throughput", ["cargo", "run", "--release", "-p", "test_runner", "--",
                                                "benchmark-chipset", "--compare"], {}))
        commands.append(("All runnable vAmigaTS captures", ["cargo", "run", "--release", "-p", "test_runner", "--",
                                                          "vamiga", "--category", "all", "--summary"], {}))
    return commands
