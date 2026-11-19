#!/usr/bin/env python3
"""Import an existing preparation dataset, render/export it, or review locally."""

import argparse
import json
from pathlib import Path

from review.model import export_markdown, import_book, load_input, write_json
from review.render import Renderer
from review.service import read_snapshot, render_accepted, replace_accepted
from review.library import BOOKS, serve
from review.ownership import BookInUseError, BookWriter
from review.source_validation import validate_book
from review.reconstruct import import_accepted_book
from review.publish import publish_book


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("import", "import-book", "render", "export", "publish", "serve", "validate"))
    parser.add_argument("--book", type=Path, help="Variant directory; required except for serve")
    parser.add_argument("--width", type=int, default=1200, help="Preparation viewport width; HTML view wraps to its panel")
    parser.add_argument("--channel", default="msedge", help="Playwright channel; empty selects bundled Chromium")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--host", choices=("127.0.0.1", "0.0.0.0"), default="127.0.0.1",
                        help="Server bind address")
    parser.add_argument("--library", type=Path, default=BOOKS, help="Imported library for serve")
    parser.add_argument("--protected-variants", type=json.loads, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.command == "serve":
        if args.book is not None:
            parser.error("serve opens the books catalog; select a book in the browser")
        serve(args.port, args.width, args.channel, host=args.host, root=args.library,
              allowed=args.protected_variants)
        return
    if args.book is None:
        parser.error("--book is required for this command")
    root = args.book.resolve()
    if args.command in {"import", "import-book", "render", "export"}:
        try:
            with BookWriter(root):
                run_book_command(args, root)
        except BookInUseError as error:
            parser.exit(1, f"{error}\n")
    else:
        run_book_command(args, root)


def run_book_command(args, root):
    if args.command == "validate":
        book = json.loads((root / 'working' / 'book.json').read_text(encoding='utf-8'))
        report = validate_book(book, root / 'pdf-to-markdown')
        print(json.dumps(report, ensure_ascii=True, indent=2))
        raise SystemExit(0 if report['valid'] else 1)
    elif args.command == "import":
        book = import_book(root)
        export_markdown(book, root / "working")
        print(f"Imported {sum(len(c['elements']) for c in book['chapters'])} elements")
    elif args.command == "import-book":
        count = import_accepted_book(root)
        print(f"Reconstructed {count} accepted elements from book.json")
    elif args.command == "render":
        book, assets, fingerprint = load_input(root / "working")
        previous = read_snapshot(root / "working")
        renderer = Renderer(args.width, args.channel)
        try:
            snapshot = renderer.render(root / "working", book, assets, fingerprint, previous)
            write_json(root / "working" / "rendered.json", snapshot)
            export_markdown(book, root / "working", previous)
            if not (root / "accepted").exists():
                replace_accepted(root, snapshot)
            else:
                render_accepted(root, renderer)
            print(snapshot["metrics"])
        finally:
            renderer.close()
    elif args.command == "publish":
        reference_root = Path(__file__).resolve().parents[3] / "Obsidian" / "Amiga" / "Reference"
        destination, assets, descriptions = publish_book(root, reference_root)
        print(f"Published {destination} with {assets} images and {descriptions} descriptions")
    elif args.command == "export":
        snapshot = read_snapshot(root / "working")
        if not snapshot or load_input(root / "working")[2] != snapshot["input_fingerprint"]:
            raise ValueError("Render the newest authored data before exporting")
        export_markdown(snapshot, root / "working")


if __name__ == "__main__":
    main()
