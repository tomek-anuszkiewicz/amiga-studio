#!/usr/bin/env python3
"""Import an existing preparation dataset, render/export it, or review locally."""

import argparse
from pathlib import Path

from review.model import export_markdown, import_book, load_input, write_json
from review.render import Renderer
from review.service import read_snapshot, render_accepted, replace_accepted, serve


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("import", "render", "export", "serve"))
    parser.add_argument("--book", type=Path, required=True)
    parser.add_argument("--width", type=int, default=1200, help="Preparation viewport width; HTML view wraps to its panel")
    parser.add_argument("--channel", default="msedge", help="Playwright channel; empty selects bundled Chromium")
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    root = args.book.resolve()
    if args.command == "import":
        book = import_book(root)
        export_markdown(book, root / "working")
        print(f"Imported {sum(len(c['elements']) for c in book['chapters'])} elements")
    elif args.command == "render":
        book, assets, fingerprint = load_input(root / "working")
        renderer = Renderer(args.width, args.channel)
        try:
            snapshot = renderer.render(root / "working", book, assets, fingerprint, read_snapshot(root / "working"))
            write_json(root / "working" / "rendered.json", snapshot)
            export_markdown(book, root / "working")
            if not (root / "accepted").exists():
                replace_accepted(root, snapshot)
            else:
                render_accepted(root, renderer)
            print(snapshot["metrics"])
        finally:
            renderer.close()
    elif args.command == "export":
        snapshot = read_snapshot(root / "working")
        if not snapshot or load_input(root / "working")[2] != snapshot["input_fingerprint"]:
            raise ValueError("Render the newest authored data before exporting")
        export_markdown(snapshot, root / "working")
    else:
        serve(root, args.port, args.width, args.channel)


if __name__ == "__main__":
    main()
