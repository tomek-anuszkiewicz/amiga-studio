"""Create one-time contextual descriptions for Reference image assets.

This standalone bootstrap utility deliberately has no cache and never overwrites
an existing ``<image>.txt`` sidecar. Bootstrap orchestration may invoke it in a
future change; it is intentionally not registered as an agent rule or skill.
"""

import argparse
import re
from pathlib import Path
from typing import Iterable, Optional


IMAGE_EXTENSIONS = {".svg", ".png", ".jpg", ".jpeg", ".webp"}
IGNORED_DIRECTORIES = {".git", ".obsidian", ".venv", "node_modules", "__pycache__"}
DEFAULT_REFERENCE_ROOT = Path(__file__).resolve().parents[2] / "Obsidian" / "Amiga" / "Reference"


def reference_images(reference_root: Path) -> Iterable[Path]:
    """Yield image files below the Reference root, excluding technical folders."""
    for image_path in sorted(reference_root.rglob("*")):
        if image_path.is_file() and image_path.suffix.lower() in IMAGE_EXTENSIONS:
            if not any(part.lower() in IGNORED_DIRECTORIES for part in image_path.parts):
                yield image_path


def sidecar_path(image_path: Path) -> Path:
    """Return the conventional sidecar path for an image."""
    return image_path.with_name(f"{image_path.name}.txt")


def clean_markdown_text(text: str) -> str:
    """Compact nearby Markdown into a short factual context extract."""
    text = re.sub(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]", r"\1", text)
    text = re.sub(r"[#`|]", " ", text)
    return re.sub(r"\s+", " ", text).strip()


def image_context(image_path: Path, reference_root: Path) -> tuple[str, str]:
    """Find the first Markdown embed and return its caption plus nearby context."""
    image_name = re.escape(image_path.name)
    pattern = re.compile(rf"!\[([^\]]*)\]\([^)]*{image_name}[^)]*\)|!\[\[[^\]]*{image_name}[^\]]*\]\]")
    for markdown_path in sorted(reference_root.rglob("*.md")):
        content = markdown_path.read_text(encoding="utf-8", errors="replace")
        match = pattern.search(content)
        if match:
            caption = (match.group(1) or "").strip() or image_path.stem.replace("_", " ")
            start = max(0, match.start() - 600)
            end = min(len(content), match.end() + 1000)
            return caption, clean_markdown_text(content[start:end])
    return image_path.stem.replace("_", " "), "No surrounding Markdown embed was found."


def description_for(image_path: Path, reference_root: Path) -> str:
    """Build a deterministic, reviewable initial sidecar description."""
    caption, context = image_context(image_path, reference_root)
    try:
        source_document = image_path.relative_to(reference_root).parts[0]
    except ValueError:
        source_document = "Reference"
    return (
        f"[Diagram: {caption}]\n"
        f"- Reference: {source_document}\n"
        f"- Asset: {image_path.name}\n"
        f"- Nearby Markdown Context: {context}\n"
        "- Review Note: Verify this initial contextual description against the image before relying on technical details.\n"
    )


def create_missing_descriptions(reference_root: Path, dry_run: bool = False) -> int:
    """Create descriptions only for images that lack a sidecar; return their count."""
    if not reference_root.is_dir():
        raise FileNotFoundError(f"Reference root does not exist: {reference_root}")

    created_count = 0
    for image_path in reference_images(reference_root):
        target = sidecar_path(image_path)
        if target.exists():
            continue
        if dry_run:
            print(f"Would create: {target.relative_to(reference_root)}")
        else:
            target.write_text(description_for(image_path, reference_root), encoding="utf-8")
            print(f"Created: {target.relative_to(reference_root)}")
        created_count += 1
    return created_count


def main() -> int:
    parser = argparse.ArgumentParser(description="Create missing one-time descriptions for Reference image assets.")
    parser.add_argument("--reference-root", type=Path, default=DEFAULT_REFERENCE_ROOT)
    parser.add_argument("--dry-run", action="store_true", help="Report missing descriptions without writing them.")
    args = parser.parse_args()

    count = create_missing_descriptions(args.reference_root, dry_run=args.dry_run)
    action = "would be created" if args.dry_run else "created"
    print(f"{count} Reference image description(s) {action}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
