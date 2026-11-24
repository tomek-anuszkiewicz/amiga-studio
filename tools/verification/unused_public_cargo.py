"""Run isolated Cargo builds and compare their structured output."""

from bisect import bisect_right
from collections import Counter, defaultdict
import json
from pathlib import Path
import re
import subprocess
import tomllib

from unused_public_syntax import target_key, text
from worker_processes import WorkerProcesses


PRIVACY_CODES = {"E0364", "E0603", "E0624"}
VISIBILITY = b"pub"
RESTRICTED = b"pub(crate)"
DEAD_CODE_FLAG = "--force-warn=dead_code"
COMPILATION_SUMMARY = re.compile(r"^error: could not compile `([^`]+)` .* due to \d+ previous errors?\b")


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


def cargo_configs(root, environment):
    cargo_home = root / environment["CARGO_HOME"] if "CARGO_HOME" in environment else Path.home() / ".cargo"
    directories = [cargo_home, *(path / ".cargo" for path in reversed(root.parents)), root / ".cargo"]
    for directory in dict.fromkeys(directories):
        config = directory / "config"
        if not config.is_file():
            config = directory / "config.toml"
        if config.is_file():
            with config.open("rb") as source:
                yield tomllib.load(source)


def add_lint_flag(value, separator=" "):
    return separator.join((value, DEAD_CODE_FLAG)) if value else DEAD_CODE_FLAG


def config_rustflags(root, environment):
    # Add to each existing nonempty target source and to Cargo's build fallback.
    # An unconditional target setting would displace the fallback.
    settings = {}
    for config in cargo_configs(root, environment):
        if "rustflags" in config.get("build", {}):
            settings[("build.rustflags", "CARGO_BUILD_RUSTFLAGS")] = config["build"]["rustflags"]
        for target, values in config.get("target", {}).items():
            if "rustflags" not in values:
                continue
            variable = None if target.startswith("cfg(") else f"CARGO_TARGET_{target.upper().replace('-', '_')}_RUSTFLAGS"
            key = f"target.{json.dumps(target, ensure_ascii=False)}.rustflags"
            previous = settings.get((key, variable), [])
            value = values["rustflags"]
            settings[key, variable] = previous + value if isinstance(previous, list) and isinstance(value, list) else value

    overrides = []
    settings.setdefault(("build.rustflags", "CARGO_BUILD_RUSTFLAGS"), [])
    for (key, variable), value in settings.items():
        if variable == "CARGO_BUILD_RUSTFLAGS" and variable in environment or environment.get(variable, "").strip():
            continue
        if key != "build.rustflags" and not (value.strip() if isinstance(value, str) else value):
            continue
        if isinstance(value, str):
            extra = add_lint_flag(value)
        elif isinstance(value, list):
            # Cargo merges an array supplied on the CLI with the configured array.
            extra = [DEAD_CODE_FLAG]
        else:
            raise ValueError(f"Expected rustflags to be a string or list at {key}.")
        overrides.extend(("--config", f"{key}={json.dumps(extra, ensure_ascii=False)}"))
    for variable, value in list(environment.items()):
        if variable == "CARGO_BUILD_RUSTFLAGS" or (
                value.strip() and variable.startswith("CARGO_TARGET_") and variable.endswith("_RUSTFLAGS")):
            environment[variable] = add_lint_flag(value)
    return overrides


def probe_rustflags(root, environment):
    if "CARGO_ENCODED_RUSTFLAGS" in environment:
        key = "CARGO_ENCODED_RUSTFLAGS"
        environment[key] = add_lint_flag(environment[key], "\x1f")
        return key, []
    if "RUSTFLAGS" in environment:
        environment["RUSTFLAGS"] = add_lint_flag(environment["RUSTFLAGS"])
        return "RUSTFLAGS", []
    return "Cargo configuration", config_rustflags(root, environment)


def build_contexts(packages, rustflags_options):
    command = ["cargo", "check", "--offline", "--all-targets", "--keep-going", "--message-format=json",
               *rustflags_options]
    for package in packages:
        command.extend(["--package", package["name"]])
    selections = [("default-features", [])]
    if any(set(package["features"]) - {"default"} for package in packages):
        selections.append(("all-features", ["--all-features"]))
    if any(package["features"].get("default") for package in packages):
        selections.append(("no-default-features", ["--no-default-features"]))
    return {name: {"command": [*command, *flags], "baseline": {"state": "pending"},
                   "batch": {"state": "pending"}, "edits": []}
            for name, flags in selections}


def normalized_path(root, value):
    path = Path(value.replace("\\", "/"))
    if not path.is_absolute():
        return path.as_posix()
    return path.relative_to(root).as_posix() if path.is_relative_to(root) else path.as_posix()


def cargo_messages(path):
    with path.open(encoding="utf-8", errors="replace") as log:
        for line in log:
            if line.startswith("error:"):
                yield {"reason": "cargo-error", "message": line.strip()}
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
            output = {"artifacts": [], "diagnostics": [], "cargo_errors": [], "cargo_success": None}
            for row in cargo_messages(log):
                if row["reason"] == "compiler-artifact":
                    output["artifacts"].append({
                        "package_id": row["package_id"], "target": target_key(row["target"]),
                        "source": normalized_path(root, row["target"]["src_path"]),
                        "features": row["features"], "test": row["profile"]["test"], "fresh": row["fresh"],
                    })
                elif row["reason"] == "compiler-message":
                    diagnostic = row["message"]
                    output["diagnostics"].append({
                        "package_id": row["package_id"], "target": target_key(row["target"]),
                        **normalize_diagnostic(root, diagnostic),
                    })
                elif row["reason"] == "cargo-error":
                    output["cargo_errors"].append(row["message"])
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


class OriginalOffsets:
    def __init__(self, edits):
        self.extra = len(RESTRICTED) - len(VISIBILITY)
        starts = defaultdict(list)
        for edit in edits:
            starts[edit["path"]].append(edit["byte_start"])
        self.starts = {
            path: [start + index * self.extra for index, start in enumerate(sorted(positions))]
            for path, positions in starts.items()
        }

    def offset(self, path, value):
        starts = self.starts.get(path, ())
        index = bisect_right(starts, value) - 1
        if index < 0:
            return value
        start = starts[index] - index * self.extra
        narrowed_offset = value - starts[index]
        if narrowed_offset <= len(RESTRICTED):
            return start + min(narrowed_offset, len(VISIBILITY))
        return value - (index + 1) * self.extra

    def span_key(self, span):
        path = span["path"]
        return (path, self.offset(path, span["byte_start"]), self.offset(path, span["byte_end"]))


def span_key(span):
    return span["path"], span["byte_start"], span["byte_end"]


def target_identity(row):
    return row["package_id"], row["target"]["name"], tuple(row["target"]["kind"])


def outside_defining_target(diagnostic, declaration, producer_id):
    return diagnostic["package_id"] != producer_id or all(
        target != diagnostic["target"] for target, _ in declaration.origins)


def diagnostic_key(diagnostic, key_for_span):
    ranges = tuple((*key_for_span(span), span["is_primary"]) for span in spans(diagnostic))
    return (*target_identity(diagnostic),
            diagnostic["level"], diagnostic["code"], diagnostic["message"], ranges)


def artifact_key(artifact):
    return (*target_identity(artifact), artifact["test"], tuple(artifact["features"]))


def new_diagnostics(baseline, batch, offsets):
    original = Counter(diagnostic_key(row, span_key) for row in baseline["diagnostics"])
    for row in batch["diagnostics"]:
        key = diagnostic_key(row, offsets.span_key)
        if original[key]:
            original[key] -= 1
        else:
            yield row


def referring_node(discovery, span, offsets):
    if not (discovery.root / span["path"]).resolve().is_relative_to(discovery.root):
        return None
    try:
        original, tree = discovery.source(span["path"])
    except (OSError, ValueError):
        return None
    _, start, stop = offsets.span_key(span)
    if span["macro_expansion"] or not 0 <= start < stop <= len(original):
        return None
    return tree.descendant_for_byte_range(start, stop)


def exact_primary_span(diagnostic):
    primary = [span for span in diagnostic["spans"] if span["is_primary"]]
    return primary[0] if len(primary) == 1 and not primary[0]["macro_expansion"] else None


def associated_call(discovery, diagnostic, offsets):
    span = exact_primary_span(diagnostic)
    if span is None:
        return None
    node = referring_node(discovery, span, offsets)
    _, start, stop = offsets.span_key(span)
    if node is None or node.type != "scoped_identifier" or (node.start_byte, node.end_byte) != (start, stop):
        return None
    owner, name = (node.child_by_field_name(field) for field in ("path", "name"))
    call = node.parent
    if (call.type == "call_expression" and call.child_by_field_name("function") == node
            and all(part is not None and part.type == "identifier" for part in (owner, name))):
        return text(owner).removeprefix("r#"), text(name).removeprefix("r#")
    return None


def retry_candidate(discovery, producer, selected, context):
    offsets = OriginalOffsets(context["edits"])
    methods = defaultdict(list)
    for index, (declaration, _) in enumerate(selected):
        if declaration.role == "inherent" and len(declaration.enclosing) == 1:
            owner, = declaration.enclosing
            methods[owner.removeprefix("r#"), declaration.name.removeprefix("r#")].append(index)

    matches = defaultdict(list)
    diagnostics = context["batch"]["diagnostics"]
    for index in context["genuine_error_indices"]:
        diagnostic = diagnostics[index]
        if diagnostic["code"] != "E0308":
            continue
        for method in methods.get(associated_call(discovery, diagnostic, offsets), ()):
            matches[method].append(index)
    if len(matches) == 1:
        method, = matches
        declaration, result = selected[method]
        references = [index for index in matches[method]
                      if outside_defining_target(diagnostics[index], declaration, producer["id"])]
        if references and result["activity"] == "confirmed":
            return method, references
    return None


def retry_resolution(package_names, context, retry):
    baseline = context["baseline"]
    diagnostics = context["batch"]["diagnostics"]
    batch = retry["batch"]
    if (batch["state"] not in {"succeeded", "failed"}
            or batch.get("cargo_success") is not (batch["state"] == "succeeded")):
        return None

    calls = retry["trigger_diagnostic_indices"]
    affected = {target_identity(diagnostics[index]) for index in calls}
    units = Counter(target_identity(artifact) for artifact in baseline["artifacts"])
    artifacts = {artifact_key(artifact) for artifact in batch["artifacts"]}
    reached = {target_identity(artifact) for artifact in baseline["artifacts"] if artifact_key(artifact) in artifacts}
    reached.update(target_identity(row) for row in batch["diagnostics"] if row["level"] == "error")
    if any(units[target] != 1 or target not in reached for target in affected):
        return None
    retry_errors = set(retry["genuine_error_indices"])
    if any(target_identity(batch["diagnostics"][index]) in affected for index in retry_errors):
        return None

    # A method lookup error can become a precise privacy error at the same method token.
    retry_offsets = OriginalOffsets(retry["edits"])
    private_sites = {(target_identity(row), retry_offsets.span_key(span))
                     for index, row in enumerate(batch["diagnostics"])
                     if index not in retry_errors and row["level"] == "error" and row["code"] == "E0624"
                     and (span := exact_primary_span(row)) is not None}
    resolved = set(calls)
    offsets = OriginalOffsets(context["edits"])
    for index in context["genuine_error_indices"]:
        diagnostic = diagnostics[index]
        target = target_identity(diagnostic)
        if diagnostic["code"] != "E0599" or target not in affected:
            continue
        span = exact_primary_span(diagnostic)
        if span is not None and (target, offsets.span_key(span)) in private_sites:
            resolved.add(index)

    unresolved_packages = {diagnostics[index]["package_id"] for index in context["genuine_error_indices"]
                           if index not in resolved}
    resolved_packages = {package_names[package_id] for package_id, _, _ in affected
                         if package_id not in unresolved_packages and package_id in package_names}
    resolved_cargo = [error for error in context["cargo_failures"] if (summary := COMPILATION_SUMMARY.match(error))
                      and summary[1] in resolved_packages]
    return sorted(resolved), resolved_cargo


def effective_build(context):
    match context:
        case {
            "retry": {"state": "accepted", "resolved_error_indices": list(indices),
                      "resolved_cargo_failures": list(cargo_failures), "batch": {"state": "succeeded" | "failed"} as build},
            "genuine_error_indices": original_indices, "cargo_failures": original_failures,
        } if indices and indices == original_indices and cargo_failures == original_failures:
            return "retry", build
    return "batch", context["batch"]


def referring_site(discovery, span, offsets):
    node = referring_node(discovery, span, offsets)
    if node is None:
        return "unresolved referring site"
    while node is not None:
        if node.type == "use_declaration":
            public = any(child.type == "visibility_modifier" and child.text == b"pub"
                         for child in node.named_children)
            return "re-export" if public else "import"
        node = node.parent
    return "direct reference"


def glob_privacy_definition(discovery, diagnostic, primary, offsets, producer, library):
    if len(primary) != 1 or library is None or (
            diagnostic["package_id"] == producer["id"] and diagnostic["target"] == library["target"]):
        return None
    site = referring_node(discovery, primary[0], offsets)
    _, start, stop = offsets.span_key(primary[0])
    if site is None or site.type != "identifier" or (site.start_byte, site.end_byte) != (start, stop):
        return None
    name = site.text.decode().removeprefix("r#")
    if diagnostic["message"] != f"function `{name}` is private":
        return None
    # rustc can point to the glob's module identifier instead of the function definition.
    notes = {offsets.span_key(span) for note in diagnostic["children"]
             if note["level"] == "note" and note["message"] == f"the function `{name}` is defined here"
             for span in note["spans"] if span["is_primary"] and not span["macro_expansion"]}
    if len(notes) != 1:
        return None
    return discovery.local_glob_definition(notes.pop(), name, producer["edition"], library)


def build_reach(baseline, batch):
    finished = [target for target in batch["artifacts"] if target["package_id"].startswith("path+")]
    compiled = {artifact_key(target) for target in finished}
    diagnosed = {target_identity(row) for row in batch["diagnostics"]}
    unfinished = [{**target, "state": "diagnostics emitted" if target_identity(target) in diagnosed
                   else "no compiler output observed"}
                  for target in baseline["artifacts"]
                  if target["package_id"].startswith("path+") and artifact_key(target) not in compiled]
    return finished, unfinished


def record_comparison(discovery, producer, package_names, selected, baseline, context):
    batch = context["batch"]
    producer_id = producer["id"]
    libraries = [unit for unit in baseline["artifacts"]
                 if unit["package_id"] == producer_id and unit["target"]["kind"] == ["lib"] and not unit["test"]]
    library = libraries[0] if len(libraries) == 1 else None
    context["compiled_targets"], context["unfinished_targets"] = build_reach(baseline, batch)
    offsets = OriginalOffsets(context["edits"])
    definitions = {span_key(span): index for index, (declaration, _) in enumerate(selected)
                   for span in (declaration.span, declaration.name_span, declaration.signature_span)
                   if span is not None}
    expected_errors = set()
    genuine_errors = []
    unresolved = []
    batch["diagnostics"] = list(new_diagnostics(baseline, batch, offsets))
    for index, diagnostic in enumerate(batch["diagnostics"]):
        primary = [span for span in diagnostic["spans"] if span["is_primary"]]
        code = diagnostic["code"]
        level = diagnostic["level"]
        definition_spans = []
        kind = None
        if code == "dead_code" and level == "warning" and diagnostic["package_id"] == producer_id:
            kind, definition_spans = "dead_code", primary
        elif code in PRIVACY_CODES and level == "error":
            kind = "privacy"
            definition_spans = [
                *(span for span in diagnostic["spans"] if not span["is_primary"]),
                *(span for child in diagnostic["children"] for span in spans(child)),
            ]

        keys = {offsets.span_key(span) for span in definition_spans if not span["macro_expansion"]}
        matches = {definitions[key] for key in keys.intersection(definitions)}
        if kind == "privacy" and code == "E0603" and not matches:
            resolved = glob_privacy_definition(discovery, diagnostic, primary, offsets, producer, library)
            if resolved is not None:
                match = definitions.get(span_key(resolved))
                if match is not None:
                    matches.add(match)
        if kind == "privacy" and len(matches) != 1:
            matches.clear()
        expected = False
        for match in sorted(matches):
            declaration, result = selected[match]
            observation = {"kind": kind, "diagnostic_index": index}
            if kind == "privacy":
                observation["referring_sites"] = []
                for span in primary:
                    _, start, stop = offsets.span_key(span)
                    observation["referring_sites"].append({
                        "kind": referring_site(discovery, span, offsets),
                        "span": {**span, "byte_start": start, "byte_end": stop},
                    })
                expected = outside_defining_target(diagnostic, declaration, producer_id)
                if expected:
                    expected_errors.add(diagnostic["package_id"])
            result["observations"].append(observation)
        if level == "error" and not expected:
            genuine_errors.append(index)
        if kind is not None and not matches:
            unresolved.append(index)

    # Cargo also reports failures outside rustc's JSON stream.
    packages_with_privacy = {package_names[package_id] for package_id in expected_errors if package_id in package_names}
    cargo_failures = []
    for error in batch["cargo_errors"]:
        summary = COMPILATION_SUMMARY.match(error)
        if summary is None or summary[1] not in packages_with_privacy:
            cargo_failures.append(error)
    context["genuine_error_indices"] = genuine_errors
    context["unresolved_diagnostic_indices"] = unresolved
    context["cargo_failures"] = cargo_failures
    if batch["state"] == "failed":
        batch["failure_kind"] = ("genuine" if genuine_errors or cargo_failures or not expected_errors
                                 else "visibility")
