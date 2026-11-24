"""Find candidate unused Rust public functions in a disposable working copy."""

import argparse
from collections import defaultdict
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from repository import git
from unused_public_cargo import (
    DEAD_CODE_FLAG, RESTRICTED, VISIBILITY, build_contexts, build_scope, metadata, probe_rustflags, record_comparison,
    run_build,
)
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
def visibility_edits(workspace, discovery, edits, context):
    by_file = defaultdict(list)
    for edit in edits:
        by_file[edit["path"]].append(edit)
    sources = {}
    for name, file_edits in by_file.items():
        path = workspace / name
        original, _ = discovery.source(name)
        if path.read_bytes() != original:
            raise ValueError(f"Captured source changed at {name}.")
        parts = []
        previous = 0
        for edit in sorted(file_edits, key=lambda span: span["byte_start"]):
            start, stop = edit["byte_start"], edit["byte_end"]
            if start < previous or original[start:stop] != VISIBILITY:
                raise ValueError(f"Invalid captured visibility at {name} byte {start}.")
            parts.extend((original[previous:start], RESTRICTED))
            previous = stop
        parts.append(original[previous:])
        sources[path] = original, b"".join(parts)
    context["source_sha256"] = {
        path.relative_to(workspace).as_posix(): {"pristine": hashlib.sha256(original).hexdigest(),
                                                 "probed": hashlib.sha256(modified).hexdigest()}
        for path, (original, modified) in sources.items()
    }
    changed = {}
    try:
        for path, (original, modified) in sources.items():
            changed[path] = original
            path.write_bytes(modified)
        yield
    finally:
        for path, original in changed.items():
            path.write_bytes(original)


def select_functions(declarations, units, name):
    selected = []
    for item, declaration, re_exports in declarations:
        result = item["comparisons"][name]
        active = declaration.activity(units)
        if active is False:
            result.update(state="inactive", reason="Cargo target or Rust feature/test condition was inactive.")
            continue
        result["activity"] = "confirmed" if active else "custom or target cfg not resolved"
        applicable = []
        for re_export in re_exports:
            for unit in units:
                if declaration.activity([unit]) is True and re_export.activity([unit]) is True:
                    applicable.append(re_export.span)
                    break
        if applicable:
            result.update(state="re-exported", producer_re_exports=applicable,
                          reason="A simple public re-export in the producer resolves to this function.")
        else:
            selected.append((declaration, result))
    return selected


def probe_context(workspace, environment, output, report, discovery, producer, package_names, declarations, name,
                  context):
    baseline, batch = context["baseline"], context["batch"]
    command = context["command"]
    report["active"] = {"phase": "baseline", "context": name}
    print(f"[INFO] Public probe baseline for {name}", flush=True)
    run_build(workspace, environment, command, output / f"{name}-baseline.log", baseline)
    if baseline["state"] != "succeeded":
        batch.update(state="skipped", reason="The pristine build failed in this context.")
        for item, _, _ in declarations:
            item["comparisons"][name].update(state="baseline blocked", reason=batch["reason"])
        return
    units = [artifact for artifact in baseline["artifacts"] if artifact["package_id"] == producer["id"]]
    selected = select_functions(declarations, units, name)
    context["edits"] = [declaration.visibility for declaration, _ in selected]
    if not selected:
        batch.update(state="skipped", reason="There are no eligible active functions in this context.")
        return

    report["active"] = {"phase": "batch", "context": name}
    print(f"[INFO] Public probe batch for {name}: {len(selected)} functions", flush=True)
    try:
        with visibility_edits(workspace, discovery, context["edits"], context):
            run_build(workspace, environment, command, output / f"{name}-batch.log", batch)
    except KeyboardInterrupt:
        batch["state"] = "interrupted"
        raise
    finally:
        if "artifacts" in batch:
            record_comparison(discovery, producer, package_names, selected, context)
        for _, result in selected:
            if result["observations"]:
                result["state"] = "observed"
            elif batch["state"] == "interrupted":
                result["state"] = "interrupted"
            elif batch["state"] != "succeeded" and batch.get("failure_kind") != "visibility":
                result.update(state="incomplete", reason="The batch stopped without an attributable diagnostic.")
            elif result["activity"] != "confirmed":
                result.update(state="activity unresolved", reason="The function's build configuration is uncertain.")
            else:
                result.update(state="inconclusive", reason="No new diagnostic for the narrowed function.")


def record_discovery(discovery, report, producer):
    edits = discovery.edits()
    re_exports = discovery.producer_re_exports(producer["edition"])
    eligible = []
    for entry in report["discovery"]["declarations"]:
        span = entry["visibility_span"]
        declaration = edits[(span["path"], span["byte_start"])]
        state = "pending"
        if entry["kind"] != "function":
            state = "outside function scan"
        elif declaration is None:
            state = "unsupported"
            entry["reason"] = ("The source is also used by a consumer target."
                               if span["path"] in discovery.consumer_sources
                               else "The same visibility token has multiple logical module origins.")
        elif declaration.role is None:
            state = "outside function scan"
            entry["reason"] = "The declaration is outside the supported free functions and inherent methods."
        else:
            key = (declaration.span["path"], declaration.span["byte_start"])
            eligible.append((entry, declaration, re_exports.get(key, [])))
        entry.update(state=state, comparisons={name: {"state": state, "observations": []}
                                               for name in report["contexts"]})
    return eligible


def configure_probe(root, workspace, output, report):
    report["active"] = {"phase": "capture"}
    capture(root, workspace, output, report["candidate"])
    environment = build_environment(workspace.parent)
    report["active"] = {"phase": "build selection"}
    report["cargo"] = toolchain(root, workspace, environment)
    report["toolchain"] = environment.get("RUSTUP_TOOLCHAIN")
    cargo_metadata = metadata(workspace, environment)
    producer, packages = build_scope(cargo_metadata, report["package"], workspace)
    package_names = {package["id"]: package["name"] for package in cargo_metadata["packages"]}
    report["packages_to_compile"] = [package["name"] for package in packages]
    flag_source, flag_options = probe_rustflags(workspace, environment)
    report["rustflags"] = {"source": flag_source, "added": DEAD_CODE_FLAG}
    report["contexts"] = build_contexts(packages, flag_options)
    report["omitted_contexts"] = [
        "Target triples outside the active Cargo configuration.",
        "Individual feature combinations outside the reported feature selections.",
        "Rustdoc, doctests and external consumers.",
    ]
    targets = [target for target in producer["targets"] if "test" not in target["kind"]]
    consumer_targets = [(package["name"], target) for package in packages for target in package["targets"]
                        if package["id"] != producer["id"] or "test" in target["kind"]]
    pending = [Path(target["src_path"]).relative_to(workspace).as_posix() for target in targets]
    pending_consumers = [{"package": package, "path": Path(target["src_path"]).relative_to(workspace).as_posix()}
                         for package, target in consumer_targets]
    report["discovery"].update(state="running", pending_targets=pending, pending_consumer_targets=pending_consumers)
    report["active"] = {"phase": "discovery"}
    discovery = Discovery(workspace, report["package"], report["discovery"])
    try:
        for target in targets:
            report["active"]["target"] = Path(target["src_path"]).relative_to(workspace).as_posix()
            discovery.discover(target)
            pending.remove(report["active"]["target"])
        for package, target in consumer_targets:
            consumer = pending_consumers[0]
            report["active"] = {"phase": "discovery", "target": consumer["path"], "consumer_package": package}
            discovery.discover_consumer(package, target)
            pending_consumers.pop(0)
        report["discovery"]["state"] = "completed"
    except KeyboardInterrupt:
        report["discovery"]["state"] = "interrupted"
        raise
    finally:
        declarations = record_discovery(discovery, report, producer)
    for name, context in report["contexts"].items():
        probe_context(workspace, environment, output, report, discovery, producer, package_names, declarations,
                      name, context)


def write_report(path, report):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def summarize(report):
    incomplete_contexts = [
        name for name, context in report["contexts"].items()
        if context["baseline"]["state"] != "succeeded" or context["batch"]["state"] not in {"succeeded", "skipped"}
        or context.get("unfinished_targets")
    ]
    report["incomplete_contexts"] = incomplete_contexts
    candidates = report["candidates"] = []
    references = report["confirmed_references"] = []
    for item in report["discovery"]["declarations"]:
        if item["state"] != "pending":
            continue
        used, warnings, re_exports, states = [], [], [], set()
        for name, result in item["comparisons"].items():
            states.add(result["state"])
            for span in result.get("producer_re_exports", []):
                if span not in re_exports:
                    re_exports.append(span)
            for row in result["observations"]:
                record = {"context": name, "diagnostic_index": row["diagnostic_index"]}
                if row["kind"] == "privacy":
                    used.append(record)
                elif row["kind"] == "dead_code":
                    warnings.append(record)
        symbol = {key: item[key] for key in ("name", "module", "enclosing", "span", "name_span")}
        if re_exports:
            item["producer_re_exports"] = re_exports
        if used or re_exports:
            item["state"] = "used" if used else "re-exported"
            references.append({**symbol, "observations": used, "producer_re_exports": re_exports})
        elif warnings:
            item["state"] = "candidate"
            candidates.append({**symbol, "observations": warnings, "incomplete_contexts": incomplete_contexts})
        elif "inconclusive" in states:
            item["state"] = "inconclusive"
        elif states == {"inactive"}:
            item["state"] = "inactive"
        else:
            item["state"] = "unexamined"


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
        incomplete = any(context["baseline"]["state"] != "succeeded" or
                         context["batch"].get("failure_kind") == "genuine"
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
        summarize(report)
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
