#!/usr/bin/env python3
"""Check English text and audit the project vocabulary that supports it."""

import argparse
from collections import defaultdict
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile
import unicodedata
from uuid import uuid4

try:
    from spellchecker import SpellChecker
except ImportError:
    SpellChecker = None

REPO_ROOT = Path(__file__).resolve().parents[3]
SCOPE_FILE = Path(__file__).with_suffix(".json")
REPORT_DIRECTORY = Path(".agent/tmp/english")
WORKER_SCAN = None
WORKER_DICTIONARY = None
WORD_PATTERN = re.compile(r"[^\W\d_][^\W_]*(?:['\u2019][^\W\d_]+)?", re.UNICODE)
VOCABULARY_ENTRY = re.compile(r"[^\W_]+(?:['\u2019][^\W_]+)?")
CASE_BOUNDARY = re.compile(r"(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])")
URL_PATTERN = re.compile(r"(?:https?://|mailto:)[^\s<>\"')\x60]+")
OPCODE_LITERAL_PATTERN = re.compile(r'("target_primary_opcode"\s*:\s*")[0-9a-fA-F]{4}(\")')
NUMBER_PATTERN = re.compile(
    r"\b0[xob][0-9a-fA-F_]+(?:[ui](?:8|16|32|64|128|size))?\b"
    r"|\$[0-9a-fA-FxXyY]+\b"
    r"|\b\d[\d_]*(?:\.\d+)?(?:[eE][+-]?\d+)?(?:[uif](?:8|16|32|64|128|size))?\b"
    r"|\b(?=[0-9a-fA-F]*\d)[0-9a-fA-F]{7,64}\b"
    r"|(?<!\w)#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b"
)
UNICODE_ESCAPE = re.compile(r"\\+u([0-9a-fA-F]{4})")
STRING_PREFIX = re.compile(r"\b(?:fr|rf|br|rb|[frbu])(?=['\"])", flags=re.IGNORECASE)
SOFT_HYPHEN = re.compile("\u00ad")
PIECE_BOUNDARY = re.compile(CASE_BOUNDARY.pattern + r"|\d+")
JSON_WHITESPACE = re.compile(r"[ \t\n\r]*")


def source_path(path, root):
    path = Path(os.path.abspath(path))
    return path.relative_to(root).as_posix() if path.is_relative_to(root) else path.as_posix()


@dataclass(frozen=True)
class Entry:
    path: str
    position: int
    spelling: str
    owner: str | None = None

    @property
    def term(self):
        return self.spelling.lower()

    @property
    def lookup(self):
        # Existing file allowances use the spelling as stored.
        return self.term if self.owner is None else self.spelling

    def identity(self):
        common = {"path": self.path, "spelling": self.spelling, "term": self.term}
        if self.owner is None:
            return {**common, "type": "word_list", "line": self.position}
        return {**common, "type": "file_word", "owner": self.owner, "position": self.position}


@dataclass
class Source:
    text: str
    digest: str

def read_source_file(path):
    data = path.read_bytes()
    text = data.decode("utf-8").replace("\r\n", "\n").replace("\r", "\n")
    return Source(text, hashlib.sha256(data).hexdigest())


def git_output(*args, allowed=(0,), input_bytes=None):
    result = subprocess.run(["git", *args], cwd=REPO_ROOT, input=input_bytes, capture_output=True)
    if result.returncode not in allowed:
        raise ValueError(result.stderr.decode("utf-8", errors="replace").strip()
                         or f"Git {args[0]} returned {result.returncode}")
    return result.stdout.decode("utf-8").split("\0")


def explicit_exclusions(scope):
    files = [REPO_ROOT / name for name in scope.get("always_excluded_files", ())]
    directories = [(REPO_ROOT / name).resolve() for name in scope["always_excluded_directories"]]
    def excluded(path):
        return (Path(os.path.abspath(path)) in files
                or any(path.resolve().is_relative_to(directory) for directory in directories))
    return excluded


def repository_eligible(relative, scope):
    path = PurePosixPath(relative)
    return (relative not in scope["excluded_files"]
            and relative not in scope.get("always_excluded_files", ())
            and not any(relative.startswith(f"{name}/") for name in scope["always_excluded_directories"])
            and not any(name in path.parts[:-1] for name in scope["excluded_directory_names"])
            and not any(relative.startswith(f"{name}/") for name in scope["excluded_directories"])
            and (path.suffix.lower() in scope["text_extensions"] or path.name in scope["text_filenames"]))


def repository_targets(scope):
    """Enumerate eligible tracked and nonignored new text, including missing tracked inputs."""
    patterns = [f":(glob,exclude)**/{name}/**" for name in scope["excluded_directory_names"]]
    patterns.extend(f":(glob,exclude){name}/**"
                    for name in scope["excluded_directories"] + scope["always_excluded_directories"])
    names = git_output("ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", *patterns)
    targets = (REPO_ROOT / name for name in sorted(set(names) - {""}) if repository_eligible(name, scope))
    return [target for target in targets if target.is_file() or not target.exists()]


def unique_json_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def json_object_positions(text, start):
    """Read the positions of an already validated JSON object's values."""
    decode = json.JSONDecoder().raw_decode
    cursor = JSON_WHITESPACE.match(text, start + 1).end()
    while text[cursor] != "}":
        name, cursor = decode(text, cursor)
        cursor = JSON_WHITESPACE.match(text, cursor).end()
        start = JSON_WHITESPACE.match(text, cursor + 1).end()
        _, end = decode(text, start)
        yield name, start
        cursor = JSON_WHITESPACE.match(text, end).end()
        if text[cursor] == ",":
            cursor = JSON_WHITESPACE.match(text, cursor + 1).end()


def file_word_positions(text):
    """Locate only the strings declared inside the root file_words object."""
    start = dict(json_object_positions(text, JSON_WHITESPACE.match(text).end())).get("file_words")
    if start is None:
        return
    decode = json.JSONDecoder().raw_decode
    for owner, start in json_object_positions(text, start):
        position = 0
        cursor = JSON_WHITESPACE.match(text, start + 1).end()
        while text[cursor] != "]":
            spelling, end = decode(text, cursor)
            yield owner, position, spelling, cursor, end
            position += 1
            cursor = JSON_WHITESPACE.match(text, end).end()
            if text[cursor] == ",":
                cursor = JSON_WHITESPACE.match(text, cursor + 1).end()


def validate_scope(scope, repository):
    if not isinstance(scope, dict):
        raise ValueError("English configuration must be a JSON object")
    keys = ["word_lists", "always_excluded_directories"]
    if repository:
        keys.extend(("excluded_directory_names", "excluded_directories", "excluded_files",
                     "text_extensions", "text_filenames"))
    if "always_excluded_files" in scope:
        keys.append("always_excluded_files")
    for key in keys:
        value = scope[key]
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            raise ValueError(f"{key} must be an array of strings")
    entries = scope.get("file_words", {})
    if not isinstance(entries, dict):
        raise ValueError("file_words must map exact file paths to lists of terms")
    for relative, terms in entries.items():
        if not relative or any(char in relative for char in "\\*?[]:"):
            raise ValueError(f"Invalid exact-file vocabulary path: {relative}")
        path = PurePosixPath(relative)
        if path.is_absolute() or ".." in path.parts or path.as_posix() != relative or relative == ".":
            raise ValueError(f"Invalid exact-file vocabulary path: {relative}")
        if not isinstance(terms, list):
            raise ValueError(f"Vocabulary for {relative} must be a list of individual terms")
        for word in terms:
            if not isinstance(word, str) or VOCABULARY_ENTRY.fullmatch(word) is None:
                raise ValueError(f"Invalid vocabulary entry in {relative}: {word}")


class VocabularyError(ValueError):
    def __init__(self, path, error):
        super().__init__(str(error))
        self.path = path


def load_vocabulary(scope, config=None, read_source=read_source_file):
    """Keep each declaration position and the source bytes used for scanning."""
    declarations = {}
    for relative in scope["word_lists"]:
        path = (SCOPE_FILE.parent / relative).resolve()
        if path in declarations:
            continue
        try:
            source = read_source(path)
        except (OSError, UnicodeError) as error:
            raise VocabularyError(path, error) from error
        entries = {}
        offset = 0
        for number, line in enumerate(source.text.splitlines(keepends=True), 1):
            declaration = line.split("#", 1)[0]
            spelling = declaration.strip()
            if spelling:
                if VOCABULARY_ENTRY.fullmatch(spelling.lower()) is None:
                    raise VocabularyError(path, f"Invalid vocabulary entry in {relative}: {spelling.lower()}")
                start = offset + len(declaration) - len(declaration.lstrip())
                entry = Entry(source_path(path, REPO_ROOT), number, spelling)
                entries[entry] = (start, start + len(spelling))
            offset += len(line)
        declarations[path] = entries
    if config is not None:
        path = SCOPE_FILE.resolve()
        declarations[path] = {
            Entry(source_path(path, REPO_ROOT), position, spelling, owner): (start, end)
            for owner, position, spelling, start, end in file_word_positions(config.text)
        }
    return declarations


def read_proposal(path, entries):
    requested = json.loads(read_source_file(path).text, object_pairs_hook=unique_json_object)
    if not isinstance(requested, list):
        raise ValueError("Removal proposal must be an array of declaration identities")
    keyed = {json.dumps(entry.identity(), sort_keys=True): entry for entry in entries}
    selected = []
    for identity in requested:
        entry = keyed.get(json.dumps(identity, sort_keys=True))
        if entry is None:
            raise ValueError(f"Removal proposal does not match a current declaration: {identity}")
        selected.append(entry)
    if len(selected) != len(set(selected)):
        raise ValueError("Removal proposal repeats the same declaration")
    return selected


def mask_declarations(text, spans):
    """Blank declaration text while preserving comments and original source positions."""
    parts = []
    previous = 0
    for start, end in sorted(spans):
        parts.extend((text[previous:start], " " * (end - start)))
        previous = end
    return "".join((*parts, text[previous:]))


def prepare_line(line, positions=False):
    """Apply the same syntax rules to ordinary checking and both audit evaluations."""
    origin = list(range(len(line))) if positions else None

    def substitute(pattern, replacement):
        nonlocal line, origin
        if origin is None:
            line = pattern.sub(replacement, line)
            return
        parts = []
        mapped = []
        cursor = 0
        for match in pattern.finditer(line):
            value = replacement(match)
            parts.extend((line[cursor:match.start()], value))
            mapped.extend(origin[cursor:match.start()])
            if pattern is OPCODE_LITERAL_PATTERN:
                mapped.extend(origin[match.start(1):match.end(1)])
                mapped.extend(origin[match.start(2):match.end(2)])
            else:
                mapped.extend([origin[match.start()]] * len(value))
            cursor = match.end()
        parts.append(line[cursor:])
        mapped.extend(origin[cursor:])
        line = "".join(parts)
        origin = mapped

    substitute(URL_PATTERN, lambda _: " ")
    substitute(OPCODE_LITERAL_PATTERN, lambda match: match[1] + match[2])
    substitute(NUMBER_PATTERN, lambda _: " ")
    substitute(UNICODE_ESCAPE, lambda match: chr(int(match[1], 16)))
    if "\u00ad" in line:
        substitute(SOFT_HYPHEN, lambda _: "")
    line = "".join(str(unicodedata.digit(character)) if character.isdigit() else character for character in line)
    substitute(STRING_PREFIX, lambda _: "")
    return line, origin


def words_in_prepared_line(line, dictionary, vocabulary, consulted=None, omitted=None, matches=None):
    def accepted(word):
        if word in dictionary:
            return True
        if word == omitted or word not in vocabulary:
            return False
        if consulted is not None:
            consulted.setdefault(word, {})[match.start()] = match
        return True

    for match in WORD_PATTERN.finditer(line) if matches is None else matches:
        token = match.group().replace("\u2019", "'")
        if token.lower().endswith("'s"):
            token = token[:-2]
        start = match.start()
        word = token.lower()
        known = accepted(word)
        if (not known and start > 0 and line[start - 1] == "\\" and token[0] in "nrtb"
                and accepted(token[1:].lower())):
            token = token[1:]
            word = token.lower()
            start += 1
            known = True
        if known:
            yield word, start, True
        elif token.endswith("s") and token[:-1].isupper():
            word = token[:-1].lower()
            yield word, start, accepted(word)
        else:
            boundary = 0
            for split in PIECE_BOUNDARY.finditer(token):
                piece = token[boundary:split.start()]
                if piece and any(character.isalpha() for character in piece):
                    word = piece.lower()
                    yield word, start + boundary, accepted(word)
                boundary = split.end()
            piece = token[boundary:]
            if piece and any(character.isalpha() for character in piece):
                word = piece.lower()
                yield word, start + boundary, accepted(word)


def unknown_words(text, dictionary, vocabulary):
    """Report unknown words with source line numbers and per-line deduplication."""
    findings = []
    for number, line in enumerate(text.splitlines(), 1):
        prepared, _ = prepare_line(line)
        words = words_in_prepared_line(prepared, dictionary.word_frequency, vocabulary)
        for word in dict.fromkeys(word for word, _, known in words if not known):
            findings.append((number, word, line.strip()))
    return findings


def worker_count():
    """Use half the available logical CPUs, with one to six scan processes."""
    available = getattr(os, "process_cpu_count", os.cpu_count)() or 1
    return min(6, max(1, available // 2))


def vocabulary_by_owner(entries):
    words = defaultdict(set)
    for entry in entries:
        words[entry.owner].add(entry.lookup)
    return dict(words)


@dataclass
class Scan:
    root: Path
    sources: dict
    declarations: dict
    vocabulary: dict
    removals: set
    revised_vocabulary: dict | None


@dataclass
class FileResult:
    owner: str
    digest: str = ""
    error: str = ""
    phase: str = "read"
    unknowns: list | None = None
    required: set | None = None
    alternatives: dict | None = None
    added: list | None = None
    hidden: list | None = None


def initialize_worker(scan, dictionary=None):
    """Keep a dictionary per process and share the same scan inputs."""
    global WORKER_SCAN, WORKER_DICTIONARY
    WORKER_SCAN = scan
    WORKER_DICTIONARY = (SpellChecker(language="en") if dictionary is None else dictionary).word_frequency


def unknown_occurrences(words):
    return {(word, index) for word, index, known in words if not known}


def occurrence_differences(original, revised, before, after, path, number):
    if (not before and not after) or (original == revised and before == after):
        return [], []

    def mapped(line, occurrences):
        if not occurrences:
            return {}
        _, origins = prepare_line(line, positions=True)
        return {(origins[index] + 1, word): {"path": path, "line": number,
                                            "column": origins[index] + 1, "word": word,
                                            "preview": line.strip()}
                for word, index in occurrences}

    baseline, proposal = mapped(original, before), mapped(revised, after)
    return ([proposal[key] for key in sorted(proposal.keys() - baseline.keys())],
            [baseline[key] for key in sorted(baseline.keys() - proposal.keys())])


def record_usage(prepared, words, consulted, vocabulary, path, number, required, alternatives):
    for term in consulted.keys() - required:
        matches = consulted[term].values()
        before = [word for word in words if any(match.start() <= word[1] < match.end() for match in matches)]
        after = list(words_in_prepared_line(prepared, WORKER_DICTIONARY, vocabulary, omitted=term, matches=matches))
        if unknown_occurrences(after) != unknown_occurrences(before):
            required.add(term)
        elif term not in alternatives:
            original_words = {word for word, _, known in before if known}
            replacement = list(dict.fromkeys(word for word, _, known in after
                                              if known and word not in original_words))
            alternatives[term] = {"path": path, "line": number, "accepted_words": replacement}


def scan_file(path):
    try:
        return scan_source(path)
    except Exception as error:
        # A failure in one target must still allow later targets to be checked.
        return FileResult(source_path(path, WORKER_SCAN.root), error=str(error) or type(error).__name__,
                          phase="worker")


def scan_source(path):
    """Read one file once and reuse its text for reporting, usage and validation."""
    scan = WORKER_SCAN
    resolved = path.resolve()
    owner = source_path(resolved, scan.root)
    relative = source_path(path, scan.root)
    try:
        source = scan.sources.get(resolved) or read_source_file(path)
    except (OSError, UnicodeError) as error:
        return FileResult(owner, error=str(error))

    declarations = scan.declarations.get(resolved, {})
    audit_text = mask_declarations(source.text, declarations.values()) if declarations else source.text
    revised_text = source.text
    vocabulary = scan.vocabulary.get(None, set()) | scan.vocabulary.get(owner, set())
    revised_vocabulary = None
    if scan.revised_vocabulary is not None:
        revised_vocabulary = scan.revised_vocabulary.get(None, set()) | scan.revised_vocabulary.get(owner, set())
        if scan.removals & declarations.keys():
            revised_text = mask_declarations(source.text, (span for entry, span in declarations.items()
                                                           if entry in scan.removals))

    unknowns, added, hidden = [], [], []
    required, alternatives = set(), {}
    for number, (line, audit_line, revised_line) in enumerate(
            zip(source.text.splitlines(), audit_text.splitlines(), revised_text.splitlines(), strict=True), 1):
        prepared, _ = prepare_line(line)
        consulted = {} if audit_line == line else None
        words = list(words_in_prepared_line(prepared, WORKER_DICTIONARY, vocabulary, consulted))
        for word in dict.fromkeys(word for word, _, known in words if not known):
            unknowns.append({"kind": "unknown_word", "path": relative, "line": number, "word": word,
                             "preview": line.strip()})
        if consulted is None:
            audit_prepared, _ = prepare_line(audit_line)
            consulted = {}
            audit_words = list(words_in_prepared_line(audit_prepared, WORKER_DICTIONARY, vocabulary, consulted))
        else:
            audit_prepared, audit_words = prepared, words
        record_usage(audit_prepared, audit_words, consulted, vocabulary, relative, number, required, alternatives)

        if revised_vocabulary is not None and (vocabulary != revised_vocabulary or line != revised_line):
            revised_prepared = prepared if line == revised_line else prepare_line(revised_line)[0]
            revised_words = words_in_prepared_line(revised_prepared, WORKER_DICTIONARY, revised_vocabulary)
            new, lost = occurrence_differences(line, revised_line, unknown_occurrences(words),
                                              unknown_occurrences(revised_words), relative, number)
            added.extend(new)
            hidden.extend(lost)
    return FileResult(owner, source.digest, unknowns=unknowns, required=required,
                      alternatives=alternatives, added=added, hidden=hidden)


def scan_files(targets, scan, workers, dictionary):
    """Keep reports deterministic while distributing files across processes."""
    if workers == 1:
        initialize_worker(scan, dictionary)
        yield from map(scan_file, targets)
    else:
        with ProcessPoolExecutor(max_workers=workers, initializer=initialize_worker,
                                 initargs=(scan,)) as pool:
            yield from pool.map(scan_file, targets, chunksize=16)


def inspect_unread_owners(scope, visited, targets):
    """Use index and ignore data in bulk, then inspect history only for absent owners."""
    unread = sorted(set(scope.get("file_words", {})) - visited)
    excluded = explicit_exclusions(scope)
    eligible = {owner for owner in unread
                if repository_eligible(owner, scope) and not excluded(REPO_ROOT / owner)}
    if not eligible:
        return {owner: ("excluded_owner", None) for owner in unread}
    index = {row[2:]: row[0] for row in git_output("ls-files", "-t", "-z", "--",
                                                  *(f":(literal){owner}" for owner in sorted(eligible))) if row}
    ignored = set(git_output("check-ignore", "--stdin", "-z", allowed=(0, 1),
                             input_bytes="".join(f"{owner}\0" for owner in sorted(eligible)).encode("utf-8")))
    states = {}
    head = None
    for owner in unread:
        target = REPO_ROOT / owner
        if owner not in eligible:
            reason = "excluded_owner"
        elif owner in index:
            if target.is_file():
                reason = "owner_read_error" if owner in targets else "owner_not_discovered"
            else:
                reason = "sparse_owner" if index[owner] == "S" else "missing_tracked_owner"
        elif owner in ignored:
            reason = "ignored_owner"
        elif target.exists() or target.is_symlink():
            reason = "owner_not_discovered"
        else:
            reason = "owner_history_unclear"
            if head is None:
                head = git_output("rev-parse", "--verify", "--quiet", "HEAD", allowed=(0, 1))[0].strip()
            if head:
                history = git_output("log", "-1", "--format=%H", "--name-status", "--no-renames",
                                     "-z", "--", f":(literal){owner}")
                if len(history) >= 3 and history[1].strip() == "D" and history[2] == owner:
                    states[owner] = ("confirmed_owner_deletion", history[0])
                    continue
        states[owner] = (reason, None)
    return states


def candidate_entries(entries, required, alternatives, visited, owner_states, full, dictionary):
    globals_by_term = defaultdict(list)
    by_owner = defaultdict(list)
    for entry in entries:
        if entry.owner is None:
            globals_by_term[entry.lookup].append(entry)
        else:
            by_owner[(entry.lookup, entry.owner)].append(entry)

    def preferred_sources(term, owners):
        global_entries = globals_by_term[term]
        if global_entries and (not full or any(not by_owner[(term, owner)] for owner in owners)):
            return global_entries[:1]
        return [by_owner[(term, owner)][0] for owner in sorted(owners) if by_owner[(term, owner)]]

    preferred = {term: preferred_sources(term, owners) for term, owners in required.items()}
    for entry in entries:
        term = entry.lookup
        sources = preferred.get(term, [])
        if entry in sources:
            continue
        if entry.owner is None and not full:
            continue
        finding = {"kind": "unused_entry_candidate", "entry": entry.identity()}
        if entry.owner is not None and entry.owner not in visited:
            state, commit = owner_states.get(entry.owner, ("owner_not_read", None))
            if state != "confirmed_owner_deletion":
                continue
            finding.update(reason=state, commit=commit)
        elif term in dictionary:
            finding["reason"] = "base_english"
        elif sources:
            # Other exact allowances matter only for the same owner. Global
            # declarations can be covered by different exact allowances at different uses.
            covering = [source.identity() for source in sources
                        if entry.owner is None or source.owner in (None, entry.owner)]
            if covering:
                finding.update(reason="covered_by_declarations", alternatives=covering)
            else:
                finding["reason"] = "no_observed_use"
        elif (entry.owner, term) in alternatives:
            finding.update(reason="accepted_split", alternative=alternatives[(entry.owner, term)])
        else:
            finding["reason"] = "no_observed_use"
        yield finding


def scan_error(report, phase, error, path=None):
    finding = {"kind": "scan_error", "phase": phase, "diagnostic": str(error)}
    if path is not None:
        finding["path"] = source_path(path, REPO_ROOT)
    report["findings"].append(finding)


def initialize_report(args):
    report = {
        "scope": "files" if args.files else "repository",
        "requested_files": [source_path(path, REPO_ROOT) for path in args.files],
        "target_count": 0,
        "scanned_count": 0,
        "complete": False,
        "exit_code": 2,
        "findings": [],
        "deferred_owners": [],
        "inputs": {"target_paths": [], "sha256": {}, "deleted_owners": {}},
    }
    if args.removals is not None:
        report["validation"] = {"accepted": False, "entries": [], "added_unknowns": [],
                                "hidden_unknowns": [], "deferred_entries": []}
    return report


def run_scan(args, report):
    sources = {}
    digests = report["inputs"]["sha256"]

    def read_source(path):
        identity = path.resolve()
        source = read_source_file(identity)
        sources[identity] = source
        digests[source_path(identity, REPO_ROOT)] = source.digest
        return source

    try:
        config = read_source(SCOPE_FILE)
        scope = json.loads(config.text, object_pairs_hook=unique_json_object)
        validate_scope(scope, not args.files)
    except (OSError, ValueError, KeyError) as error:
        scan_error(report, "configuration", error, SCOPE_FILE)
        return
    try:
        declarations = load_vocabulary(scope, config, read_source)
    except VocabularyError as error:
        scan_error(report, "vocabulary", error, error.path)
        return
    entries = [entry for source_declarations in declarations.values() for entry in source_declarations]
    validation = report.get("validation")
    removals = None
    if validation is not None:
        try:
            removals = read_proposal(args.removals, entries)
            validation["entries"] = [entry.identity() for entry in removals]
        except (OSError, ValueError) as error:
            scan_error(report, "proposal", error, args.removals)
    try:
        targets = args.files or repository_targets(scope)
        excluded = explicit_exclusions(scope)
        targets = [path for path in targets if not excluded(path)]
    except (OSError, ValueError, RuntimeError) as error:
        scan_error(report, "discovery", error)
        return
    target_paths = sorted({source_path(path, REPO_ROOT) for path in targets})
    report["inputs"]["target_paths"] = target_paths
    report["target_count"] = len(targets)

    if SpellChecker is None:
        scan_error(report, "dictionary", "Install the scanner dependency: "
                   "python -m pip install -r tools/verification/checks/requirements.txt")
        return
    try:
        dictionary = SpellChecker(language="en")
    except Exception as error:
        scan_error(report, "dictionary", error)
        return

    selected = set(removals or ())
    revised_vocabulary = (vocabulary_by_owner(entry for entry in entries if entry not in selected)
                          if removals is not None else None)
    scan = Scan(REPO_ROOT, sources, declarations, vocabulary_by_owner(entries), selected, revised_vocabulary)
    required = defaultdict(set)
    alternatives = {}
    visited = set()
    workers = min(worker_count(), max(1, len(targets)))
    scans = iter(scan_files(targets, scan, workers, dictionary))
    for path in targets:
        try:
            result = next(scans)
        except Exception as error:
            scan_error(report, "worker", error, path)
            break
        if result.error:
            scan_error(report, result.phase, result.error, path)
            if result.phase == "read":
                print(f"[ERROR] Cannot read {path}: {result.error}", file=sys.stderr)
            continue
        report["scanned_count"] += 1
        visited.add(result.owner)
        digests[source_path(path, REPO_ROOT)] = result.digest
        report["findings"].extend(result.unknowns)
        if result.unknowns:
            print(f"[FAIL] Unknown English vocabulary in {path}:")
            for finding in result.unknowns:
                print(f"  Line {finding['line']}: [{finding['word']}] {finding['preview']}")
        for term in result.required:
            required[term].add(result.owner)
        for term, observed in result.alternatives.items():
            alternatives.setdefault((None, term), observed)
            alternatives.setdefault((result.owner, term), observed)
        if validation is not None:
            validation["added_unknowns"].extend(result.added)
            validation["hidden_unknowns"].extend(result.hidden)

    owner_states = {}
    if not args.files:
        try:
            owner_states = inspect_unread_owners(scope, visited, set(target_paths))
        except (OSError, ValueError, RuntimeError) as error:
            scan_error(report, "owner_discovery", error)
        for owner, (reason, commit) in owner_states.items():
            if commit is None:
                report["deferred_owners"].append({"owner": owner, "reason": reason})
            else:
                report["inputs"]["deleted_owners"][owner] = commit

    count_errors = sum(finding["kind"] == "scan_error" for finding in report["findings"])
    count_unknowns = sum(finding["kind"] == "unknown_word" for finding in report["findings"])
    complete = count_errors == 0
    report["complete"] = complete
    report["findings"].extend(candidate_entries(
        entries, required, alternatives, visited, owner_states, complete and not args.files,
        dictionary.word_frequency))
    rejected = False
    if validation is not None:
        for entry in removals or ():
            if entry.owner is not None and entry.owner not in visited:
                state, _ = owner_states.get(entry.owner, ("owner_not_read", None))
                if state != "confirmed_owner_deletion":
                    validation["deferred_entries"].append({"entry": entry.identity(), "reason": state})
        validation["accepted"] = (complete and not validation["added_unknowns"]
                                  and not validation["hidden_unknowns"] and not validation["deferred_entries"])
        rejected = not validation["accepted"]
    report["exit_code"] = 2 if count_errors else 1 if count_unknowns or rejected else 0
    print(f"[{'FAIL' if report['exit_code'] else 'PASS'}] English scan: {len(targets)} files; {workers} processes; "
          f"{count_unknowns} unknown word occurrences; {count_errors} scan errors.")


def publish_report(report):
    destination = REPO_ROOT / REPORT_DIRECTORY
    destination.mkdir(parents=True, exist_ok=True)
    if not destination.resolve().is_relative_to(REPO_ROOT):
        raise ValueError("English report directory must stay inside the repository")
    path = destination / f"scan-{uuid4().hex}.json"
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=destination,
                                         suffix=".tmp", delete=False) as output:
            temporary = Path(output.name)
            json.dump(report, output, ensure_ascii=False, indent=2)
            output.write("\n")
        temporary.replace(path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    print(f"[INFO] English scan report: {path.relative_to(REPO_ROOT).as_posix()} | scope={report['scope']} "
          f"targets={report['target_count']} scanned={report['scanned_count']} findings={len(report['findings'])}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path,
                        help="By default, scan the repository using check_english.json. "
                             "Always-excluded files and directories are also skipped when supplied explicitly.")
    parser.add_argument("--removals", type=Path, help="Validate a JSON array of proposed vocabulary removals.")
    args = parser.parse_args()
    if args.files and args.removals is not None:
        parser.error("removal validation requires a repository scan")
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    report = initialize_report(args)
    run_scan(args, report)
    for finding in report["findings"]:
        if finding["kind"] != "scan_error" or finding["phase"] == "read":
            continue
        phase = finding["phase"]
        label = "Cannot prepare English scan" if phase in ("configuration", "vocabulary", "discovery") else "English scan"
        path = f" ({finding['path']})" if "path" in finding else ""
        print(f"[ERROR] {label}{path}: {finding['diagnostic']}", file=sys.stderr)
    try:
        publish_report(report)
    except (OSError, ValueError) as error:
        print(f"[ERROR] Cannot publish English scan report: {error}", file=sys.stderr)
        return 2
    return report["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
