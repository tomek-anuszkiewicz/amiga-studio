"""Probe Rust public declarations in a disposable copy of the working tree."""

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from repository import git
from unused_public_cargo import RESTRICTED, VISIBILITY, build_contexts, build_scope, comparison, metadata, run_build
from unused_public_syntax import Discovery


def git_paths(root, *args):
    return set(git(root, "--no-optional-locks", *args, "-z").decode().split("\0")) - {""}


def capture(root, workspace, output, record):
    tracked = git_paths(root, "ls-files", "--cached")
    untracked = git_paths(root, "ls-files", "--others", "--exclude-standard")
    unstaged = git_paths(root, "diff", "--no-ext-diff", "--name-only", "--no-renames")
    staged = git_paths(root, "diff", "--cached", "--no-ext-diff", "--name-only", "--no-renames")
    # Cargo and Rustup can read these inputs even when Git ignores them.
    local_config = {".cargo/config", ".cargo/config.toml", "rust-toolchain", "rust-toolchain.toml", "Cargo.lock"}
    changed = record.setdefault("working_copy_selections", [])
    digest = hashlib.sha256()
    record["file_count"] = 0
    for name in sorted(tracked | untracked | local_config):
        source = root / name
        if source.is_relative_to(output):
            continue
        if not source.is_file():
            if name in unstaged:
                changed.append({"path": name, "version": "absent from working copy", "staged": name in staged})
            continue
        if source.is_symlink():
            raise ValueError(f"Cannot capture linked source {name!r} as a regular working copy file.")
        destination = workspace / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        data = source.read_bytes()
        destination.write_bytes(data)
        if os.name != "nt":
            destination.chmod(source.stat().st_mode & 0o777)
        file_digest = hashlib.sha256(data).hexdigest()
        digest.update(json.dumps([name, file_digest], ensure_ascii=True).encode())
        record["file_count"] += 1
        if name in unstaged or name in staged or name in untracked:
            changed.append({"path": name, "version": "working copy", "differs_from_index": name in unstaged,
                            "staged": name in staged, "untracked": name in untracked, "sha256": file_digest})
    record.update(state="captured", sha256=digest.hexdigest())


def build_environment(sandbox):
    target = str(sandbox / "target")
    scratch = sandbox / "tmp"
    scratch.mkdir()
    return dict(os.environ, CARGO_TARGET_DIR=target, CARGO_BUILD_BUILD_DIR=target,
                TMP=str(scratch), TEMP=str(scratch), TMPDIR=str(scratch), CARGO_TERM_COLOR="never")


def toolchain(root, workspace, environment):
    # Rustup may select a directory override in the live checkout.
    command = ["rustup", "show", "active-toolchain"]
    if not environment.get("RUSTUP_TOOLCHAIN"):
        try:
            result = subprocess.run(command, cwd=root, capture_output=True, encoding="utf-8", errors="replace")
        except FileNotFoundError:
            pass
        else:
            if result.returncode == 0 and result.stdout.split():
                environment["RUSTUP_TOOLCHAIN"] = result.stdout.split()[0]
    return subprocess.check_output(["cargo", "--version", "--verbose"], cwd=workspace,
                                   env=environment, encoding="utf-8", errors="replace").strip()


@contextmanager
def visibility_edit(workspace, original, edit, entry):
    path = workspace / edit["path"]
    start, stop = edit["byte_start"], edit["byte_end"]
    if original[start:stop] != VISIBILITY or path.read_bytes() != original:
        raise ValueError(f"Captured source or visibility changed at {edit['path']} byte {start}.")
    modified = original[:start] + RESTRICTED + original[stop:]
    entry["source_sha256"] = {"pristine": hashlib.sha256(original).hexdigest(),
                             "probed": hashlib.sha256(modified).hexdigest()}
    try:
        path.write_bytes(modified)
        yield
    finally:
        path.write_bytes(original)


def probe_context(workspace, environment, output, report, discovery, producer, declarations, name, context):
    baseline = context["baseline"]
    command = context["command"]
    report["active"] = {"phase": "baseline", "context": name}
    print(f"[INFO] Public probe baseline for {name}", flush=True)
    run_build(workspace, environment, command, output / f"{name}-baseline.log", baseline)
    if baseline["state"] != "succeeded":
        for entry, declaration in declarations:
            if declaration is not None:
                entry["comparisons"][name].update(state="baseline blocked",
                                                  reason="The pristine build failed in this context.")
        return
    units = [artifact for artifact in baseline["artifacts"] if artifact["package_id"] == producer["id"]]
    for index, (entry, declaration) in enumerate(declarations, 1):
        if declaration is None:
            continue
        result = entry["comparisons"][name]
        report["active"] = {"phase": "comparison", "context": name,
                            "declaration": entry["visibility_span"]}
        active = declaration.activity(units)
        if active is False:
            result.update(state="inactive", reason="Cargo target or Rust feature/test condition was inactive.")
            continue
        result["activity"] = "confirmed" if active else "custom or target cfg not resolved"
        build = result.setdefault("build", {"state": "pending"})
        try:
            original, _ = discovery.source(declaration.span["path"])
            with visibility_edit(workspace, original, declaration.visibility, result):
                run_build(workspace, environment, command, output / f"{name}-{index:04}.log", build)
        except KeyboardInterrupt:
            result["state"] = "interrupted"
            raise
        finally:
            if "artifacts" in build:
                result.update(comparison(discovery, declaration, baseline, build))
            # The structured observations and full log retain the compiler messages.
            build.pop("diagnostics", None)
            build.pop("artifacts", None)
        if any(row["kind"] != "unresolved diagnostic" for row in result["observations"]):
            result["state"] = "observed"
        elif result["observations"]:
            result["state"] = "attribution unresolved"
        elif build["state"] != "succeeded":
            result.update(state="failed", reason="The edited build stopped without an attributable compiler observation.")
        elif active is None:
            result.update(state="activity unresolved", reason="The declaration's build configuration could not be determined.")
        else:
            result.update(state="inconclusive", reason="No new diagnostic. Internal code may still use this declaration.")


def configure_probe(root, workspace, output, report):
    report["active"] = {"phase": "capture"}
    capture(root, workspace, output, report["candidate"])
    environment = build_environment(workspace.parent)
    report["active"] = {"phase": "build selection"}
    report["cargo"] = toolchain(root, workspace, environment)
    report["toolchain"] = environment.get("RUSTUP_TOOLCHAIN")
    producer, packages = build_scope(metadata(workspace, environment), report["package"], workspace)
    report["packages_to_compile"] = [package["name"] for package in packages]
    report["contexts"] = build_contexts(packages)
    report["omitted_contexts"] = [
        "Target triples outside the active Cargo configuration.",
        "Individual feature combinations outside the reported feature selections.",
        "Rustdoc, doctests and external consumers.",
    ]
    targets = [target for target in producer["targets"] if "test" not in target["kind"]]
    pending = [Path(target["src_path"]).relative_to(workspace).as_posix() for target in targets]
    report["discovery"].update(state="running", pending_targets=pending)
    report["active"] = {"phase": "discovery"}
    discovery = Discovery(workspace, report["package"], report["discovery"])
    try:
        for target in targets:
            report["active"]["target"] = Path(target["src_path"]).relative_to(workspace).as_posix()
            discovery.discover(target)
            pending.remove(report["active"]["target"])
        report["discovery"]["state"] = "completed"
    except KeyboardInterrupt:
        report["discovery"]["state"] = "interrupted"
        raise
    finally:
        edits = discovery.edits()
        declarations = []
        for entry in report["discovery"]["declarations"]:
            span = entry["visibility_span"]
            declaration = edits[(span["path"], span["byte_start"])]
            state = "pending" if declaration else "unsupported"
            entry.update(original_visibility=VISIBILITY.decode(), probed_visibility=RESTRICTED.decode(),
                         comparisons={name: {"state": state, "baseline_context": name}
                                      for name in report["contexts"]})
            if declaration is None:
                entry["reason"] = "The same visibility token has multiple logical module origins."
            declarations.append((entry, declaration))
    for name, context in report["contexts"].items():
        probe_context(workspace, environment, output, report, discovery, producer, declarations, name, context)


def write_report(path, report):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def run_probe(root, package, results):
    root = root.resolve()
    results.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="unused-public-", dir=results)).resolve()
    path = output / "report.json"
    print(f"[INFO] Public probe report: {path}", flush=True)
    report = {
        "package": package, "state": "running", "candidate": {"state": "pending"},
        "discovery": {"state": "pending", "declarations": [], "gaps": []}, "contexts": {},
    }
    started = time.monotonic()
    write_report(path, report)
    code = 0
    try:
        # Avoid inheriting the live workspace's .cargo configuration a second time.
        with tempfile.TemporaryDirectory(prefix="amiga-public-") as temporary:
            sandbox = Path(temporary).resolve()
            workspace = sandbox / "workspace"
            workspace.mkdir()
            configure_probe(root, workspace, output, report)
        states = (entry["state"] for declaration in report["discovery"]["declarations"]
                  for entry in declaration["comparisons"].values())
        incomplete = any(state in {"failed", "baseline blocked"} for state in states)
        incomplete = incomplete or any(context["baseline"]["state"] != "succeeded"
                                       for context in report["contexts"].values())
        report["state"] = "partial" if incomplete else "completed"
        code = int(incomplete)
        report.pop("active")
    except KeyboardInterrupt:
        report.update(state="interrupted", reason="Interrupted by the user.")
        code = 130
    except (ImportError, OSError, RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        report.update(state="failed", reason=str(error))
        if isinstance(error, ImportError):
            report["reason"] += ". Install the dependencies in tools/verification/requirements.txt."
        print(f"[ERROR] Public probe failed: {report['reason']}", flush=True)
        code = 1
    finally:
        report["elapsed_seconds"] = round(time.monotonic() - started, 3)
        write_report(path, report)
    print(f"[INFO] Public probe {report['state']}; see {path}", flush=True)
    return code


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package")
    parser.add_argument("--results-dir", type=Path, default=Path(".agent/tmp/verify"))
    args = parser.parse_args(argv)
    return run_probe(Path.cwd(), args.package, args.results_dir)


if __name__ == "__main__":
    raise SystemExit(main())
