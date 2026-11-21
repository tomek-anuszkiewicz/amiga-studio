"""Rebuild the closure snapshots with Git and the real freeze and seal commands."""

from hashlib import sha256
from itertools import count
import json
import os
from pathlib import Path
import sys
from unittest.mock import patch
from uuid import UUID
from zipfile import ZipFile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from build_repositories import (CONFIG, ENVIRONMENT, make_portable_evidence, repository_files,
                                temporary_root, write_entry)
from test_integration_closure import (CLOSURE_ARCHIVE, EXACT_LITERAL, INVALID_TEXT, MISSPELLED_NOTES,
                                      REVIEWED_TERM, SCOPE_FILE, WORD_LIST, PublishedClosureTests, read, save)
import evidence
import verify


def save_state(archive, case, state, sealed, *, successor=False, linked=()):
    parent = case.root.parent

    def relative(path):
        return Path(path).relative_to(parent).as_posix()

    def encode(value):
        return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()

    # Each checkout gets today's tools on restore. Keep only its tracked scan
    # configuration and software vocabulary from the tools directory.
    # Successors add objects and new worktrees. Existing checkouts keep the state
    # left by live Git, scans and publication attempts.
    additions = ["primary/.git/objects/"]
    for checkout in linked:
        additions.extend((f"primary/.git/worktrees/{checkout.name}/", f"primary/.git/refs/heads/{checkout.name}"))
    files = {}
    for path, content in repository_files(parent, case.root, (case.source, *linked)):
        if successor and path.startswith(("primary/", "task/")) and not path.startswith(tuple(additions)):
            continue
        local = path.split("/", 1)[1]
        if not local.startswith((".agent/", "tools/")) or local in (SCOPE_FILE, WORD_LIST):
            files[path] = content
    if not successor:
        files["primary/.git/config"] = CONFIG.encode()

    frozen = read(sealed)
    for key in ("source_root", "target_root"):
        frozen[key] = relative(frozen[key])
    for source in frozen["sources"]:
        source["root"] = relative(source["root"])
    if frozen["verification"] == "confirmed-fast-forward":
        confirmation = Path(frozen["confirmation_path"])
        frozen["confirmation_path"] = relative(confirmation)
        files[relative(confirmation)] = confirmation.read_bytes()
    else:
        directory = Path(frozen["evidence_directory"])
        frozen["evidence_directory"] = relative(directory)
        frozen["evidence"] = make_portable_evidence(case.source, directory)
        for group in frozen["evidence"]["groups"].values():
            for path in group["artifacts"]:
                artifact = directory / path
                files[relative(artifact)] = artifact.read_bytes()
        if frozen["sources_explicit"]:
            frozen["review"]["sources"] = evidence.digest(frozen["sources"])
        review_artifacts = {}
        for original, identity in frozen["review_artifacts"].items():
            artifact = Path(original)
            content = artifact.read_bytes()
            if frozen["sources_explicit"] and artifact.name in {"review.json", "sources.json"}:
                data = frozen["review"] if artifact.name == "review.json" else frozen["sources"]
                content = encode(data)
                identity = sha256(content).hexdigest()
            review_artifacts[relative(artifact)] = identity
            files[relative(artifact)] = content
        frozen["review_artifacts"] = review_artifacts

    files[f"task/.agent/tmp/sealed-{state}.json"] = encode(frozen)
    for path, content in sorted(files.items()):
        write_entry(archive, f"{state}/{path}", content)


def typo_states(archive, case):
    case.write(case.root, "notes.md", MISSPELLED_NOTES)
    case.base = case.commit(case.root, "Establish an unchanged authored file")
    case.git(case.source, "merge", "--ff-only", case.base)
    case.seal()
    save_state(archive, case, "typo", case.sealed)
    case.write(case.source, "notes.md", "The task is supposed to succeed.\n")
    sealed = case.seal_canonical(case.source, case.base, "Fix the reported English wording")
    save_state(archive, case, "typo-corrected", sealed, successor=True)


def vocabulary_states(archive, case):
    case.seal({"notes.md": f"The accepted terminology includes {REVIEWED_TERM}.\n",
               "literal.py": f"rejected = '{EXACT_LITERAL}'\n"})
    save_state(archive, case, "vocabulary", case.sealed)
    word_list = case.source / WORD_LIST
    with word_list.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(REVIEWED_TERM + "\n")
    scope_path = case.source / SCOPE_FILE
    scope = read(scope_path)
    scope["file_words"]["literal.py"] = [EXACT_LITERAL]
    save(scope_path, scope)
    sealed = case.seal_canonical(case.source, case.base, "Record reviewed terminology and exact literal")
    save_state(archive, case, "vocabulary-corrected", sealed, successor=True)


def clean_state(archive, case):
    case.seal()
    save_state(archive, case, "clean", case.sealed)


def combined_states(archive, case):
    case.seal()
    divergent = case.published
    case.git(case.source, "branch", "divergent-task", divergent)
    case.write(case.root, "notes.md", "Maintain ordinary task notes.\n")
    first_base = case.commit(case.root, "Advance master independently")
    case.write(case.root, "notes.md", "Keep the latest task notes.\n")
    base = case.commit(case.root, "Advance master between scan and publication")
    case.git(case.root, "branch", "next-master", base)
    case.git(case.root, "reset", "--hard", first_base)
    case.git(case.source, "merge", "--no-ff", first_base)
    sealed = case.seal_canonical(case.source, first_base, "Seal the divergent task")
    save_state(archive, case, "divergent", sealed)

    case.git(case.root, "merge", "--ff-only", base)
    case.git(case.source, "merge", "--no-ff", base)
    other = case.root.parent / "other"
    case.git(case.root, "worktree", "add", "-b", "other", str(other), base)
    case.write(other, "invalid.md", INVALID_TEXT)
    second = case.commit(other, "Add a file with a scanner read error")
    case.git(case.source, "merge", "--no-ff", second)
    sources = [{"ref": "refs/heads/task", "root": str(case.source), "tip": divergent},
               {"ref": "refs/heads/other", "root": str(other), "tip": second}]
    sealed = case.seal_canonical(case.source, base, "Seal the selected sources", sources)
    save_state(archive, case, "combined", sealed, successor=True, linked=(other,))

    case.write(case.source, "invalid.md", "The source text is available.\n")
    sealed = case.seal_canonical(case.source, base, "Repair the file before publication", sources)
    save_state(archive, case, "combined-corrected", sealed, successor=True)


def build(output):
    serial = count(1)
    # Native case reports belong to the generation run, outside the snapshots.
    with (patch.dict(os.environ, ENVIRONMENT),
          patch.object(evidence.uuid, "uuid4", side_effect=lambda: UUID(int=next(serial))),
          patch.object(verify, "run_reported", side_effect=lambda _, __, run, **kwargs: run(None)),
          ZipFile(output, "w") as archive):
        for prepare in (typo_states, vocabulary_states, clean_state, combined_states):
            with temporary_root() as parent:
                case = PublishedClosureTests()
                case.configure(parent)
                case.initialize_repository()
                prepare(archive, case)


if __name__ == "__main__":
    build(Path(sys.argv[1]) if len(sys.argv) > 1 else CLOSURE_ARCHIVE)
