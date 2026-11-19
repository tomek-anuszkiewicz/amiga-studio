"""Configure read-only source overlays on the writable, host-backed library."""

import argparse
import json
import os
from pathlib import Path


PROTECTED_INPUTS = ("source.pdf", "source.json", "source-ocr.pdf", "source-ocr.json",
                    "pdf-to-markdown", "pdf")


def selected_path(path, *, directory):
    path = Path(path).absolute()
    for part in (path, *path.parents):
        if part.is_symlink() or getattr(part, "is_junction", lambda: False)():
            raise ValueError(f"Mount path cannot use a link: {part}")
    if not (path.is_dir() if directory else path.is_file()):
        raise ValueError(f"Mount source is missing or has the wrong type: {path}")
    return path.resolve()


def bind(source, target, *, read_only=True):
    return {"type": "bind", "source": source.as_posix().replace("$", "$$"), "target": target.replace("$", "$$"),
            "read_only": read_only, "bind": {"create_host_path": False}}


def overlay(library, diagnostics, config, auth=None):
    for protected in (config, auth):
        if protected and any(protected.is_relative_to(root) for root in (library, diagnostics)):
            raise ValueError("Configuration and authentication must be outside writable mount roots")
    if library.is_relative_to(diagnostics) or diagnostics.is_relative_to(library):
        raise ValueError("Library and diagnostics roots must be separate")

    mounts = []
    variants = []
    for recipe in sorted(library.glob("*/*/working/book.json")):
        selected_path(recipe, directory=False)
        variant = recipe.parent.parent
        name = variant.relative_to(library).as_posix()
        variants.append(name)
        # A writable variant directory lets Working, Accepted and book.json
        # retain their current atomic replacement operations.
        mounts.append(bind(variant, f"/library/{name}", read_only=False))
        for member in PROTECTED_INPUTS:
            original = variant / member
            if original.exists() or original.is_symlink():
                selected_path(original, directory=member in {"pdf", "pdf-to-markdown"})
                mounts.append(bind(original, f"/library/{name}/{member}"))

    environment = {"BOOK_REVIEW_PROTECTED_VARIANTS": json.dumps(variants).replace("$", "$$")}
    if auth:
        mounts.append(bind(auth, "/run/secrets/book-review-auth"))
        environment["BOOK_REVIEW_AUTH"] = "/run/secrets/book-review-auth"
    return {"services": {"book-review": {"volumes": mounts, "environment": environment}}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--library", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        library = selected_path(args.library, directory=True)
        if library != selected_path(os.environ["BOOK_REVIEW_LIBRARY"], directory=True):
            raise ValueError("The overlay and Compose must use the same library root")
        diagnostics = selected_path(os.environ["BOOK_REVIEW_DIAGNOSTICS"], directory=True)
        config = selected_path(os.environ["BOOK_REVIEW_CONFIG"], directory=False)
        auth_file = os.environ.get("BOOK_REVIEW_AUTH")
        auth = selected_path(auth_file, directory=False) if auth_file else None
        if auth and auth.is_relative_to(Path(__file__).resolve().parents[1]):
            raise ValueError("Authentication must be outside the Docker build context")
        result = overlay(library, diagnostics, config, auth)
        args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    except (KeyError, ValueError, OSError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
