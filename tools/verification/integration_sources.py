"""Immutable selected sources shared by freeze, publication and cleanup."""
from pathlib import Path

from evidence import digest
from repository import resolve_base
from integration_state import ancestor, checkout, clean, common, value, worktrees


def validate_sources(root, sources, source_ref, target_ref, head, merge_heads=(), *, registrations=None):
    if not isinstance(sources, list) or not sources:
        raise ValueError("Expected a nonempty selected source list")
    checkout_root = root.resolve()
    common_directory = common(root).resolve()
    if registrations is None:
        registrations = worktrees(root)
    refs, roots = set(), set()
    for source in sources:
        if not isinstance(source, dict) or set(source) != {"ref", "root", "tip"}:
            raise ValueError("Each selected source requires exactly ref, root and tip")
        branch, location, tip = source["ref"], Path(source["root"]).resolve(), source["tip"]
        if (not branch.startswith("refs/heads/") or branch == target_ref
                or branch in refs or location in roots or not Path(source["root"]).is_absolute()):
            raise ValueError("Selected sources must have unique local branches and absolute checkouts")
        refs.add(branch)
        roots.add(location)
        resolve_base(root, tip)
        if (checkout(root, branch, registrations) != location
                or (location != checkout_root and common(location).resolve() != common_directory)):
            raise ValueError("Selected source checkout ownership changed: " + branch)
        if branch == source_ref:
            if location != checkout_root:
                raise ValueError("Assembly checkout differs from the selected source")
            ancestor(root, tip, head)
        else:
            clean(location)
            if value(root, "rev-parse", branch) != tip or value(location, "symbolic-ref", "HEAD") != branch:
                raise ValueError("Selected source advanced or changed branch: " + branch)
            if not any(_contains(root, tip, candidate) for candidate in (head, *merge_heads)):
                raise ValueError("Selected source is absent from the assembled history: " + branch)
    if source_ref not in refs:
        raise ValueError("Selected sources must include the assembly branch")
    return sources


def _contains(root, tip, candidate):
    try:
        ancestor(root, tip, candidate)
        return True
    except ValueError:
        return False


def source_locks(frozen):
    return [(item["ref"], item["tip"]) for item in frozen["sources"]
            if item["ref"] != frozen["source_ref"]]


def cleanup_record(frozen):
    identity = digest([frozen["base"], frozen["commit"], frozen["source_ref"], frozen["sources"]])
    return Path(frozen["target_root"]) / ".agent/tmp/integration" / identity / "integration.json"
