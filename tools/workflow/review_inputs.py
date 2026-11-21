"""Input identities for independent review routing."""

from contextlib import contextmanager
from hashlib import sha256
import os
from pathlib import Path
import json
import re
import subprocess
from tempfile import TemporaryDirectory

from jsonschema import Draft202012Validator

from guidance import check_candidate
from result import SCHEMA_PATH, portable, references, render, validate


SCHEMA = json.loads(SCHEMA_PATH.read_bytes())
LANE_SKILLS = {contract["lane"]: skill for skill, contract in SCHEMA["x-skill-contracts"].items()
               if contract["kind"] == "review" and contract["agent"] == "TA-code-reviewer"
               and skill.startswith("TA-review-")}
REFERENCE = SCHEMA["properties"]["inputs"]["properties"]["plans"]["items"]
CANDIDATE = SCHEMA["$defs"]["guidance"]["properties"]["current_candidate"]["anyOf"][0]
STRING = {"type": "string", "minLength": 1}
SLUG = SCHEMA["properties"]["task_id"]


class ReviewInputError(ValueError):
    """A required input identity or judgment is absent or inconsistent."""


class UnavailableArtifact(ReviewInputError):
    """The required artifact cannot be read with its recorded identity."""

    def __init__(self, ref, reason):
        super().__init__(reason)
        self.ref = ref


class MissingArtifact(UnavailableArtifact):
    """The named artifact file was confirmed absent."""


def object_fields(required, optional=None):
    return {"type": "object", "properties": {**required, **(optional or {})},
            "required": list(required), "additionalProperties": False}


def array(value, minimum=0):
    return {"type": "array", "items": value, "minItems": minimum}


def choices(*values):
    return {"enum": list(values)}


def nullable(value):
    return {"anyOf": [value, {"type": "null"}]}


KEY = object_fields({"lane": choices(*LANE_SKILLS), "id": STRING})
LANE_INPUT = object_fields({
    "lane": choices(*LANE_SKILLS), "scope": array(STRING, 1),
    "requirements": array(REFERENCE, 1), "criteria": array(REFERENCE, 1),
    "partition": nullable(REFERENCE), "evidence": array(REFERENCE),
})
LANE_STATE = object_fields({
    "input": REFERENCE, "state": choices("due", "running", "returned", "failed", "missing"),
}, {"report": REFERENCE, "reason": STRING})
GATE = object_fields({
    "id": STRING, "state": choices("due", "running", "passed", "failed", "missing"), "impact": STRING,
}, {"evidence": REFERENCE})
ROUND = object_fields({
    "task_id": SLUG, "work_item_id": SLUG, "number": {"type": "integer", "minimum": 1},
    "previous": nullable(REFERENCE), "candidate": CANDIDATE, "patch": REFERENCE,
    "lane_states": {"type": "object", "propertyNames": choices(*LANE_SKILLS), "additionalProperties": LANE_STATE},
    "gates": array(GATE),
}, {"started_by": REFERENCE, "earlier_reports": array(REFERENCE), "source_results": array(REFERENCE)})
EFFECT = object_fields({
    "facet": choices("wording", "behavior", "architecture", "evidence", "requirements", "unknown"),
    "reason": STRING, "evidence": array(REFERENCE, 1),
})
ASSESSMENT = object_fields({
    "finding": KEY, "source": REFERENCE, "owner": choices(*LANE_SKILLS),
    "kind": choices("required", "deferrable", "suggestion"), "impact": STRING,
    "basis": STRING, "evidence": array(REFERENCE, 1),
}, {"duplicate_of": KEY})
REQUEST = object_fields({
    "task_id": SLUG, "work_item_id": SLUG,
    "operation": choices("batch", *LANE_SKILLS.values()),
    "head": REFERENCE, "assessments": array(ASSESSMENT), "selected_findings": array(KEY),
}, {"repair": object_fields({"before": REFERENCE, "after": REFERENCE, "paths": array(STRING),
                             "effects": array(EFFECT, 1)}),
    "decision": REFERENCE, "challenge_report": REFERENCE, "prior_challenge_reports": array(REFERENCE),
    "risk_decisions": array({"anyOf": [REFERENCE, object_fields({"finding": KEY, "evidence": REFERENCE})]})})
ROUTING = next(item["properties"]["routing"] for item in SCHEMA["properties"]["result"]["oneOf"]
               if item["properties"].get("kind", {}).get("const") == "review")
CONTEXT = object_fields({
    "id": STRING, "run_id": SLUG, "fresh": {"const": True}, "inherited_author_history": {"const": False},
})


def check_shape(value, schema, name):
    rows = sorted(f"{'/'.join(map(str, error.path))}: {error.message}"
                  for error in Draft202012Validator(schema).iter_errors(value))
    if rows:
        raise ReviewInputError(f"{name}: {'; '.join(rows)}")
    return value


def current_route_path(task, item):
    return (Path(".agent/tmp/workflow-TA") / task / item / "review-routing" / "current.json").as_posix()


def fingerprint(value):
    return sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def normalize_candidate(candidate):
    if not candidate["scope"] or len(set(candidate["scope"])) != len(candidate["scope"]):
        raise ReviewInputError("candidate scope must be nonempty and unique")
    paths = [ref["path"] for ref in candidate["untracked"]]
    if len(set(paths)) != len(paths) or any(ref["sha256"] is None for ref in candidate["untracked"]):
        raise ReviewInputError("candidate untracked files require unique paths and exact hashes")
    if any(not portable(path) for path in (*candidate["scope"], *paths)):
        raise ReviewInputError("candidate scope and file paths must be portable")
    if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", candidate["base_commit"]):
        raise ReviewInputError("candidate base must be an immutable Git commit")
    return {**candidate, "scope": sorted(candidate["scope"]),
            "untracked": sorted(candidate["untracked"], key=lambda ref: ref["path"])}


def qualified(key):
    return key["lane"], key["id"]


def qualification(task, item, key):
    return {"task_id": task, "work_item_id": item, "lane": key[0], "id": key[1]}


def snapshot_hash_matches(content, expected):
    # A Windows checkout can hash CRLF text while Git stores LF.
    return sha256(content).hexdigest() == expected or (
        b"\0" not in content and b"\r" not in content
        and sha256(content.replace(b"\n", b"\r\n")).hexdigest() == expected
    )


def batch_objects(git, queries):
    """Read historical blobs and check baseline object existence in one Git process."""
    requests = tuple((command, ref.encode("utf-8")) for command, ref in queries)
    process = git("cat-file", "--batch-command", "-z",
                  patch=b"".join(command + b" " + ref + b"\0" for command, ref in requests))
    if process.returncode:
        raise ReviewInputError("cannot read objects for the preserved candidate")
    output = process.stdout
    position = 0
    for command, ref in requests:
        missing = ref + b" missing\n"
        if output.startswith(missing, position):
            position += len(missing)
            yield None, None
            continue
        header_end = output.find(b"\n", position)
        if header_end == -1:
            raise ReviewInputError("invalid Git object response for the preserved candidate")
        header = output[position:header_end].split(b" ")
        if len(header) != 3 or not header[2].isdigit():
            raise ReviewInputError("invalid Git object response for the preserved candidate")
        _, kind, size = header
        position = header_end + 1
        content = None
        if command == b"contents":
            end = position + int(size)
            if output[end:end + 1] != b"\n":
                raise ReviewInputError("invalid Git object response for the preserved candidate")
            content = output[position:end]
            position = end + 1
        yield kind, content
    if position != len(output):
        raise ReviewInputError("invalid Git object response for the preserved candidate")


class Artifacts:
    def __init__(self, root):
        self.root = root

    def path(self, path):
        location = self.root / path
        if not portable(path) or not location.resolve().is_relative_to(self.root):
            raise ReviewInputError(f"reference escapes the selected repository: {path}")
        return location

    def read(self, ref):
        location = self.path(ref["path"])
        if ref["sha256"] is None:
            raise ReviewInputError(f"reference needs a portable path and content identity: {ref['path']}")
        try:
            content = location.read_bytes()
        except FileNotFoundError as error:
            raise MissingArtifact(ref, f"unavailable reference {ref['path']}: {error}") from error
        except OSError as error:
            raise UnavailableArtifact(ref, f"unavailable reference {ref['path']}: {error}") from error
        if sha256(content).hexdigest() != ref["sha256"]:
            raise UnavailableArtifact(ref, f"changed reference: {ref['path']}")
        return content

    def read_json(self, ref, schema=None):
        content = self.read(ref)
        try:
            data = json.loads(content)
        except (UnicodeError, ValueError) as error:
            raise ReviewInputError(f"{ref['path']}: {error}") from error
        return check_shape(data, schema, ref["path"]) if schema is not None else data

    def check_all(self, value):
        for ref in references(value):
            self.read(ref)

    def check_snapshot_evidence(self, refs, record):
        """Check explicit evidence against the candidate that supplied it."""
        candidate = record["candidate"]
        untracked = {ref["path"]: ref["sha256"] for ref in candidate["untracked"]}
        tracked_or_private = []
        for ref in refs:
            path = ref["path"]
            self.path(path)
            if path not in untracked:
                tracked_or_private.append(ref)
            elif ref["sha256"] != untracked[path]:
                raise UnavailableArtifact(ref, f"changed reference: {path}")
        if not tracked_or_private:
            return

        with self.scratch_git() as git:
            self.load_index(git, record)
            queries = [
                query
                for ref in tracked_or_private
                for query in ((b"contents", f":0:{ref['path']}"), (b"info", f"{candidate['base_commit']}:{ref['path']}"))
            ]
            objects = list(batch_objects(git, queries))
            for ref, (kind, content), (base_kind, _) in zip(
                    tracked_or_private, objects[::2], objects[1::2], strict=True):
                path = ref["path"]
                if kind == b"blob":
                    if not snapshot_hash_matches(content, ref["sha256"]):
                        raise UnavailableArtifact(ref, f"changed reference: {path}")
                elif base_kind is not None:
                    raise UnavailableArtifact(ref, f"reference absent from preserved candidate: {path}")
                else:
                    # Private plans and other external inputs can lie under a broad scope.
                    self.read(ref)

    @contextmanager
    def scratch_git(self):
        # Git writes only to the disposable index and object store. The
        # repository's existing objects are available through an alternate.
        objects = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-path", "objects"],
                                 cwd=self.root, capture_output=True)
        if objects.returncode:
            raise ReviewInputError("cannot locate Git objects for the preserved candidate")
        with TemporaryDirectory() as folder:
            scratch = Path(folder)
            database = scratch / "objects"
            (database / "info").mkdir(parents=True)
            (database / "info" / "alternates").write_bytes(objects.stdout)
            env = {**os.environ, "GIT_INDEX_FILE": str(scratch / "index"),
                   "GIT_OBJECT_DIRECTORY": str(database)}
            def git(*args, patch=None):
                return subprocess.run(["git", "-c", "core.splitIndex=false", "-c", "core.fsmonitor=false", *args],
                                      cwd=self.root, env=env, input=patch, capture_output=True)
            yield git

    def load_index(self, git, record):
        base = record["candidate"]["base_commit"]
        patch = self.read(record["patch"])
        if patch and not patch.startswith(b"diff --git "):
            raise ReviewInputError("candidate patch is not a native binary Git diff")
        if git("read-tree", "--reset", base).returncode or (
            patch and git("apply", "--cached", "--whitespace=nowarn", "-", patch=patch).returncode
        ):
            raise ReviewInputError("cannot reconstruct the preserved candidate from its Git patch")

    def report(self, ref):
        result = self.read_json(ref)
        problems = validate(result, SCHEMA, self.root, check_references=False)
        if problems:
            raise ReviewInputError(f"{ref['path']}: {'; '.join(problems)}")
        text = (self.root / ref["path"]).with_suffix(".md")
        if not text.resolve().is_relative_to(self.root):
            raise ReviewInputError(f"{ref['path']}: the paired text escapes the selected repository")
        text_ref = {"path": text.relative_to(self.root).as_posix(), "sha256": None}
        try:
            observed = text.read_text(encoding="utf-8")
        except FileNotFoundError as error:
            raise MissingArtifact(text_ref, f"unavailable paired text {text_ref['path']}: {error}") from error
        except (OSError, UnicodeError) as error:
            raise UnavailableArtifact(text_ref, f"unavailable paired text {text_ref['path']}: {error}") from error
        if observed != render(result):
            raise UnavailableArtifact(text_ref, f"{ref['path']}: the review's paired text differs")
        return result

    def history(self, request):
        rounds = []
        visited = set()
        ref = request["head"]
        while ref:
            if ref["path"] in visited:
                raise ReviewInputError("round history contains a repeated or cyclic entry")
            visited.add(ref["path"])
            record = self.read_json(ref, ROUND)
            if any(record[name] != request[name] for name in ("task_id", "work_item_id")):
                raise ReviewInputError("round history belongs to a different task or work item")
            record["candidate"] = normalize_candidate(record["candidate"])
            patch = self.read(record["patch"])
            if sha256(patch).hexdigest() != record["candidate"]["diff_sha256"]:
                raise ReviewInputError("history patch differs from its candidate identity")
            rounds.append((ref, record))
            ref = record["previous"]
        rounds.reverse()
        for number, (_, record) in enumerate(rounds, start=1):
            if record["number"] != number:
                raise ReviewInputError("round history is incomplete or its cumulative number conflicts")
        errors = []
        check_candidate(rounds[-1][1]["candidate"], self.root, errors)
        if errors:
            raise ReviewInputError("; ".join(errors))
        return rounds

    def changed_paths(self, before, after):
        """Compare the reconstructed tracked trees and the exact untracked identities."""
        with self.scratch_git() as git:
            trees = []
            for record in (before, after):
                self.load_index(git, record)
                tree = git("write-tree")
                if tree.returncode:
                    raise ReviewInputError("cannot identify the preserved candidate's tracked tree")
                trees.append(tree.stdout.decode("ascii").strip())
            diff = git("diff-tree", "--no-commit-id", "--name-only", "-r", "--no-renames", "-z", *trees)
            if diff.returncode:
                raise ReviewInputError("Git cannot compare the preserved candidate trees")
            tracked = set(diff.stdout.decode("utf-8").split("\0")) - {""}
        previous, current = ({ref["path"]: ref["sha256"] for ref in record["candidate"]["untracked"]}
                             for record in (before, after))
        untracked = {path for path in previous.keys() | current.keys() if previous.get(path) != current.get(path)}
        return sorted(tracked | untracked)
