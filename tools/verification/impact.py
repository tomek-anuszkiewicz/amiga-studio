"""Conservative Rust test selection from changed inputs and Cargo dependencies."""
from dataclasses import dataclass
import json
from pathlib import Path
import subprocess


@dataclass(frozen=True)
class RustScope:
    # None means full workspace; an empty tuple means no Rust tests.
    packages: tuple[str, ...] | None
    reason: str

    @property
    def ordinary(self):
        return None if self.packages is None else tuple(
            name for name in self.packages if name != "test_runner")

    @property
    def harness(self):
        return self.packages is None or "test_runner" in self.packages

    def explain(self):
        scope = "full workspace" if self.packages is None else ", ".join(self.packages) or "none"
        return [f"Rust test scope: {scope}. {self.reason}",
                "Standard harness: " + ("selected." if self.harness else "skipped; test_runner is unaffected."),
                "Coverage: current source files of the selected ordinary packages."
                if self.ordinary is None or self.ordinary else
                "Coverage: skipped; any previous summary is not evidence for this run."]


def workspace_packages(root):
    """Read current workspace membership without building or changing the lockfile."""
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline", "--locked"],
        cwd=root, capture_output=True, text=True, encoding="utf-8")
    if result.returncode:
        raise ValueError("Cargo metadata failed: " + result.stderr.strip())
    data = json.loads(result.stdout)
    members = set(data["workspace_members"])
    packages = [package for package in data["packages"] if package["id"] in members]
    if not members or {package["id"] for package in packages} != members:
        raise ValueError("Cargo metadata has incomplete workspace membership")
    roots = {Path(package["manifest_path"]).resolve().parent: package["name"] for package in packages}
    if len(roots) != len(packages) or len(set(roots.values())) != len(packages):
        raise ValueError("Cargo metadata has ambiguous package ownership")
    for directory in roots:
        directory.relative_to(root.resolve())
    return packages, roots


def workspace_graph(root):
    """Include all declared local edges, even disabled features and target conditions."""
    packages, roots = workspace_packages(root)
    consumers = {name: set() for name in roots.values()}
    for package in packages:
        for dependency in package["dependencies"]:
            if dependency.get("path") is not None:
                directory = Path(dependency["path"]).resolve()
                if directory not in roots:
                    raise ValueError("Local dependency is outside the workspace graph")
                consumers[roots[directory]].add(package["name"])
    return roots, consumers


def language_data_input(path):
    target = Path(path)
    return (path == "tools/verification/checks/check_english.json"
            or target.parent.as_posix() == "tools/verification/checks/english_words"
            and target.suffix == ".txt")


def full_scope_input(path):
    if language_data_input(path):
        return False
    name = Path(path).name
    return (name in {"Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain", "rust-toolchain.toml"}
            or path.startswith((".cargo/", "Obsidian/Amiga/Design/"))
            or path.startswith("tools/verification/") and not path.endswith(".md"))


def independent_input(path, python_inputs):
    # Package ownership is checked first: README files can be included by Rust.
    if language_data_input(path):
        return True
    if path.endswith(".md") and ("/" not in path or path.startswith(("docs/", ".agents/", "tools/"))):
        return True
    if path.endswith(".rs"):
        return False
    return any(path.startswith(owned) if owned.endswith("/") else path == owned
               for owned in python_inputs)


def select_rust_scope(root, changes, python_inputs=()):
    changes = sorted(changes)
    if not changes:
        return RustScope(None, "Clean tree: no change boundary, so run the full Rust scope.")
    broad = [path for path in changes if full_scope_input(path)]
    if broad:
        return RustScope(None, "Shared build, verification or specification input changed: " + ", ".join(broad))
    try:
        roots, consumers = workspace_graph(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        return RustScope(None, f"Workspace graph unavailable; full Rust fallback: {error}")
    owners = set()
    unknown = []
    for path in changes:
        absolute = (root / path).resolve()
        matches = [directory for directory in roots if absolute.is_relative_to(directory)]
        if matches:
            owners.add(roots[max(matches, key=lambda directory: len(directory.parts))])
        elif not independent_input(path, python_inputs):
            unknown.append(path)
    if unknown:
        return RustScope(None, "Input ownership is unknown; full Rust fallback: " + ", ".join(unknown))
    affected = set(owners)
    pending = list(owners)
    while pending:
        for consumer in consumers[pending.pop()] - affected:
            affected.add(consumer)
            pending.append(consumer)
    reason = ("Changed packages: " + ", ".join(sorted(owners)) + "; includes all transitive consumers."
              if owners else "Only independent documentation or known Python tool inputs changed.")
    return RustScope(tuple(sorted(affected)), reason)
