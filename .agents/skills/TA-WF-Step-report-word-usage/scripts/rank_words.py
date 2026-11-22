"""Rank word forms in the working copies of selected project Markdown files."""

import argparse
from collections import Counter, defaultdict
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile
import unicodedata

from markdown_text import blank_span, prose_lines


ROOT_FILES = {
    "AGENTS.md": "agent_instructions",
    "README.md": "project_docs",
    "ROADMAP.md": "project_docs",
    "DIARY.md": "diary",
}
DIRECTORIES = {
    ".agents/references": "agent_instructions",
    ".agents/skills": "agent_instructions",
    ".codex/agents": "agent_instructions",
    "Obsidian/Amiga/Design": "design",
    "docs": "project_docs",
    "tools/amiga-rag-mcp-server": "tool_docs",
    "tools/artifacts": "tool_docs",
    "tools/blep_generator": "tool_docs",
    "tools/bootstrap": "tool_docs",
    "tools/graphify": "tool_docs",
    "tools/verification": "tool_docs",
    "tools/workflow": "tool_docs",
    "crates/machine_loop/tests": "test_docs",
    "crates/test_runner/tests/overrides/singlestep": "test_docs",
    "benchmarks/m68k": "m68k_docs",
    "examples/m68k": "m68k_docs",
}
SOURCE_ROOTS = ROOT_FILES | DIRECTORIES
SCOPE_ANCESTORS = {parent.as_posix() for path in DIRECTORIES
                   for parent in PurePosixPath(path).parents}
EXCLUDED_DIRECTORIES = (
    "tools/bootstrap/books",
    "tools/blep_generator/output",
    "tools/AmigaTestKit",
    "tools/m68k-asm",
    "Obsidian/Amiga/Reference",
    "Obsidian/Amiga/References",
    "tests/singlestep",
    "tests/benchmarks/quick",
    "tests/benchmarks/standard",
    "tests/benchmarks/thorough",
    "benchmarks/results",
)
EXCLUDED_DIRECTORY_NAMES = {
    ".agent", ".git", ".obsidian", "node_modules", "target", "build", "_build", "build-output",
    "dist", "graphify-out", "__pycache__", "cache", "caches", ".cache",
    ".mypy_cache", ".pytest_cache", ".ruff_cache", ".venv", "venv",
    ".virtualenv", "virtualenv", ".tox", ".nox",
}
REFERENCE_DIRECTORY_NAMES = {"refsrc", "refsource", "refsources", "revsrc", "revsource",
                             "revsources", "referencesrc", "referencesource", "referencesources"}
GENERATED_REPORTS = {"benchmarks/m68k_benchmark_anomaly_report.md"}
ASSETS = Path(__file__).resolve().parents[1] / "assets"
FILTER_FILES = {"common": ASSETS / "common-words.txt",
                "project": ASSETS / "project-words.txt"}
DEFAULT_OUTPUT = Path(".agent/tmp/word-usage/report.json")
REPORT_NAME = "project-word-usage"
REPORT_HEADER = f'{{\n  "report": "{REPORT_NAME}",\n'.encode()

NONPROSE = re.compile(
    r"</?[A-Za-z][^<>\n]*>|<[^<>\s]+>|\[\^[^\]\n]+\]"
    r"|(?:[A-Za-z][\w+.-]*://|mailto:|www\.)[^\s<>`|]+"
    r"|[\w.+%~-]+@[\w.-]+\.[A-Za-z]{2,}"
    r"|[\w.~:+%-]*[/\\][\w.~:/\\%+-]*"
    r"|[\w-]+(?:(?:::|\.)[\w-]+)+"
    r"|(?<!\w)(?:--[\w-]+|#[\w/-]+)"
)
WORD = re.compile(r"\w+(?:['’]\w+)*")
ASCII_WORD = re.compile(r"[A-Za-z]+(?:['’][A-Za-z]+)*")
COMPOUND_CASE = re.compile(r"[a-z][A-Z]|[A-Z]{2}[a-z]")


def ignored_directory(path):
    lower = path.casefold()
    name = lower.rsplit("/", 1)[-1]
    return (name in EXCLUDED_DIRECTORY_NAMES
            or name.replace("_", "").replace("-", "") in REFERENCE_DIRECTORY_NAMES
            or any(lower == directory.casefold() for directory in EXCLUDED_DIRECTORIES))


def source_scope(path):
    """Return the included scope root for an eligible Git-relative Markdown path."""
    if not path.casefold().endswith(".md") or path.casefold() in GENERATED_REPORTS:
        return None
    if path in ROOT_FILES:
        return path
    for parent in PurePosixPath(path).parents:
        directory = parent.as_posix()
        if ignored_directory(directory):
            break
        if directory in DIRECTORIES:
            return directory
    return None


def git(root, *args):
    result = subprocess.run(["git", *args], cwd=root, capture_output=True, check=False)
    if result.returncode:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise ValueError(message or f"Git exited with status {result.returncode}")
    return result.stdout


def selected_paths(root):
    if git(root, "rev-parse", "--show-prefix").strip():
        raise ValueError("Select the Git checkout root with --root.")
    scope = [f":(top,literal){path}" for path in SOURCE_ROOTS]
    excluded = [f":(top,exclude,literal){path}" for path in EXCLUDED_DIRECTORIES]
    listed = git(root, "ls-files", "-z", "--cached", "--others", "--exclude-standard",
                 "--", *scope, *excluded)
    return [(path, location) for path in sorted(set(os.fsdecode(listed).split("\0")))
            if (location := source_scope(path)) is not None]


def discover_unclassified(root, output):
    """Inventory names outside the configured scope without opening Markdown files."""
    directories = {}
    issues = []

    def record_error(error):
        directory = Path(error.filename).relative_to(root).as_posix()
        issues.append({"directory": directory, "message": error.strerror})

    for parent, children, files in os.walk(root, onerror=record_error):
        base = Path(parent).relative_to(root)
        children[:] = sorted(name for name in children if not ignored_directory((base / name).as_posix())
                             and (base / name).as_posix() not in DIRECTORIES)
        for name in sorted(files):
            path = (base / name).as_posix()
            if (not name.casefold().endswith(".md") or path in ROOT_FILES
                    or path.casefold() in GENERATED_REPORTS or root / path == output):
                continue
            directory = next((folder.as_posix() for folder in reversed([base, *base.parents])
                              if folder.as_posix() not in SCOPE_ANCESTORS), base.as_posix())
            summary = directories.setdefault(directory, {"directory": directory, "markdown_count": 0,
                                                          "examples": []})
            summary["markdown_count"] += 1
            if len(summary["examples"]) < 3:
                summary["examples"].append(path)
    return [directories[name] for name in sorted(directories)], issues


def words(line):
    line = NONPROSE.sub(lambda match: blank_span(match[0]), line)
    for match in WORD.finditer(line):
        raw = match[0]
        if not ASCII_WORD.fullmatch(raw) or COMPOUND_CASE.search(raw):
            continue
        word = raw.lower().replace("’", "'").removesuffix("'s")
        if len(word.replace("'", "")) < 3:
            continue
        neighbors = line[max(0, match.start() - 1):match.start()] + line[match.end():match.end() + 1]
        if any(unicodedata.category(char).startswith("M") for char in neighbors):
            continue
        yield word, match.start()


def read_filter(path):
    return {word for line in path.read_text(encoding="utf-8-sig").splitlines()
            if (word := line.partition("#")[0].strip().lower())}


def sample_context(line, position):
    start = max(0, position - 70)
    end = start + 160
    return ("…" if start else "") + line[start:end].strip() + ("…" if end < len(line) else "")


def build_report(root, paths, filters, limit):
    excluded = set().union(*filters.values())
    counts, documents = Counter(), Counter()
    samples = defaultdict(list)
    scope = {group: {"document_count": 0, "token_count": 0, "candidate_count": 0}
             for group in sorted(set(SOURCE_ROOTS.values()))}
    scope_roots = dict.fromkeys(sorted(SOURCE_ROOTS), 0)
    scanned, issues = [], []
    for path, location in paths:
        source = root / path
        try:
            if source.resolve(strict=True) != source:
                raise OSError("Markdown source is a redirected path.")
            lines = source.read_text(encoding="utf-8-sig").split("\n")
        except FileNotFoundError:
            issues.append({"path": path, "kind": "missing", "message": "Markdown source is missing."})
            continue
        except (OSError, UnicodeError) as error:
            issues.append({"path": path, "kind": "read_error",
                           "message": getattr(error, "strerror", None) or str(error)})
            continue

        scanned.append(path)
        scope_roots[location] += 1
        group_summary = scope[SOURCE_ROOTS[location]]
        group_summary["document_count"] += 1
        in_document = Counter()
        for number, line in enumerate(prose_lines(lines), 1):
            for word, position in words(line):
                group_summary["token_count"] += 1
                if word in excluded:
                    continue
                if word not in in_document and len(samples[word]) < 3:
                    samples[word].append({"path": path, "line": number,
                                          "context": sample_context(lines[number - 1], position)})
                in_document[word] += 1
        counts.update(in_document)
        documents.update(in_document.keys())
        group_summary["candidate_count"] += in_document.total()

    top = sorted(counts, key=lambda word: (-counts[word], word))[:limit]
    return {
        "report": REPORT_NAME,
        "complete": not issues,
        "document_count": len(scanned),
        "scanned_paths": scanned,
        "issues": issues,
        "scope": scope,
        "scope_roots": scope_roots,
        "filters": {name: len(entries) for name, entries in filters.items()},
        "token_count": sum(group["token_count"] for group in scope.values()),
        "candidate_count": counts.total(),
        "distinct_candidate_count": len(counts),
        "ranking": [{"word": word, "count": counts[word], "document_count": documents[word],
                     "samples": samples[word]} for word in top],
    }


def safe_output(root, path):
    output = root / path
    if output.is_symlink():
        raise ValueError("Report output cannot be a symbolic link.")
    output = output.resolve()
    relative = output.relative_to(root).as_posix() if output.is_relative_to(root) else None
    if ((relative is not None and source_scope(relative) is not None) or
            output in FILTER_FILES.values()):
        raise ValueError("Report output cannot overwrite a source or word list.")
    try:
        with output.open("rb") as existing:
            if existing.read(len(REPORT_HEADER)) != REPORT_HEADER:
                raise ValueError("Report output already exists and is not a word usage report.")
    except FileNotFoundError:
        pass
    return output


def write_report(output, report):
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n", dir=output.parent,
                                         delete=False) as stream:
            temporary = Path(stream.name)
            json.dump(report, stream, ensure_ascii=True, indent=2)
            stream.write("\n")
        temporary.replace(output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="Git checkout root")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT,
                        help="JSON output path, relative to the selected root by default")
    parser.add_argument("--limit", type=int, default=20, help="number of ranking rows (default: 20)")
    args = parser.parse_args(argv)
    if args.limit < 1:
        parser.error("--limit must be a positive integer")

    try:
        root = args.root.resolve(strict=True)
        output = safe_output(root, args.output)
        paths = selected_paths(root)
        filters = {name: read_filter(path) for name, path in FILTER_FILES.items()}
        report = build_report(root, paths, filters, args.limit)
        report["unclassified_directories"], report["discovery_issues"] = discover_unclassified(root, output)
        write_report(safe_output(root, args.output), report)
    except (OSError, UnicodeError, ValueError) as error:
        print(f"Word usage report failed: {error}", file=sys.stderr)
        return 1

    print(f"Scanned {report['document_count']} Markdown files. "
          f"Counted {report['candidate_count']} word occurrences after filtering.")
    print(f"{'Rank':>4}  {'Word':<24} {'Count':>7} {'Documents':>10}")
    for rank, row in enumerate(report["ranking"], 1):
        print(f"{rank:>4}  {row['word']:<24} {row['count']:>7} {row['document_count']:>10}")
    for directory in report["unclassified_directories"]:
        print(f"Unclassified Markdown names in {directory['directory']}: {directory['markdown_count']}")
    if report["discovery_issues"]:
        print("The directory inventory encountered errors. See discovery_issues in the report.", file=sys.stderr)
    print(f"JSON report: {output}")
    if report["issues"]:
        print("The report is incomplete. See its issues for unreadable or missing files.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
