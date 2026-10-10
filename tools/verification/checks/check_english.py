#!/usr/bin/env python3
"""Check every word against the English dictionary and reviewed project vocabulary."""

import argparse
from concurrent.futures import ProcessPoolExecutor
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import unicodedata

try:
    from spellchecker import SpellChecker
except ImportError:
    print("[ERROR] Install the scanner dependency: python -m pip install -r tools/verification/checks/requirements.txt", file=sys.stderr)
    sys.exit(2)

REPO_ROOT = Path(__file__).resolve().parents[3]
SCOPE_FILE = Path(__file__).with_suffix(".json")
WORKER_SCOPE = None
WORKER_VOCABULARY = None
WORKER_DICTIONARY = None
WORD_PATTERN = re.compile(r"[^\W\d_][^\W_]*(?:['\u2019][^\W\d_]+)?", re.UNICODE)
CASE_BOUNDARY = re.compile(r"(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])")
URL_PATTERN = re.compile(r"(?:https?://|mailto:)[^\s<>\"'`)]+")
NUMBER_PATTERN = re.compile(
    r"\b0[xob][0-9a-fA-F_]+(?:[ui](?:8|16|32|64|128|size))?\b"
    r"|\$[0-9a-fA-FxXyY]+\b"
    r"|\b\d[\d_]*(?:\.\d+)?(?:[eE][+-]?\d+)?(?:[uif](?:8|16|32|64|128|size))?\b"
    r"|\b(?=[0-9a-f]*\d)[0-9a-f]{7,64}\b"
    r"|(?<!\w)#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b"
)


def repository_targets(scope):
    """Enumerate unchanged tracked files and nonignored new files, pruning data trees."""
    command = ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--"]
    command.extend(f":(glob,exclude)**/{name}/**" for name in scope["excluded_directory_names"])
    command.extend(f":(glob,exclude){directory}/**" for directory in scope["excluded_directories"])
    result = subprocess.run(command, cwd=REPO_ROOT, capture_output=True, check=True)
    targets = []
    for relative in sorted(set(result.stdout.decode("utf-8").split("\0")) - {""}):
        path = Path(relative)
        if relative in scope["excluded_files"]:
            continue
        if path.suffix.lower() not in scope["text_extensions"] and path.name not in scope["text_filenames"]:
            continue
        target = REPO_ROOT / path
        if target.is_file():
            targets.append(target)
    return targets


def load_vocabulary(scope):
    """Load explicit reviewed terms; never learn vocabulary from the scanned source."""
    words = set()
    for relative in scope["word_lists"]:
        for line in (SCOPE_FILE.parent / relative).read_text(encoding="utf-8").splitlines():
            word = line.split("#", 1)[0].strip().lower()
            if word:
                if re.fullmatch(r"[^\W_]+(?:['\u2019][^\W_]+)?", word) is None:
                    raise ValueError(f"Invalid vocabulary entry in {relative}: {word}")
                words.add(word)
    return words


def words_in_line(line, dictionary, vocabulary):
    """Separate identifier words while retaining approved register and API spellings."""
    line = URL_PATTERN.sub(" ", line)
    line = NUMBER_PATTERN.sub(" ", line)
    line = re.sub(r"\\+u([0-9a-fA-F]{4})", lambda match: chr(int(match[1], 16)), line)
    line = line.replace("\u00ad", "")
    line = "".join(str(unicodedata.digit(character)) if character.isdigit() else character for character in line)
    # String prefixes are syntax, not part of the first word inside the string.
    line = re.sub(r"\b(?:fr|rf|br|rb|[frbu])(?=['\"])", "", line, flags=re.IGNORECASE)
    for match in WORD_PATTERN.finditer(line):
        token = match.group().replace("\u2019", "'")
        if token.lower().endswith("'s"):
            token = token[:-2]
        if (match.start() > 0 and line[match.start() - 1] == "\\" and token[0] in "nrtb"
                and token.lower() not in vocabulary and token.lower() not in dictionary.word_frequency
                and (token[1:].lower() in vocabulary or token[1:].lower() in dictionary.word_frequency)):
            token = token[1:]
        if token.lower() in vocabulary or token.lower() in dictionary.word_frequency:
            yield token.lower()
            continue
        if token.endswith("s") and token[:-1].isupper():
            yield token[:-1].lower()
            continue
        for part in re.split(r"\d+", CASE_BOUNDARY.sub(" ", token)):
            for piece in part.split():
                word = piece.lower()
                if any(character.isalpha() for character in word):
                    yield word


def unknown_words(text, dictionary, vocabulary):
    """Report unknown words with real source line numbers, including uppercase words."""
    findings = []
    for number, line in enumerate(text.splitlines(), 1):
        seen = set()
        for word in words_in_line(line, dictionary, vocabulary):
            if word not in seen and word not in vocabulary and word not in dictionary.word_frequency:
                findings.append((number, word, line.strip()))
                seen.add(word)
    return findings


def vocabulary_for_file(path, scope, vocabulary):
    """Keep deliberate example literals confined to their owning file."""
    try:
        relative = path.resolve().relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return vocabulary
    return vocabulary | set(scope.get("file_words", {}).get(relative, []))


def worker_count():
    """Use half the available logical CPUs, with one to six scan processes."""
    available = getattr(os, "process_cpu_count", os.cpu_count)() or 1
    return min(6, max(1, available // 2))


def initialize_worker(scope, vocabulary):
    """Load one dictionary per process and reuse it across files."""
    global WORKER_SCOPE, WORKER_VOCABULARY, WORKER_DICTIONARY
    WORKER_SCOPE = scope
    WORKER_VOCABULARY = vocabulary
    WORKER_DICTIONARY = SpellChecker(language="en")


def scan_file(path):
    """Return findings or a read error for the parent to report in file order."""
    try:
        findings = unknown_words(path.read_text(encoding="utf-8"), WORKER_DICTIONARY,
                                 vocabulary_for_file(path, WORKER_SCOPE, WORKER_VOCABULARY))
    except (OSError, UnicodeError) as error:
        return [], str(error)
    return findings, None


def scan_files(targets, scope, vocabulary, workers):
    """Keep reports deterministic while distributing files across processes."""
    if workers == 1:
        initialize_worker(scope, vocabulary)
        yield from map(scan_file, targets)
    else:
        with ProcessPoolExecutor(max_workers=workers, initializer=initialize_worker,
                                 initargs=(scope, vocabulary)) as pool:
            yield from pool.map(scan_file, targets, chunksize=16)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path,
                        help="Explicit files; by default scan the full repository using check_english.json")
    args = parser.parse_args()
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        scope = json.loads(SCOPE_FILE.read_text(encoding="utf-8"))
        vocabulary = load_vocabulary(scope)
        targets = args.files or repository_targets(scope)
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"[ERROR] Cannot prepare English scan: {error}", file=sys.stderr)
        return 2
    total = 0
    errors = 0
    workers = min(worker_count(), max(1, len(targets)))
    for path, (findings, error) in zip(targets, scan_files(targets, scope, vocabulary, workers)):
        if error is not None:
            print(f"[ERROR] Cannot read {path}: {error}", file=sys.stderr)
            errors += 1
            continue
        if findings:
            print(f"[FAIL] Unknown English vocabulary in {path}:")
            for number, word, preview in findings:
                print(f"  Line {number}: [{word}] {preview}")
            total += len(findings)
    print(f"[{'FAIL' if total or errors else 'PASS'}] English scan: {len(targets)} files; {workers} processes; "
          f"{total} unknown word occurrences; {errors} read errors.")
    return 2 if errors else 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
