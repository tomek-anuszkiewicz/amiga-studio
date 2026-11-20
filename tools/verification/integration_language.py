"""Collect and check the full English scan receipt for a sealed integration."""
from hashlib import sha256
from importlib.resources import files
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from uuid import uuid4

from evidence import digest, file_digest
from integration_sources import cleanup_record
from integration_state import clean, common, publication_lock, save, value
from repository import git


SCOPE_FILE = "tools/verification/checks/check_english.json"
ENGLISH_COMMAND = ["tools/verification/verify.py", "check", "english", "--log-level", "full"]
SCANNER_STATUS = re.compile(r"^\[(?:PASS|FAIL)\] Strict English Policy .*: command exited with code (\d+)$",
                            re.MULTILINE)
PASS_SUMMARY = re.compile(r"^\[PASS\] English scan: (\d+) files; \d+ processes; "
                          r"0 unknown word occurrences; 0 read errors\.$", re.MULTILINE)


def run_stage(root, directory, name, arguments):
    command = [sys.executable, *arguments]
    print("[CMD] " + subprocess.list2cmdline(command), flush=True)
    result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            env={**os.environ, "PYTHONIOENCODING": "utf-8"})
    log = directory / f"{name}-{uuid4().hex}.log"
    log.write_bytes(result.stdout)
    output = result.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    print(output, end="", flush=True)
    print("[LOG] " + str(log), flush=True)
    return result.returncode, output, log.name


def read_record(path):
    content = path.read_bytes()
    return json.loads(content), sha256(content).hexdigest()


def scanner_inputs():
    """Use the scanner's own target selection with its actual working files."""
    from checks.check_english import REPO_ROOT, repository_targets

    scope_path = REPO_ROOT / SCOPE_FILE
    scope_bytes = scope_path.read_bytes()
    scope = json.loads(scope_bytes.decode("utf-8"))
    excluded = [(REPO_ROOT / name).resolve() for name in scope["always_excluded_directories"]]
    targets = {path.relative_to(REPO_ROOT).as_posix(): file_digest(path)
               for path in repository_targets(scope)
               if not any(path.resolve().is_relative_to(directory) for directory in excluded)}
    vocabulary = {name: file_digest(scope_path.parent / name) for name in scope["word_lists"]}
    dictionary = files("spellchecker")
    dictionary_inputs = {item.name: file_digest(item) for item in dictionary.iterdir()
                         if item.name.endswith(".py")}
    dictionary_inputs["resources/en.json.gz"] = file_digest(dictionary / "resources" / "en.json.gz")
    tool_paths = (scope_path.with_suffix(".py"), Path(__file__),
                  *(REPO_ROOT / "tools/verification" / name for name in ("verify.py", "commands.py", "cli.py")))
    return {"scope": sha256(scope_bytes).hexdigest(), "vocabulary": vocabulary, "dictionary": dictionary_inputs,
            "tools": {path.relative_to(REPO_ROOT).as_posix(): file_digest(path) for path in tool_paths},
            "target_count": len(targets), "target_digest": digest(targets)}


def scan_inputs(root, frozen):
    clean(root)
    if value(root, "rev-parse", "HEAD", "HEAD^{tree}").splitlines() != [frozen["commit"], frozen["tree"]]:
        raise ValueError("English scan requires the exact sealed commit and tree")
    # Git's normal clean check skips these entries, which may hide missing or edited text.
    if any(entry[:1] != b"H" for entry in git(root, "ls-files", "-v", "-z").split(b"\0") if entry):
        raise ValueError("English scan requires a complete checkout without hidden index entries")
    result = subprocess.run([sys.executable, "tools/verification/integration_language.py"], cwd=root,
                            capture_output=True, encoding="utf-8", errors="replace")
    if result.returncode:
        raise ValueError("Cannot identify English scan inputs: " + (result.stderr or result.stdout).strip())
    return json.loads(result.stdout)


def scan_english(root, path):
    from integration import check_source

    with publication_lock(common(root)):
        frozen, record_digest = read_record(path)
        if frozen["state"] != "sealed" or Path(frozen["source_root"]).resolve() != root.resolve():
            raise ValueError("Run the full English scan on a sealed candidate in its recorded source checkout")
        check_source(root, frozen)
        directory = cleanup_record(frozen).parent
        receipt_path = directory / "english.json"
        attempt_path = directory / f"english-{uuid4().hex}.json"
        receipt = {"state": "running", "sealed_sha256": record_digest, "commit": frozen["commit"],
                   "tree": frozen["tree"], "attempt": attempt_path.name, "command": ENGLISH_COMMAND,
                   "exit_code": None, "scanner_exit_code": None}
        save(receipt_path, receipt)
        try:
            # Run the ordinary verifier even when input capture fails so its native error survives.
            before = None
            try:
                before = scan_inputs(root, frozen)
            except (OSError, ValueError, RuntimeError) as error:
                receipt["error"] = str(error)
            code, output, log = run_stage(root, directory, "english", ENGLISH_COMMAND)
            child = SCANNER_STATUS.search(output)
            child_code = int(child[1]) if child else None
            receipt.update(exit_code=code, scanner_exit_code=child_code, log=log,
                           log_sha256=file_digest(directory / log), inputs=before)
            if before is None:
                raise ValueError(receipt["error"])
            if scan_inputs(root, frozen) != before or file_digest(path) != record_digest:
                raise ValueError("Sealed candidate or English scan inputs changed during the scan")
            summary = PASS_SUMMARY.search(output)
            if code == child_code == 0 and summary and int(summary[1]) == before["target_count"]:
                receipt["state"] = "passed"
                print("[PASS] Full English scan receipt: " + str(receipt_path))
                return 0
            print("[FAIL] English scan did not produce a passing full receipt")
            return code or 1
        except (OSError, ValueError, RuntimeError) as error:
            receipt["error"] = str(error)
            raise
        finally:
            if receipt["state"] == "running":
                receipt["state"] = "blocked"
            save(attempt_path, receipt)
            save(receipt_path, receipt)


def check_retained_english(frozen, receipt):
    if (receipt.get("state"), receipt.get("commit"), receipt.get("tree"),
            receipt.get("exit_code"), receipt.get("scanner_exit_code")) != (
            "passed", frozen["commit"], frozen["tree"], 0, 0):
        raise ValueError("A passing full English receipt for the sealed candidate is required")
    directory = cleanup_record(frozen).parent
    log_name = receipt.get("log", "")
    if not re.fullmatch(r"english-[0-9a-f]{32}\.log", log_name):
        raise ValueError("English scan log name is invalid")
    log = directory / log_name
    if not log.is_file() or file_digest(log) != receipt.get("log_sha256"):
        raise ValueError("English scan log is missing or changed")


def require_english(path, frozen, record_digest):
    from integration import read

    record = cleanup_record(frozen).with_name("english.json")
    if not record.is_file():
        raise ValueError("Run integrate scan-english on the sealed candidate before publication")
    receipt = read(record)
    check_retained_english(frozen, receipt)
    if (receipt.get("sealed_sha256") != record_digest
            or receipt.get("inputs") != scan_inputs(Path(frozen["source_root"]), frozen)
            or file_digest(path) != record_digest):
        raise ValueError("Sealed candidate or English scan inputs changed; scan the final commit again")
    return receipt


if __name__ == "__main__":
    print(json.dumps(scanner_inputs()))
