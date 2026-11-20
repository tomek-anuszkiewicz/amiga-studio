"""Run the separately selected post-publication stages for one local integration."""
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

from integration import read
from integration_language import ENGLISH_COMMAND, SCANNER_STATUS, SCOPE_FILE, check_retained_english, run_stage
from integration_sources import cleanup_record
from integration_state import RefTransaction, ancestor, clean, common, no_operation, publication_lock
from integration_state import represents, save, value, worktrees
from repository import git, snapshot


MASTER = "refs/heads/master"
GRAPH_FILE = "graphify-out/graph.json"


def closure_path(frozen):
    return cleanup_record(frozen).with_name("closure.json")


def published_target(root, path):
    supplied = read(path)
    if supplied["state"] != "published":
        raise ValueError("Post-publication stages require successful publication")
    target = Path(supplied["target_root"]).resolve()
    if (root.resolve() != target or supplied["target_ref"] != MASTER
            or Path(worktrees(root)[0]["worktree"]).resolve() != target):
        raise ValueError("Run post-publication stages in the recorded primary master checkout")
    frozen = read(cleanup_record(supplied))
    if ({key: value for key, value in supplied.items() if key != "cleanup"}
            != {key: value for key, value in frozen.items() if key != "cleanup"}):
        raise ValueError("Supplied publication does not match the surviving record")
    return frozen


def closure_record(frozen):
    path = closure_path(frozen)
    identity = {"publication": cleanup_record(frozen).parent.name, "published_commit": frozen["commit"]}
    record = read(path) if path.exists() else identity.copy()
    if any(record.get(key) != item for key, item in identity.items()):
        raise ValueError("Closure receipt identifies a different publication")
    if frozen["version"] >= 3:
        language = retained_language(frozen)
        if record.setdefault("language", language) != language:
            raise ValueError("Closure English differs from the published receipt")
    return path, record


def check_master(root, commit, *, untracked=False):
    before = master_commit(root)
    if before != commit:
        raise ValueError("Master differs from the recorded closure commit")
    clean(root, untracked=untracked)


def master_commit(root):
    if value(root, "symbolic-ref", "--quiet", "HEAD") != MASTER:
        raise ValueError("Post-publication stages require local master")
    return value(root, "rev-parse", "HEAD")


def english_findings(root, output):
    path = None
    findings = []
    for line in output.splitlines():
        heading = re.fullmatch(r"\[FAIL\] Unknown English vocabulary in (.+):", line)
        if heading:
            path = (root / heading[1]).resolve().relative_to(root).as_posix()
        finding = re.fullmatch(r"  Line (\d+): \[([^\]]+)\] (.*)", line)
        if finding and path is not None:
            findings.append({"path": path, "line": int(finding[1]), "word": finding[2], "context": finding[3]})
    return findings


def language_commit(frozen, record):
    language = record.get("language", {})
    return language.get("scan_commit", frozen["commit"])


def retained_language(frozen):
    receipt = frozen.get("english", {})
    check_retained_english(frozen, receipt)
    return {"state": "complete", "scan_commit": frozen["commit"], "final_commit": frozen["commit"],
            "exit_code": 0, "scanner_exit_code": 0, "receipt": receipt}


def require_historical(frozen):
    if frozen["version"] >= 3:
        raise ValueError("This publication already carries its passing English receipt; continue with closure-graph")


def correction_origin(root, publication, successor):
    """Bind a canonical successor to an unresolved scan at its exact base."""
    original = published_target(root, publication)
    require_historical(original)
    _, record = closure_record(original)
    language = record.get("language", {})
    if language.get("state") not in {"blocked", "findings"} or language.get("prepared_commit"):
        raise ValueError("A source correction requires unresolved closure English")
    base = language_commit(original, record)
    if (successor.get("verification") != "canonical" or successor["base"] != base
            or successor["commit"] == base or successor["target_ref"] != MASTER
            or Path(successor["target_root"]).resolve() != root.resolve()):
        raise ValueError("A source correction requires a distinct canonically verified successor of the exact scan commit")
    ancestor(root, base, successor["commit"])
    return {"publication": record["publication"], "base": base}


def next_language_commit(root, frozen, record, publication, correction, retry):
    language = record.get("language")
    current = language_commit(frozen, record)
    if language and language["state"] == "running":
        raise ValueError("An interrupted English scan needs inspection before closure can continue")
    if retry and language is None:
        raise ValueError("A recorded English scanner error is required for retry")
    if correction is not None:
        successor = correction
        successor_id = cleanup_record(successor).parent.name
        if language and language.get("correction") == successor_id and successor["commit"] == current:
            return current, retry and language["state"] == "blocked", successor_id
        origin = correction_origin(root, publication, successor)
        if successor.get("correction_for") != origin:
            raise ValueError("The successor publication was not linked to this unresolved closure")
        return successor["commit"], True, successor_id
    return current, language is None or (retry and language["state"] == "blocked"), (language or {}).get("correction")


def scan_language(root, publication, *, retry=False, correction=None):
    with publication_lock(common(root)):
        frozen = published_target(root, publication)
        require_historical(frozen)
        path, record = closure_record(frozen)
        successor = published_target(root, correction) if correction is not None else None
        commit, start, correction_id = next_language_commit(root, frozen, record, publication, successor, retry)
        previous = record.get("language")
        if not start:
            check_master(root, previous.get("final_commit", commit))
            print("[INFO] Existing closure English result: " + previous["state"])
            return previous["exit_code"]

        check_master(root, commit)
        if previous:
            record.setdefault("language_history", []).append(previous)
        if successor is not None and successor["version"] >= 3:
            record["language"] = {**retained_language(successor), "correction": correction_id}
            save(path, record)
            print("[INFO] Reusing passing English receipt from linked successor " + commit)
            return 0
        language = {"state": "running", "scan_commit": commit, "exit_code": 1}
        if correction_id is not None:
            language["correction"] = correction_id
        record["language"] = language
        save(path, record)
        try:
            before = snapshot(root)
            code, output, log = run_stage(root, path.parent, "english", ENGLISH_COMMAND)
            child = SCANNER_STATUS.search(output)
            child_code = int(child[1]) if child else None
            language.update(state="blocked", exit_code=code or 1, scanner_exit_code=child_code, snapshot=before, log=log)
            check_master(root, commit)
            if snapshot(root) != before:
                raise ValueError("English scan inputs changed; closure remains open")
            if code == child_code == 0:
                language.update(state="complete", final_commit=commit, exit_code=0)
            elif code == child_code == 1:
                findings = english_findings(root, output)
                summary = re.search(r"(\d+) unknown word occurrences; 0 read errors\.", output)
                if not summary or not findings or int(summary[1]) != len(findings):
                    raise ValueError("English findings are incomplete; inspect the retained scan log")
                language.update(state="findings", findings=findings)
        except (OSError, ValueError, RuntimeError) as error:
            language.update(state="blocked", error=str(error))
            raise
        finally:
            save(path, record)
        return language["exit_code"]


def checked_words(words):
    if (not isinstance(words, list) or not words or len(set(words)) != len(words)
            or not all(re.fullmatch(r"[^\W_]+(?:['?][^\W_]+)?", word) and word == word.lower() for word in words)):
        raise ValueError("Vocabulary additions must list distinct lowercase words")
    return words


def add_file_words(text, previous, additions):
    """Change only the selected file entries in the scanner's JSON document."""
    section = re.search(r'(?m)^  "file_words": \{', text)
    if section is None:
        raise ValueError("Cannot locate the scanner's file_words object")
    start = section.end()
    _, length = json.JSONDecoder().raw_decode(text[start - 1:])
    stop = start - 2 + length
    members = text[start:stop]
    newline = "\r\n" if "\r\n" in text else "\n"
    for relative, tokens in additions.items():
        key = json.dumps(relative)
        if relative in previous:
            match = re.search(r"(?m)^    " + re.escape(key) + r":\s*(?=\[)", members)
            if match is None:
                raise ValueError("Cannot locate existing file_words entry for " + relative)
            _, end = json.JSONDecoder().raw_decode(members[match.end():])
            at = match.end() + end - 1
            trimmed = len(members[:at].rstrip())
            text_values = ", ".join(map(json.dumps, tokens))
            separator = ", " if previous[relative] else ""
            members = members[:trimmed] + separator + text_values + members[trimmed:]
        else:
            comma = "," if members.strip() else ""
            members = f"{newline}    {key}: {json.dumps(tokens)}{comma}" + (members or newline + "  ")
    return text[:start] + members + text[stop:]


def vocabulary_changes(root, commit, language, additions):
    choices = read(additions)
    if not isinstance(choices, dict) or choices.keys() - {"word_lists", "file_words"}:
        raise ValueError("Vocabulary additions accept only word_lists and file_words")
    word_lists = choices.get("word_lists", {})
    file_words = choices.get("file_words", {})
    if not all(isinstance(collection, dict) for collection in (word_lists, file_words)):
        raise ValueError("word_lists and file_words must map paths to selected words")

    config_text = git(root, "show", f"{commit}:{SCOPE_FILE}").decode("utf-8")
    scope = json.loads(config_text)
    findings = {(finding["path"], finding["word"]) for finding in language["findings"]}
    global_words = set()
    updates = {}
    for name, words in word_lists.items():
        if name not in scope["word_lists"]:
            raise ValueError("Not a configured vocabulary list: " + name)
        for word in checked_words(words):
            if word in global_words or not any(token == word for _, token in findings):
                raise ValueError("Duplicate addition or word absent from the scan: " + word)
            global_words.add(word)
        relative = (Path(SCOPE_FILE).parent / name).as_posix()
        old = git(root, "show", f"{commit}:{relative}")
        newline = b"\r\n" if b"\r\n" in old else b"\n"
        separator = newline if old and not old.endswith(b"\n") else b""
        updates[relative] = old + separator + newline.join(word.encode("utf-8") for word in words) + newline

    exact_words = set()
    for relative, words in file_words.items():
        for word in checked_words(words):
            if (relative, word) not in findings or word in global_words:
                raise ValueError(f"Invalid exact-file addition: {relative} {word}")
            exact_words.add((relative, word))
    if any(word not in global_words and (relative, word) not in exact_words for relative, word in findings):
        raise ValueError("Unresolved English findings block the vocabulary commit")
    if file_words:
        previous = scope.get("file_words", {})
        updated = add_file_words(config_text, previous, file_words)
        expected = {**scope, "file_words": {**previous, **{
            name: previous.get(name, []) + words for name, words in file_words.items()
        }}}
        if json.loads(updated) != expected:
            raise ValueError("Vocabulary update would change scanner policy beyond exact-file additions")
        updates[SCOPE_FILE] = updated.encode("utf-8")
    return updates


def create_vocabulary_commit(root, directory, parent, updates):
    """Build the candidate from its immutable parent with an isolated index."""
    with tempfile.TemporaryDirectory(dir=directory) as temporary:
        env = {**os.environ, "GIT_INDEX_FILE": str(Path(temporary) / "index")}

        def run(*args, content=None):
            return subprocess.run(["git", *args], cwd=root, env=env, input=content,
                                  capture_output=True, check=True).stdout.decode("ascii").strip()

        run("read-tree", parent)
        for name, data in updates.items():
            mode = git(root, "ls-tree", parent, "--", name).split()[0].decode("ascii")
            blob = run("hash-object", "-w", "--stdin", content=data)
            run("update-index", "--cacheinfo", f"{mode},{blob},{name}")
        tree = run("write-tree")
        return run("commit-tree", tree, "-p", parent, "-m", "chore: record reviewed English vocabulary")


def commit_vocabulary(root, publication, additions):
    with publication_lock(common(root)):
        frozen = published_target(root, publication)
        require_historical(frozen)
        path, record = closure_record(frozen)
        language = record.get("language", {})
        published = language_commit(frozen, record)
        if language.get("state") == "complete":
            check_master(root, language["final_commit"])
            print("[INFO] Closure English stage already completed")
            return
        if language.get("state") != "findings":
            raise ValueError("A recorded English scan with vocabulary-only findings is required")

        pending = language.get("prepared_commit")
        head = master_commit(root)
        if pending is None:
            check_master(root, published)
            if snapshot(root) != language["snapshot"]:
                raise ValueError("Master or scan inputs changed; preserve external work")
            updates = vocabulary_changes(root, published, language, additions)
            pending = create_vocabulary_commit(root, path.parent, published, updates)
            language["prepared_commit"] = pending
            save(path, record)

        tree = value(root, "rev-parse", pending + "^{tree}")
        if head == published:
            transaction = RefTransaction(root, MASTER, published, None, pending)
            try:
                no_operation(root)
                if master_commit(root) != published:
                    raise ValueError("Master advanced while preparing the vocabulary commit")
                if not represents(root, tree):
                    clean(root, untracked=False)
                    if snapshot(root) != language["snapshot"]:
                        raise ValueError("Master or scan inputs changed; preserve external work")
                    git(root, "read-tree", "-m", "-u", published, pending)
                if not represents(root, tree):
                    raise ValueError("Checkout changed during the vocabulary update; preserve its current state")
                transaction.commit()
            finally:
                transaction.close()
        check_master(root, pending)
        language.update(state="complete", final_commit=pending, exit_code=0)
        save(path, record)
        print("[PASS] Recorded vocabulary-only commit " + pending)


def refresh_project_graph(root, publication):
    with publication_lock(common(root)):
        frozen = published_target(root, publication)
        path, record = closure_record(frozen)
        language = record.get("language", {})
        if language.get("state") != "complete":
            raise ValueError("Complete closure English before refreshing the project graph")
        commit = language["final_commit"]
        check_master(root, commit, untracked=True)
        graph = root / GRAPH_FILE
        if receipt := record.get("graph"):
            if receipt["commit"] != commit or sha256(graph.read_bytes()).hexdigest() != receipt["sha256"]:
                raise ValueError("Recorded project graph differs from the retained receipt")
            print("[INFO] Reusing project graph for " + commit)
            return 0

        if record.get("graph_attempt", {}).get("state") == "running":
            raise ValueError("An interrupted graph refresh needs inspection before retry")
        attempt = {"state": "running", "commit": commit}
        record["graph_attempt"] = attempt
        save(path, record)
        try:
            code, _, log = run_stage(root, path.parent, "graph",
                                     ["tools/graphify/refresh.py", "--graph", "project"])
            attempt.update(exit_code=code, log=log)
            check_master(root, commit, untracked=True)
            if code:
                return code
            record["graph"] = {"commit": commit, "path": GRAPH_FILE,
                               "sha256": sha256(graph.read_bytes()).hexdigest(), "log": log}
            attempt["state"] = "complete"
            print("[PASS] Project graph for " + commit)
            return 0
        finally:
            if attempt["state"] == "running":
                attempt["state"] = "failed"
            save(path, record)


def finished_graph(frozen, record):
    language = record.get("language", {})
    graph = record.get("graph", {})
    final = language.get("final_commit")
    if language.get("state") != "complete" or not final or graph.get("commit") != final:
        raise ValueError("Cleanup requires completed closure English and project graph evidence")
    return final


def capture_binding(root, task_id, execution_id):
    capture = Path(__file__).resolve().parents[1] / "workflow/capture"
    if str(capture) not in sys.path:
        sys.path.append(str(capture))
    from preservation import registered_execution
    return registered_execution(root, task_id, execution_id)


def require_capture(root, frozen, task_id, execution_id):
    original = frozen.get("capture", {})
    if (original.get("task_id"), original.get("execution_id")) != (task_id, execution_id):
        raise ValueError("Preservation requires the task and execution registered for this publication")
    if capture_binding(root, task_id, execution_id) != original:
        raise ValueError("Capture registration changed after publication")


def finish_preservation(root, publication, task_id, execution_id, copy, *, selected=None):
    with publication_lock(common(root)):
        frozen = published_target(root, publication)
        path, record = closure_record(frozen)
        final = finished_graph(frozen, record)
        check_master(root, final)
        require_capture(root, frozen, task_id, execution_id)
        identity = {"task_id": task_id, "execution_id": execution_id, "commit": final}
        state = record.setdefault("preservation", dict(identity))
        if any(state.get(key) != item for key, item in identity.items()):
            raise ValueError("Preservation identifies a different task, execution or closure commit")
        if frozen.get("delivery") is not None:
            workflow = Path(__file__).resolve().parents[1] / "workflow"
            if str(workflow) not in sys.path:
                sys.path.append(str(workflow))
            from delivery import check_preservation_selection, record_preservation
            check_preservation_selection(frozen, state, selected)
        copy(state, lambda: save(path, record))
        check_master(root, final)
        if frozen.get("delivery") is not None:
            record_preservation(root, frozen, state)
        state["complete"] = True
        save(path, record)


def require_closed(root, frozen):
    """Keep published inputs until language, graph, and required copies survive."""
    _, record = closure_record(frozen)
    final = finished_graph(frozen, record)
    state = record.get("preservation", {})
    if state.get("complete") is not True or state.get("commit") != final or not state.get("files"):
        raise ValueError("Cleanup requires completed preservation of the selected task files")
    require_capture(root, frozen, state["task_id"], state["execution_id"])
    archive = root / ".agent/task-archives" / state["task_id"] / state["execution_id"]
    if archive.resolve() != archive or not archive.is_relative_to(root / ".agent/task-archives"):
        raise ValueError("Cleanup requires the original private archive location")
    for item in state["files"].values():
        destination = archive / item["destination"]
        if destination.resolve() != destination or not destination.is_relative_to(archive) or not destination.is_file():
            raise ValueError(f"Required preserved file is unavailable: {destination}")
    ancestor(root, frozen["commit"], final)
    check_master(root, final)
    return final
