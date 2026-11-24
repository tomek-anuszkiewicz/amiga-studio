"""Run isolated Cargo builds and compare their structured output."""

from collections import Counter
import json
from pathlib import Path
import re
import subprocess

from unused_public_syntax import target_key
from worker_processes import WorkerProcesses


PRIVACY_CODES = {"E0364", "E0365", "E0451", "E0603", "E0616", "E0624"}
VISIBILITY = b"pub"
RESTRICTED = b"pub(crate)"


def metadata(root, environment):
    result = subprocess.run(["cargo", "metadata", "--offline", "--no-deps", "--format-version", "1"],
                            cwd=root, env=environment, capture_output=True, encoding="utf-8", errors="replace")
    if result.returncode:
        raise ValueError(result.stderr.strip())
    return json.loads(result.stdout)


def build_scope(workspace, name, root):
    members = set(workspace["workspace_members"])
    packages = [package for package in workspace["packages"] if package["id"] in members]
    matches = [package for package in packages if package["name"] == name]
    if len(matches) != 1:
        raise ValueError(f"Expected one workspace package named {name!r}.")
    producer = matches[0]
    paths = {Path(package["manifest_path"]).parent.resolve(): package["id"] for package in packages}
    if Path(workspace["workspace_root"]).resolve() != root:
        raise ValueError("Cargo selected a workspace outside the captured candidate.")
    selected = {producer["id"]}
    while True:
        previous = selected.copy()
        for package in packages:
            dependencies = (Path(dependency["path"]).resolve() for dependency in package["dependencies"]
                            if dependency.get("path"))
            if selected.intersection(paths.get(path) for path in dependencies):
                selected.add(package["id"])
        if selected == previous:
            break
    # Cargo metadata without dependency resolution lists workspace members only.
    for package in packages:
        if not Path(package["manifest_path"]).resolve().is_relative_to(root):
            raise ValueError(f"Package {package['name']!r} is outside the captured candidate.")
        for target in package["targets"]:
            if not Path(target["src_path"]).resolve().is_relative_to(root):
                raise ValueError(f"Cargo target {target['name']!r} is outside the captured candidate.")
        for dependency in package["dependencies"]:
            if dependency.get("path") and not Path(dependency["path"]).resolve().is_relative_to(root):
                raise ValueError(f"Local dependency {dependency['name']!r} is outside the captured candidate.")

    return producer, [package for package in packages if package["id"] in selected]


def build_contexts(packages):
    command = ["cargo", "check", "--offline", "--all-targets", "--message-format=json"]
    for package in packages:
        command.extend(["--package", package["name"]])
    selections = [("default-features", [])]
    if any(set(package["features"]) - {"default"} for package in packages):
        selections.append(("all-features", ["--all-features"]))
    if any(package["features"].get("default") for package in packages):
        selections.append(("no-default-features", ["--no-default-features"]))
    return {name: {"command": [*command, *flags], "baseline": {"state": "pending"}}
            for name, flags in selections}


def normalized_path(root, value):
    path = Path(value.replace("\\", "/"))
    if not path.is_absolute():
        return path.as_posix()
    return path.relative_to(root).as_posix() if path.is_relative_to(root) else path.as_posix()


def cargo_messages(path):
    with path.open(encoding="utf-8", errors="replace") as log:
        for line in log:
            if not line.startswith("{"):
                continue
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if row.get("reason") in {"compiler-artifact", "compiler-message", "build-finished"}:
                yield row


def run_build(root, environment, command, log, observation):
    observation.update(state="running", log=str(log))
    try:
        with log.open("wb") as output, WorkerProcesses() as workers:
            child = workers.start(command, output, environment, root)
            code = child.wait()
        observation.update(exit_code=code, state="succeeded" if code == 0 else "failed")
    except KeyboardInterrupt:
        observation["state"] = "interrupted"
        raise
    except OSError as error:
        observation.update(state="failed", reason=str(error))
        raise
    finally:
        if log.is_file():
            output = {"artifacts": [], "diagnostics": [], "cargo_success": None}
            for row in cargo_messages(log):
                if row["reason"] == "compiler-artifact":
                    output["artifacts"].append({
                        "package_id": row["package_id"], "target": target_key(row["target"]),
                        "features": row["features"], "test": row["profile"]["test"], "fresh": row["fresh"],
                    })
                elif row["reason"] == "compiler-message" and (row["message"]["code"] or row["message"]["spans"]):
                    diagnostic = row["message"]
                    output["diagnostics"].append({
                        "package_id": row["package_id"], "target": target_key(row["target"]),
                        **normalize_diagnostic(root, diagnostic),
                    })
                elif row["reason"] == "build-finished":
                    output["cargo_success"] = row["success"]
            observation.update(output)


def normalize_diagnostic(root, message):
    def span(row):
        return {
            "path": normalized_path(root, row["file_name"]),
            **{key: row[key] for key in ("byte_start", "byte_end", "line_start", "line_end",
                                        "column_start", "column_end", "is_primary", "label")},
            "macro_expansion": row["expansion"] is not None,
        }

    return {
        "code": message["code"]["code"] if message["code"] else None,
        "message": message["message"], "level": message["level"],
        "spans": [span(row) for row in message["spans"]],
        "children": [normalize_diagnostic(root, child) for child in message["children"]],
    }


def spans(diagnostic):
    yield from diagnostic["spans"]
    for child in diagnostic["children"]:
        yield from spans(child)


def original_offsets(span, edit):
    position = edit["byte_start"]
    extra = len(RESTRICTED) - len(VISIBILITY)

    def offset(value):
        if span["path"] != edit["path"] or value <= position + len(VISIBILITY):
            return value
        return max(position + len(VISIBILITY), value - extra)

    return offset(span["byte_start"]), offset(span["byte_end"])


def diagnostic_key(diagnostic, edit=None):
    ranges = tuple((span["path"], *(original_offsets(span, edit) if edit else
                                    (span["byte_start"], span["byte_end"])), span["is_primary"])
                   for span in spans(diagnostic))
    return (diagnostic["package_id"], diagnostic["target"]["name"], tuple(diagnostic["target"]["kind"]),
            diagnostic["level"], diagnostic["code"], diagnostic["message"], ranges)


def artifact_key(artifact):
    return (artifact["package_id"], artifact["target"]["name"], tuple(artifact["target"]["kind"]),
            artifact["test"], tuple(artifact["features"]))


def new_diagnostics(baseline, probe, edit):
    original = Counter(diagnostic_key(row) for row in baseline["diagnostics"])
    for row in probe["diagnostics"]:
        key = diagnostic_key(row, edit)
        if original[key]:
            original[key] -= 1
        else:
            yield row


def contains(declaration, span, edit):
    original = declaration.span
    start, stop = original_offsets(span, edit)
    return (span["path"] == original["path"] and original["byte_start"] <= start
            and stop <= original["byte_end"] and not span["macro_expansion"])


def referring_site(discovery, span, edit):
    if not (discovery.root / span["path"]).resolve().is_relative_to(discovery.root):
        return "unresolved referring site"
    try:
        original, tree = discovery.source(span["path"])
    except (OSError, ValueError):
        return "unresolved referring site"
    start, stop = original_offsets(span, edit)
    if span["macro_expansion"] or not 0 <= start < stop <= len(original):
        return "unresolved referring site"
    node = tree.descendant_for_byte_range(start, stop)
    while node is not None:
        if node.type == "use_declaration":
            public = any(child.type == "visibility_modifier" and child.text == b"pub"
                         for child in node.named_children)
            return "re-export" if public else "import"
        node = node.parent
    return "direct reference"


def observation_for(discovery, declaration, diagnostic):
    edit = declaration.visibility
    result = {"kind": "unresolved diagnostic", "diagnostic": diagnostic}
    primary = [span for span in diagnostic["spans"] if span["is_primary"]]
    if diagnostic["code"] == "dead_code":
        scope = declaration.module
        if declaration.kind == "module":
            scope = (*scope, declaration.name.removeprefix("r#"))
        for span in primary:
            symbols = [symbol for symbol in discovery.declarations.values()
                       if (symbol is declaration or declaration.kind == "module"
                           and symbol.module[:len(scope)] == scope) and contains(symbol, span, edit)]
            if symbols:
                symbol = min(symbols, key=lambda item: item.span["byte_end"] - item.span["byte_start"])
                return {**result, "kind": "dead_code", "item": symbol.report(discovery.package)}
    if diagnostic["code"] in PRIVACY_CODES:
        # Some privacy errors name a field and its owner without a declaration span.
        field = re.search(r"^field `([^`]+)` of struct `([^`]+)` is private", diagnostic["message"])
        field_match = (field is not None and declaration.kind == "field" and declaration.enclosing
                       and (declaration.name, declaration.enclosing[-1]) == field.groups())
        if field_match or any(contains(declaration, span, edit) for span in spans(diagnostic)):
            return {**result, "kind": "privacy", "referring_sites": [
                {"kind": referring_site(discovery, span, edit), "span": span} for span in primary
            ]}
    return result


def comparison(discovery, declaration, baseline, probe):
    compiled = {artifact_key(target) for target in probe["artifacts"]}
    diagnosed = {(row["package_id"], row["target"]["name"], tuple(row["target"]["kind"]))
                 for row in probe["diagnostics"]}
    unfinished = [{**target, "state": "diagnostics emitted" if artifact_key(target)[:3] in diagnosed
                   else "no compiler output observed"}
                  for target in baseline["artifacts"] if artifact_key(target) not in compiled]
    observations = [observation_for(discovery, declaration, row)
                    for row in new_diagnostics(baseline, probe, declaration.visibility)]
    finished = [{**target, "fresh": target["fresh"]} for target in probe["artifacts"]
                if target["package_id"].startswith("path+")]
    return {"observations": observations, "compiled_targets": finished, "unfinished_targets": unfinished}
